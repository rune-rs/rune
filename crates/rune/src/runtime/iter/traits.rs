use crate::runtime::{Value, VmResult};

/// Trait implemented by a rune iterator.
pub trait Iterator {
    /// The length of the remaining iterator.
    fn size_hint(&self) -> VmResult<(usize, Option<usize>)>;

    /// Get the next value out of the iterator.
    fn next(&self) -> VmResult<Option<Value>>;

    /// Get the length of the iterator if it is an exact length iterator.
    #[inline]
    fn len(&self) -> VmResult<usize> {
        let (lower, upper) = vm_try!(self.size_hint());

        if !matches!(upper, Some(upper) if lower == upper) {
            return VmResult::panic(format!("`{:?}` is not an exact-sized iterator", self));
        }

        VmResult::Ok(lower)
    }
}

/// Traits used for double-ended iterators.
pub trait DoubleEndedIterator: Iterator {
    /// Get the next back value out of the iterator.
    fn next_back(&self) -> VmResult<Option<Value>>;
}
