use crate::alloc::Allocator;
use crate::error::Error;

/// Helper trait for joining iterators.
pub trait TryJoin<S, T, A: Allocator>: Sized {
    /// Try to join the given value in the given allocator.
    fn try_join_in<I>(iter: I, sep: S, alloc: A) -> Result<Self, Error>
    where
        I: IntoIterator<Item = T>;
}
