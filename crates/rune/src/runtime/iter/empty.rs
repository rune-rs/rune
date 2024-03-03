use crate::runtime::{VmResult, Value, Iterator, DoubleEndedIterator};

/// An empty iterator.
pub struct Empty {
    _private: (),
}

impl Iterator for Empty {
    #[inline(always)]
    fn size_hint(&self) -> VmResult<(usize, Option<usize>)> {
        VmResult::Ok((0, Some(0)))
    }

    #[inline(always)]
    fn next(&self) -> VmResult<Option<Value>> {
        VmResult::Ok(None)
    }
}

impl DoubleEndedIterator for Empty {
    #[inline(always)]
    fn next_back(&self) -> VmResult<Option<Value>> {
        VmResult::Ok(None)
    }
}
