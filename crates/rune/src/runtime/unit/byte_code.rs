use core::mem::size_of;

use serde::{Deserialize, Serialize};

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, Vec};
use crate::runtime::unit::{BadInstruction, BadJump, EncodeError, UnitEncoder, UnitStorage};
use crate::runtime::Inst;

/// Unit stored as byte code, which is a more compact representation than
/// `ArrayUnit`, but takes more time to execute since it needs to be decoded as
/// it's being executed.
#[derive(Debug, TryClone, Default, Serialize, Deserialize)]
pub struct ByteCodeUnit {
    /// The instructions contained in the source file.
    #[try_clone(with = Clone::clone)]
    bytes: rust_alloc::vec::Vec<u8>,
    /// Known jump offsets.
    offsets: Vec<usize>,
}

/// Iterator for [`ByteCodeUnit`].
pub struct ByteCodeUnitIter<'a> {
    address: &'a [u8],
    len: usize,
}

impl<'a> Iterator for ByteCodeUnitIter<'a> {
    type Item = (usize, Inst);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.address.is_empty() {
            return None;
        }

        let ip = self.len.checked_sub(self.address.len())?;
        let inst = musli_storage::decode(&mut self.address).ok()?;
        Some((ip, inst))
    }
}

impl UnitEncoder for ByteCodeUnit {
    #[inline]
    fn offset(&self) -> usize {
        self.bytes.len()
    }

    #[inline]
    fn encode(&mut self, inst: Inst) -> Result<(), EncodeError> {
        musli_storage::encode(&mut self.bytes, &inst)?;
        Ok(())
    }

    #[inline]
    fn extend_offsets(&mut self, extra: usize) -> alloc::Result<usize> {
        let base = self.offsets.len();
        self.offsets.try_extend((0..extra).map(|_| 0))?;
        Ok(base)
    }

    #[inline]
    fn mark_offset(&mut self, index: usize) {
        if let Some(o) = self.offsets.get_mut(index) {
            *o = self.bytes.len();
        }
    }

    #[inline]
    fn label_jump(&self, base: usize, _: usize, jump: usize) -> usize {
        base.wrapping_add(jump)
    }
}

impl UnitStorage for ByteCodeUnit {
    type Iter<'this> = ByteCodeUnitIter<'this>;

    #[inline]
    fn end(&self) -> usize {
        self.bytes.len()
    }

    #[inline]
    fn bytes(&self) -> usize {
        self.bytes
            .len()
            .wrapping_add(self.offsets.len().wrapping_mul(size_of::<usize>()))
    }

    #[inline]
    fn iter(&self) -> Self::Iter<'_> {
        ByteCodeUnitIter {
            address: &self.bytes[..],
            len: self.bytes.len(),
        }
    }

    fn get(&self, ip: usize) -> Result<Option<(Inst, usize)>, BadInstruction> {
        let Some(mut bytes) = self.bytes.get(ip..) else {
            return Ok(None);
        };

        let start = bytes.as_ptr();
        let inst: Inst = musli_storage::decode(&mut bytes).map_err(|_| BadInstruction { ip })?;
        let len = (bytes.as_ptr() as usize).wrapping_sub(start as usize);
        Ok(Some((inst, len)))
    }

    #[inline]
    fn translate(&self, jump: usize) -> Result<usize, BadJump> {
        let Some(&offset) = self.offsets.get(jump) else {
            return Err(BadJump { jump });
        };

        Ok(offset)
    }
}
