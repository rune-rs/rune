use crate::runtime::{Value, Function, VmResult, Protocol, Iterator, DoubleEndedIterator};

#[derive(Debug)]
pub struct Filter {
    iter: Value,
    filter: Function,
}

impl Iterator for Filter {
    fn size_hint(&self) -> VmResult<(usize, Option<usize>)> {
        let (_, upper) = vm_try!(self.iter.call_protocol(Protocol::SIZE_HINT, ()));
        VmResult::Ok((0, upper))
    }

    fn next(&self) -> VmResult<Option<Value>> {
        while let Some(value) = vm_try!(self.iter.call_protocol(Protocol::NEXT, ())) {
            if vm_try!(self.filter.call::<bool>((value.clone(),))) {
                return VmResult::Ok(Some(value));
            }
        }

        VmResult::Ok(None)
    }
}

impl DoubleEndedIterator for Filter {
    fn next_back(&self) -> VmResult<Option<Value>> {
        while let Some(value) = vm_try!(self.iter.call_protocol(Protocol::NEXT_BACK, ())) {
            if vm_try!(self.filter.call::<bool>((value.clone(),))) {
                return VmResult::Ok(Some(value));
            }
        }

        VmResult::Ok(None)
    }
}
