use core::fmt;
use core::iter;
use core::mem::size_of;
use core::slice;

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, Vec};

#[cfg(feature = "byte-code")]
use musli_storage::error::BufferError;
use serde::{Deserialize, Serialize};

use crate::runtime::Inst;

mod sealed {
    pub trait Sealed {}

    #[cfg(feature = "byte-code")]
    impl Sealed for crate::runtime::unit::ByteCodeUnit {}
    impl Sealed for crate::runtime::unit::ArrayUnit {}
}

/// Builder trait for unit storage.
pub trait UnitEncoder: self::sealed::Sealed {
    /// Current offset in storage, which also corresponds to the instruction
    /// pointer being built.
    #[doc(hidden)]
    fn offset(&self) -> usize;

    /// Encode an instruction into the current storage.
    #[doc(hidden)]
    fn encode(&mut self, inst: Inst) -> Result<(), EncodeError>;

    /// Indicate that the given number of offsets have been added.
    #[doc(hidden)]
    fn extend_offsets(&mut self, extra: usize) -> alloc::Result<usize>;

    /// Mark that the given offset index is at the current offset.
    #[doc(hidden)]
    fn mark_offset(&mut self, index: usize);

    /// Calculate label jump.
    #[doc(hidden)]
    fn label_jump(&self, base: usize, offset: usize, jump: usize) -> usize;
}

/// Instruction storage used by a [`Unit`][super::Unit].
pub trait UnitStorage: self::sealed::Sealed + fmt::Debug + Default {
    /// Iterator over instructions and their corresponding instruction offsets.
    type Iter<'this>: Iterator<Item = (usize, Inst)>
    where
        Self: 'this;

    /// Size of unit storage. This can be seen as the instruction pointer which
    /// is just beyond the last instruction.
    fn end(&self) -> usize;

    /// Get the number of bytes which is used to store unit bytecode.
    fn bytes(&self) -> usize;

    /// Iterate over all instructions.
    fn iter(&self) -> Self::Iter<'_>;

    /// Get the instruction at the given instruction pointer.
    fn get(&self, ip: usize) -> Result<Option<(Inst, usize)>, BadInstruction>;

    /// Translate the given jump offset.
    fn translate(&self, jump: usize) -> Result<usize, BadJump>;
}

/// Unit stored as array of instructions.
#[derive(Debug, TryClone, Default, Serialize, Deserialize)]
pub struct ArrayUnit {
    instructions: Vec<Inst>,
}

impl UnitEncoder for ArrayUnit {
    #[inline]
    fn offset(&self) -> usize {
        self.instructions.len()
    }

    #[inline]
    fn encode(&mut self, inst: Inst) -> Result<(), EncodeError> {
        self.instructions.try_push(inst)?;
        Ok(())
    }

    #[inline]
    fn extend_offsets(&mut self, _: usize) -> alloc::Result<usize> {
        Ok(self.instructions.len())
    }

    #[inline]
    fn mark_offset(&mut self, _: usize) {}

    #[inline]
    fn label_jump(&self, base: usize, offset: usize, _: usize) -> usize {
        base.wrapping_add(offset)
    }
}

impl UnitStorage for ArrayUnit {
    type Iter<'this> = iter::Enumerate<iter::Copied<slice::Iter<'this, Inst>>>;

    #[inline]
    fn end(&self) -> usize {
        self.instructions.len()
    }

    #[inline]
    fn bytes(&self) -> usize {
        self.instructions.len().wrapping_mul(size_of::<Inst>())
    }

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
#[doc(hidden)]
pub struct EncodeError {
    kind: EncodeErrorKind,
}

#[cfg(feature = "byte-code")]
impl From<BufferError> for EncodeError {
    #[inline]
    fn from(error: BufferError) -> Self {
        Self {
            kind: EncodeErrorKind::BufferError { error },
        }
    }
}

impl From<alloc::Error> for EncodeError {
    #[inline]
    fn from(error: alloc::Error) -> Self {
        Self {
            kind: EncodeErrorKind::AllocError { error },
        }
    }
}

impl fmt::Display for EncodeError {
    #[inline]
    fn fmt(
        &self,
        #[cfg_attr(not(feature = "byte-code"), allow(unused))] f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match &self.kind {
            #[cfg(feature = "byte-code")]
            EncodeErrorKind::BufferError { error } => error.fmt(f),
            EncodeErrorKind::AllocError { error } => error.fmt(f),
        }
    }
}

cfg_std! {
    impl std::error::Error for EncodeError {}
}

#[derive(Debug)]
enum EncodeErrorKind {
    #[cfg(feature = "byte-code")]
    BufferError {
        error: BufferError,
    },
    AllocError {
        error: alloc::Error,
    },
}

/// Error indicating that a bad instruction was located at the given instruction
/// pointer.
#[derive(Debug)]
pub struct BadInstruction {
    pub(crate) ip: usize,
}

impl fmt::Display for BadInstruction {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bad instruction at instruction {}", self.ip)
    }
}

cfg_std! {
    impl std::error::Error for BadInstruction {}
}

/// Error indicating that a bad instruction was located at the given instruction
/// pointer.
#[derive(Debug)]
pub struct BadJump {
    pub(crate) jump: usize,
}

impl fmt::Display for BadJump {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bad jump index {}", self.jump)
    }
}

cfg_std! {
    impl std::error::Error for BadJump {}
}
