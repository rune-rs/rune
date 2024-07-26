use crate::alloc;
use crate::compile::meta;

use super::{FromValue, MaybeTypeOf, Value, VmResult};

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
    pub(crate) fn size_hint(&self) -> VmResult<(usize, Option<usize>)> {
        self.iter.protocol_size_hint()
    }

    #[inline]
    pub(crate) fn next(&mut self) -> VmResult<Option<Value>> {
        self.iter.protocol_next()
    }
}

impl FromValue for Iterator {
    #[inline]
    fn from_value(value: Value) -> VmResult<Self> {
        value.into_iter()
    }
}

impl MaybeTypeOf for Iterator {
    #[inline]
    fn maybe_type_of() -> alloc::Result<meta::DocType> {
        Ok(meta::DocType::empty())
    }
}
