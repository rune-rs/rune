use core::fmt;
use core::iter::FusedIterator;
use core::mem;

use crate::slice;

/// An iterator over the elements of a `VecDeque`.
///
/// This `struct` is created by the [`iter`] method on [`super::VecDeque`]. See its
/// documentation for more.
///
/// [`iter`]: super::VecDeque::iter
pub struct RawIter<T> {
    i1: slice::RawIter<T>,
    i2: slice::RawIter<T>,
}

impl<T> RawIter<T> {
    pub(super) fn new(i1: slice::RawIter<T>, i2: slice::RawIter<T>) -> Self {
        Self { i1, i2 }
    }
}

impl<T: fmt::Debug> fmt::Debug for RawIter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Iter").finish()
    }
}

// FIXME(#26925) Remove in favor of `#[derive(Clone)]`
impl<T> Clone for RawIter<T> {
    fn clone(&self) -> Self {
        RawIter {
            i1: self.i1.clone(),
            i2: self.i2.clone(),
        }
    }
}

impl<T> Iterator for RawIter<T> {
    type Item = *const T;

    #[inline]
    fn next(&mut self) -> Option<*const T> {
        match self.i1.next() {
            Some(val) => Some(val),
            None => {
                // most of the time, the iterator will either always
                // call next(), or always call next_back(). By swapping
                // the iterators once the first one is empty, we ensure
                // that the first branch is taken as often as possible,
                // without sacrificing correctness, as i1 is empty anyways
                mem::swap(&mut self.i1, &mut self.i2);
                self.i1.next()
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }

    fn fold<Acc, F>(self, accum: Acc, mut f: F) -> Acc
    where
        F: FnMut(Acc, Self::Item) -> Acc,
    {
        let accum = self.i1.fold(accum, &mut f);
        self.i2.fold(accum, &mut f)
    }

    #[inline]
    fn last(mut self) -> Option<*const T> {
        self.next_back()
    }
}

impl<T> DoubleEndedIterator for RawIter<T> {
    #[inline]
    fn next_back(&mut self) -> Option<*const T> {
        match self.i2.next_back() {
            Some(val) => Some(val),
            None => {
                // most of the time, the iterator will either always
                // call next(), or always call next_back(). By swapping
                // the iterators once the second one is empty, we ensure
                // that the first branch is taken as often as possible,
                // without sacrificing correctness, as i2 is empty anyways
                mem::swap(&mut self.i1, &mut self.i2);
                self.i2.next_back()
            }
        }
    }

    fn rfold<Acc, F>(self, accum: Acc, mut f: F) -> Acc
    where
        F: FnMut(Acc, Self::Item) -> Acc,
    {
        let accum = self.i2.rfold(accum, &mut f);
        self.i1.rfold(accum, &mut f)
    }
}

impl<T> ExactSizeIterator for RawIter<T> {
    fn len(&self) -> usize {
        self.i1.len() + self.i2.len()
    }
}

impl<T> FusedIterator for RawIter<T> {}
