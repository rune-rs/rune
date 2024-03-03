use crate::runtime::{Value, Protocol, VmResult, Iterator, DoubleEndedIterator};

#[derive(Debug)]
pub struct Enumerate {
    iter: Value,
    count: usize,
}

impl Iterator for Enumerate {
    fn size_hint(&self) -> VmResult<(usize, Option<usize>)> {
        self.iter.call_protocol(Protocol::SIZE_HINT, ())
    }

    #[inline]
    fn next(&self) -> VmResult<Option<Value>> {
        let value = match vm_try!(self.iter.call_protocol::<Option<Value>>(Protocol::NEXT, ())) {
            Some(value) => value,
            None => return VmResult::Ok(None),
        };

        let index = self.count;
        self.count = self.count.saturating_add(1);
        VmResult::Ok(Some(vm_try!((index, value).to_value())))
    }
}

impl DoubleEndedIterator for Enumerate {
    #[inline]
    fn next_back(&self) -> VmResult<Option<Value>> {
        let value = match vm_try!(self.iter.call_protocol::<Option<Value>>(Protocol::NEXT_BACK, ())) {
            Some(value) => value,
            None => return VmResult::Ok(None),
        };

        let len = vm_try!(self.len());
        VmResult::Ok(Some(vm_try!((self.count + len, value).to_value())))
    }
}
