use crate::runtime::{Value, VmResult, Iterator, Protocol};

#[derive(Debug)]
pub struct Chain {
    a: Option<Value>,
    b: Option<Value>,
}

impl Iterator for Chain {
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self {
                a: Some(a),
                b: Some(b),
            } => {
                let (a_lower, a_upper) = vm_try!(a.call_protocol(Protocol::SIZE_HINT, ()));
                let (b_lower, b_upper) = vm_try!(b.call_protocol(Protocol::SIZE_HINT, ()));

                let lower = a_lower.saturating_add(b_lower);

                let upper = match (a_upper, b_upper) {
                    (Some(x), Some(y)) => x.checked_add(y),
                    _ => None,
                };

                (lower, upper)
            }
            Self {
                a: Some(a),
                b: None,
            } => vm_try!(a.call_protocol(Protocol::SIZE_HINT, ())),
            Self {
                a: None,
                b: Some(b),
            } => vm_try!(b.call_protocol(Protocol::SIZE_HINT, ())),
            Self { a: None, b: None } => (0, Some(0)),
        }
    }

    #[inline]
    fn next(&self) -> VmResult<Option<Value>> {
        VmResult::Ok(match fuse!(self.a.call_protocol(Protocol::NEXT, ())) {
            None => maybe!(self.b.call_protocol(Protocol::NEXT, ())),
            item => item,
        })
    }
}

impl DoubleEndedIterator for Chain {
    #[inline]
    fn next_back(&self) -> VmResult<Option<Value>> {
        VmResult::Ok(match fuse!(self.b.call_protocol(Protocol::NEXT_BACK, ())) {
            None => maybe!(self.a.call_protocol(Protocol::NEXT_BACK, ())),
            item => item,
        })
    }
}
