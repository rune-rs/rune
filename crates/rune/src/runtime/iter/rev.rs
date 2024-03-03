use crate::runtime::{Iterator, VmResult, Protocol, Value, DoubleEndedIterator};

#[derive(Debug)]
#[repr(transparent)]
pub struct Rev {
    iter: Value,
}

impl Iterator for Rev {
    #[inline]
    fn size_hint(&self) -> VmResult<(usize, Option<usize>)> {
        self.iter.call_protocol(Protocol::SIZE_HINT, ())
    }

    #[inline]
    fn next(&self) -> VmResult<Option<Value>> {
        self.iter.call_protocol(Protocol::NEXT_BACK, ())
    }
}

impl DoubleEndedIterator for Rev {
    #[inline]
    fn next_back(&self) -> VmResult<Option<Value>> {
        self.iter.call_protocol(Protocol::NEXT, ())
    }
}
