use core::fmt;
use core::iter;
use core::slice;

use crate::no_std::error;
use crate::no_std::vec::Vec;

use musli_storage::error::BufferError;
use serde::{Deserialize, Serialize};

use crate::runtime::Inst;

mod sealed {
    pub trait Sealed {}

    impl Sealed for super::BytesStorage {}

    impl Sealed for super::ArrayStorage {}
}

/// Builder trait for unit storage.
pub trait UnitStorageBuilder: self::sealed::Sealed {
    /// Current offset in storage, which also corresponds to the instruction
    /// pointer being built.
    fn offset(&self) -> usize;

    /// Encode an instruction into the current storage.
    fn encode(&mut self, inst: Inst) -> Result<(), EncodeError>;

    /// Indicate that the given number of offsets have been added.
    fn extend_offsets(&mut self, extra: usize) -> usize;

    /// Mark that the given offset index is at the current offset.
    fn mark_offset(&mut self, index: usize);

    /// Calculate label jump.
    fn label_jump(&self, base: usize, offset: usize, jump: usize) -> usize;
}

/// Instruction storage used by a [`Unit`][super::Unit].
pub trait UnitStorage:
    self::sealed::Sealed + UnitStorageBuilder + fmt::Debug + Clone + Default
{
    /// Iterator over instructions and their corresponding instruction offsets.
    type Iter<'this>: Iterator<Item = (usize, Inst)>
    where
        Self: 'this;

    /// Iterate over all instructions.
    fn iter(&self) -> Self::Iter<'_>;

    /// Get the instruction at the given instruction pointer.
    fn get(&self, ip: usize) -> Result<Option<(Inst, usize)>, BadInstruction>;

    /// Translate the given jump offset.
    fn translate(&self, jump: usize) -> Result<usize, BadJump>;
}

/// Unit stored as bytes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BytesStorage {
    /// The instructions contained in the source file.
    bytes: Vec<u8>,
    /// Known jump offsets.
    offsets: Vec<usize>,
}

/// Iterator for [`BytesStorage`].
pub struct BytesStorageIter<'a> {
    address: &'a [u8],
    len: usize,
}

impl<'a> Iterator for BytesStorageIter<'a> {
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

impl UnitStorageBuilder for BytesStorage {
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
    fn extend_offsets(&mut self, extra: usize) -> usize {
        let base = self.offsets.len();
        self.offsets.extend((0..extra).map(|_| 0));
        base
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

impl UnitStorage for BytesStorage {
    type Iter<'this> = BytesStorageIter<'this>;

    #[inline]
    fn iter(&self) -> Self::Iter<'_> {
        BytesStorageIter {
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

/// Unit stored as array of instructions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArrayStorage {
    instructions: Vec<Inst>,
}

impl UnitStorageBuilder for ArrayStorage {
    #[inline]
    fn offset(&self) -> usize {
        self.instructions.len()
    }

    #[inline]
    fn encode(&mut self, inst: Inst) -> Result<(), EncodeError> {
        self.instructions.push(inst);
        Ok(())
    }

    #[inline]
    fn extend_offsets(&mut self, _: usize) -> usize {
        self.instructions.len()
    }

    #[inline]
    fn mark_offset(&mut self, _: usize) {}

    #[inline]
    fn label_jump(&self, base: usize, offset: usize, _: usize) -> usize {
        base.wrapping_add(offset)
    }
}

impl UnitStorage for ArrayStorage {
    type Iter<'this> = iter::Enumerate<iter::Copied<slice::Iter<'this, Inst>>>;

    #[inline]
    fn iter(&self) -> Self::Iter<'_> {
        self.instructions.iter().copied().enumerate()
    }

    #[inline]
    fn get(&self, ip: usize) -> Result<Option<(Inst, usize)>, BadInstruction> {
        let Some(inst) = self.instructions.get(ip) else {
            return Ok(None);
        };

        Ok(Some((*inst, 1)))
    }

    #[inline]
    fn translate(&self, jump: usize) -> Result<usize, BadJump> {
        Ok(jump)
    }
}

/// Error indicating that encoding failed.
#[derive(Debug)]
pub struct EncodeError {
    kind: EncodeErrorKind,
}

impl From<BufferError> for EncodeError {
    #[inline]
    fn from(error: BufferError) -> Self {
        Self {
            kind: EncodeErrorKind::BufferError { error },
        }
    }
}

impl fmt::Display for EncodeError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            EncodeErrorKind::BufferError { error } => error.fmt(f),
        }
    }
}

impl error::Error for EncodeError {}

#[derive(Debug)]
enum EncodeErrorKind {
    BufferError { error: BufferError },
}

/// Error indicating that a bad instruction was located at the given instruction
/// pointer.
#[derive(Debug)]
pub struct BadInstruction {
    ip: usize,
}

impl fmt::Display for BadInstruction {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bad instruction at instruction {}", self.ip)
    }
}

impl error::Error for BadInstruction {}

/// Error indicating that a bad instruction was located at the given instruction
/// pointer.
#[derive(Debug)]
pub struct BadJump {
    jump: usize,
}

impl fmt::Display for BadJump {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bad jump index {}", self.jump)
    }
}

impl error::Error for BadJump {}
