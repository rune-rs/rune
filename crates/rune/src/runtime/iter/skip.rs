use core::cmp;

use crate::runtime::{VmResult, Value, Protocol, Iterator, DoubleEndedIterator};

#[derive(Debug)]
pub struct Skip {
    iter: Value,
    n: usize,
}

impl Iterator for Skip {
    #[inline]
    fn size_hint(&self) -> VmResult<(usize, Option<usize>)> {
        let (lower, upper) = vm_try!(self.iter.call_protocol(Protocol::SIZE_HINT, ()));

        let lower = lower.saturating_sub(self.n);
        let upper = upper.map(|x| x.saturating_sub(self.n));

        VmResult::Ok((lower, upper))
    }

    #[inline]
    fn next(&self) -> VmResult<Option<Value>> {
        if self.n > 0 {
            let old_n = self.n;
            self.n = 0;

            for _ in 0..old_n {
                match vm_try!(self.iter.next()) {
                    Some(..) => (),
                    None => return VmResult::Ok(None),
                }
            }
        }

        self.iter.call_protocol(Protocol::NEXT, ())
    }
}

impl DoubleEndedIterator for Skip {
    #[inline]
    fn next_back(&self) -> VmResult<Option<Value>> {
        VmResult::Ok(if vm_try!(self.len()) > 0 {
            vm_try!(self.iter.call_protocol(Protocol::NEXT_BACK, ()))
        } else {
            None
        })
    }
}
