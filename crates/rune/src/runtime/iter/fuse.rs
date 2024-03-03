use crate::runtime::{Value, VmResult, Protocol, Iterator, DoubleEndedIterator};

#[derive(Debug)]
pub struct Fuse {
    iter: Option<Value>,
}

impl Fuse {
    fn new(iter: Value) -> Self {
        Self { iter: Some(iter) }
    }
}

impl Iterator for Fuse {
    #[inline]
    fn size_hint(&self) -> VmResult<(usize, Option<usize>)> {
        match &self.iter {
            Some(iter) => iter.call_protocol(Protocol::SIZE_HINT, ()),
            None => VmResult::Ok((0, Some(0))),
        }
    }

    #[inline]
    fn next(&self) -> VmResult<Option<Value>> {
        if let Some(iter) = &self.iter {
            if let Some(value) = vm_try!(iter.call_protocol(Protocol::NEXT, ())) {
                return VmResult::Ok(Some(value));
            }

            self.iter = None;
        }

        VmResult::Ok(None)
    }
}

impl DoubleEndedIterator for Fuse {
    #[inline]
    fn next_back(&self) -> VmResult<Option<Value>> {
        if let Some(iter) = &mut self.iter {
            if let Some(value) = vm_try!(iter.call_protocol(Protocol::NEXT_BACK, ())) {
                return VmResult::Ok(Some(value));
            }

            self.iter = None;
        }

        VmResult::Ok(None)
    }
}
