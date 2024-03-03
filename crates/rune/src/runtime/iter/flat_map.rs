use crate::runtime::{Value, VmResult, Protocol, Iterator, DoubleEndedIterator, Fuse};

#[derive(Debug)]
pub struct FlatMap {
    map: Fuse,
    frontiter: Option<Value>,
    backiter: Option<Value>,
}

impl Iterator for FlatMap {
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (flo, fhi) = match &self.frontiter {
            Some(iter) => vm_try!(iter.call_protocol(Protocol::SIZE_HINT, ())),
            None => (0, Some(0)),
        };

        let (blo, bhi) = match &self.backiter {
            Some(iter) => vm_try!(iter.call_protocol(Protocol::SIZE_HINT, ())),
            None => (0, Some(0)),
        };
        
        let lo = flo.saturating_add(blo);

        match (vm_try!(self.map.call_protocol(Protocol::SIZE_HINT, ())), fhi, bhi) {
            ((0, Some(0)), Some(a), Some(b)) => VmResult::Ok((lo, a.checked_add(b))),
            _ => VmResult::Ok((lo, None)),
        }
    }

    fn next(&self) -> VmResult<Option<Value>> {
        loop {
            if let Some(iter) = &self.frontiter {
                match vm_try!(iter.next()) {
                    None => self.frontiter = None,
                    item @ Some(_) => return VmResult::Ok(item),
                }
            }

            match vm_try!(self.map.next()) {
                None => {
                    return VmResult::Ok(match &self.backiter {
                        Some(backiter) => vm_try!(backiter.next()),
                        None => None,
                    })
                }
                Some(value) => {
                    let iterator = vm_try!(value.into_iter());
                    self.frontiter = Some(iterator.iter)
                }
            }
        }
    }
}

impl DoubleEndedIterator for FlatMap {
    fn next_back(&self) -> VmResult<Option<Value>> {
        loop {
            if let Some(ref mut iter) = self.backiter {
                match vm_try!(iter.next_back()) {
                    None => self.backiter = None,
                    item @ Some(_) => return VmResult::Ok(item),
                }
            }

            match vm_try!(self.map.next_back()) {
                None => {
                    return VmResult::Ok(match &mut self.frontiter {
                        Some(frontiter) => vm_try!(frontiter.next_back()),
                        None => None,
                    })
                }
                Some(value) => {
                    let iterator = vm_try!(value.into_iter());
                    self.backiter = Some(iterator.iter);
                }
            }
        }
    }
}
