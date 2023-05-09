use crate::hash::Hash;

/// Full type information.
#[derive(Debug, Clone)]
pub struct FullTypeOf {
    #[doc(hidden)]
    pub hash: Hash,
}
