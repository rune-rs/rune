use crate::hash::Hash;

/// Full type information.
#[derive(Debug, Clone)]
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
