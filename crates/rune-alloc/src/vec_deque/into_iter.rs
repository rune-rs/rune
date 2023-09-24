use core::fmt;
use core::iter::FusedIterator;
use core::ptr;

use crate::alloc::{Allocator, Global};
use crate::clone::TryClone;
use crate::error::Error;

use super::VecDeque;

/// An owning iterator over the elements of a `VecDeque`.
///
/// This `struct` is created by the [`into_iter`] method on [`VecDeque`]
/// (provided by the [`IntoIterator`] trait). See its documentation for more.
///
/// [`into_iter`]: VecDeque::into_iter
pub struct IntoIter<T, A: Allocator = Global> {
    inner: VecDeque<T, A>,
}

impl<T, A: Allocator + Clone> TryClone for IntoIter<T, A>
where
    T: TryClone,
{
    #[inline]
    fn try_clone(&self) -> Result<Self, Error> {
        Ok(IntoIter {
            inner: self.inner.try_clone()?,
        })
    }

    #[inline]
    fn try_clone_from(&mut self, source: &Self) -> Result<(), Error> {
        self.inner.try_clone_from(&source.inner)
    }
}

impl<T, A: Allocator> IntoIter<T, A> {
    pub(super) fn new(inner: VecDeque<T, A>) -> Self {
        IntoIter { inner }
    }

    pub(super) fn into_vecdeque(self) -> VecDeque<T, A> {
        self.inner
    }
}

impl<T: fmt::Debug, A: Allocator> fmt::Debug for IntoIter<T, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IntoIter").field(&self.inner).finish()
    }
}

impl<T, A: Allocator> Iterator for IntoIter<T, A> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
        self.inner.pop_front()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.inner.len();
        (len, Some(len))
    }

    #[inline]
    fn count(self) -> usize {
        self.inner.len
    }

    #[inline]
    fn fold<B, F>(mut self, mut init: B, mut f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        struct Guard<'a, T, A: Allocator> {
            deque: &'a mut VecDeque<T, A>,
            // `consumed <= deque.len` always holds.
            consumed: usize,
        }

        impl<'a, T, A: Allocator> Drop for Guard<'a, T, A> {
            fn drop(&mut self) {
                self.deque.len -= self.consumed;
                self.deque.head = self.deque.to_physical_idx(self.consumed);
            }
        }

        let mut guard = Guard {
            deque: &mut self.inner,
            consumed: 0,
        };

        let (head, tail) = guard.deque.as_slices();

        init = head
            .iter()
            .map(|elem| {
                guard.consumed += 1;
                // SAFETY: Because we incremented `guard.consumed`, the
                // deque effectively forgot the element, so we can take
                // ownership
                unsafe { ptr::read(elem) }
            })
            .fold(init, &mut f);

        tail.iter()
            .map(|elem| {
                guard.consumed += 1;
                // SAFETY: Same as above.
                unsafe { ptr::read(elem) }
            })
            .fold(init, &mut f)
    }

    #[inline]
    fn last(mut self) -> Option<Self::Item> {
        self.inner.pop_back()
    }
}

impl<T, A: Allocator> DoubleEndedIterator for IntoIter<T, A> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        self.inner.pop_back()
    }

    #[inline]
    fn rfold<B, F>(mut self, mut init: B, mut f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        struct Guard<'a, T, A: Allocator> {
            deque: &'a mut VecDeque<T, A>,
            // `consumed <= deque.len` always holds.
            consumed: usize,
        }

        impl<'a, T, A: Allocator> Drop for Guard<'a, T, A> {
            fn drop(&mut self) {
                self.deque.len -= self.consumed;
            }
        }

        let mut guard = Guard {
            deque: &mut self.inner,
            consumed: 0,
        };

        let (head, tail) = guard.deque.as_slices();

        init = tail
            .iter()
            .map(|elem| {
                guard.consumed += 1;
                // SAFETY: See `try_fold`'s safety comment.
                unsafe { ptr::read(elem) }
            })
            .fold(init, &mut f);

        head.iter()
            .map(|elem| {
                guard.consumed += 1;
                // SAFETY: Same as above.
                unsafe { ptr::read(elem) }
            })
            .fold(init, &mut f)
    }
}

impl<T, A: Allocator> ExactSizeIterator for IntoIter<T, A> {
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<T, A: Allocator> FusedIterator for IntoIter<T, A> {}
