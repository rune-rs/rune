use crate::runtime::{Value, VmResult, Protocol, Function, Iterator, DoubleEndedIterator};

/// An iterator that maps the values of `iter` with `f`.
///
/// This type is created by the [`map`] method on [`Iterator`]. See its
/// documentation for more.
#[derive(Debug)]
pub struct Map {
    iter: Value,
    map: Function,
}

impl Iterator for Map {
    fn size_hint(&self) -> VmResult<(usize, Option<usize>)> {
        self.iter.call_protocol(Protocol::SIZE_HINT, ())
    }

    fn next(&self) -> VmResult<Option<Value>> {
        if let Some(value) = vm_try!(self.iter.call_protocol(Protocol::NEXT, ())) {
            let out = vm_try!(self.map.call::<Value>((value,)));
            return VmResult::Ok(Some(out));
        }

        VmResult::Ok(None)
    }
}

impl DoubleEndedIterator for Map {
    fn next_back(&self) -> VmResult<Option<Value>> {
        if let Some(value) = vm_try!(self.iter.call_protocol(Protocol::NEXT_BACK, ())) {
            let out = vm_try!(self.map.call::<Value>((value,)));
            return VmResult::Ok(Some(out));
        }

        VmResult::Ok(None)
    }
}
