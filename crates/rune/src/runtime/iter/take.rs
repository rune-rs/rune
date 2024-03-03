use core::cmp;

use crate::runtime::{VmResult, Value, Protocol};

use super::Iterator;

#[derive(Debug)]
pub struct Take {
    iter: Value,
    n: usize,
}

impl Iterator for Take {
    #[inline]
    fn size_hint(&self) -> VmResult<(usize, Option<usize>)> {
        if self.n == 0 {
            return VmResult::Ok((0, Some(0)));
        }

        let (lower, upper) = self.iter.size_hint();

        let lower = cmp::min(lower, self.n);

        let upper = match upper {
            Some(x) if x < self.n => Some(x),
            _ => Some(self.n),
        };

        VmResult::Ok((lower, upper))
    }

    #[inline]
    fn next(&self) -> VmResult<Option<Value>> {
        if self.n == 0 {
            return VmResult::Ok(None);
        }

        self.n -= 1;
        self.iter.call_protocol(Protocol::NEXT, ())
    }

    #[inline]
    fn next_back(&self) -> VmResult<Option<Value>> {
        if self.n == 0 {
            return VmResult::Ok(None);
        }

        self.n -= 1;
        self.iter.call_protocol(Protocol::NEXT_BACK, ())
    }
}
