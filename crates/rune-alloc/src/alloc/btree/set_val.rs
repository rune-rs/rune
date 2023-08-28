use crate::alloc::{Error, TryClone};

/// Zero-Sized Type (ZST) for internal `BTreeSet` values.
/// Used instead of `()` to differentiate between:
/// * `BTreeMap<T, ()>` (possible user-defined map)
/// * `BTreeMap<T, SetValZST>` (internal set representation)
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Clone, Default)]
pub(crate) struct SetValZST;

impl TryClone for SetValZST {
    fn try_clone(&self) -> Result<Self, Error> {
        Ok(Self)
    }
}
