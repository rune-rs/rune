use crate::hash::{Hash, IntoHash};
use crate::type_of::FullTypeOf;

/// Helper to register a parameterized function.
///
/// This is used to wrap the name of the function in order to associated
/// parameters with it.
#[derive(Clone)]
#[non_exhaustive]
pub struct Params<T, const N: usize> {
    #[doc(hidden)]
    pub name: T,
    #[doc(hidden)]
    pub parameters: [FullTypeOf; N],
}

impl<T, const N: usize> Params<T, N> {
    /// Construct a new parameters wrapper.
    pub const fn new(name: T, parameters: [FullTypeOf; N]) -> Self {
        Self { name, parameters }
    }
}

impl<T, const N: usize> IntoHash for Params<T, N>
where
    T: IntoHash,
{
    #[inline]
    fn into_hash(self) -> Hash {
        self.name.into_hash()
    }
}
