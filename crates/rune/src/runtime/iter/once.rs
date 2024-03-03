use crate::runtime::{Value, Iterator, VmResult, DoubleEndedIterator};

pub struct Once {
    value: Option<Value>,
}

impl Once {
    /// Construct a new `Once` iterator.
    pub(super) fn new(value: Value) -> Self {
        Self {
            value: Some(value),
        }
    }
}

impl Iterator for Once {
    #[inline(always)]
    fn size_hint(&self) -> VmResult<(usize, Option<usize>)> {
        let len = usize::from(self.value.is_some());
        VmResult::Ok((len, Some(len)))
    }

    #[inline(always)]
    fn next(&self) -> VmResult<Option<Value>> {
        VmResult::Ok(self.value.take())
    }
}

impl DoubleEndedIterator for Once {
    #[inline(always)]
    fn next_back(&self) -> VmResult<Option<Value>> {
        VmResult::Ok(self.value.take())
    }
}
