use crate::alloc;
use crate::compile::meta;

use super::{FromValue, MaybeTypeOf, RuntimeError, Value, VmError};

/// An owning iterator.
#[derive(Debug)]
pub struct Iterator {
    iter: Value,
}

impl Iterator {
    pub(crate) fn new(iter: Value) -> Self {
        Self { iter }
    }

    #[inline]
    pub(crate) fn size_hint(&self) -> Result<(usize, Option<usize>), VmError> {
        self.iter.protocol_size_hint()
    }

    #[inline]
    pub(crate) fn next(&mut self) -> Result<Option<Value>, VmError> {
        self.iter.protocol_next()
    }
}

impl FromValue for Iterator {
    #[inline]
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        value.into_iter().map_err(RuntimeError::from)
    }
}

impl MaybeTypeOf for Iterator {
    #[inline]
    fn maybe_type_of() -> alloc::Result<meta::DocType> {
        Ok(meta::DocType::empty())
    }
}
