use crate::runtime::{Value, VmResult, Protocol, Iterator, DoubleEndedIterator};

#[derive(Debug)]
pub struct Peekable {
    iter: Value,
    peeked: Option<Option<Value>>,
}

impl Peekable {
    #[inline]
    fn peek(&mut self) -> VmResult<Option<Value>> {
        if let Some(value) = &self.peeked {
            return VmResult::Ok(value.clone());
        }

        let value = vm_try!(self.iter.call_protocol(Protocol::NEXT, ()));
        self.peeked = Some(value.clone());
        VmResult::Ok(value)
    }
}

impl Iterator for Peekable {
    #[inline]
    fn size_hint(&self) -> VmResult<(usize, Option<usize>)> {
        let peek_len = match self.peeked {
            Some(None) => return VmResult::Ok((0, Some(0))),
            Some(Some(_)) => 1,
            None => 0,
        };
        let (lo, hi) = vm_try!(self.iter.call_protocol::<(usize, Option<usize>)>(Protocol::SIZE_HINT, ()));
        let lo = lo.saturating_add(peek_len);
        let hi = match hi {
            Some(x) => x.checked_add(peek_len),
            None => None,
        };
        VmResult::Ok((lo, hi))
    }

    #[inline]
    fn next(&self) -> VmResult<Option<Value>> {
        match self.peeked.take() {
            Some(v) => VmResult::Ok(v),
            None => self.iter.call_protocol(Protocol::NEXT, ()),
        }
    }
}

impl DoubleEndedIterator for Peekable {
    #[inline]
    fn next_back(&self) -> VmResult<Option<Value>> {
        match self.peeked.as_mut() {
            Some(v @ Some(_)) => {
                if let Some(value) = vm_try!(self.iter.call_protocol(Protocol::NEXT_BACK, ())) {
                    return VmResult::Ok(Some(value));
                }

                VmResult::Ok(v.take())
            },
            Some(None) => VmResult::Ok(None),
            None => self.iter.call_protocol(Protocol::NEXT_BACK, ()),
        }
    }
}
