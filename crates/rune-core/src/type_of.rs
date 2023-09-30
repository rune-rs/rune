use crate as rune;
use crate::alloc::prelude::*;
use crate::hash::Hash;

/// Full type information.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct FullTypeOf {
    #[doc(hidden)]
    pub hash: Hash,
}

impl FullTypeOf {
    #[inline]
    #[doc(hidden)]
    pub fn new(hash: Hash) -> Self {
        Self { hash }
    }
}
