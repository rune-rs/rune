use core::fmt;
use core::iter::FusedIterator;
use core::marker::PhantomData;
use core::mem::{self, ManuallyDrop};
use core::slice::{self};

use crate::alloc::SizedTypeProperties;
use crate::alloc::{Allocator, Global};
use crate::ptr::{self, NonNull};
use crate::raw_vec::RawVec;

/// An iterator that moves out of a vector.
///
/// This `struct` is created by the `into_iter` method on [`Vec`](super::Vec)
/// (provided by the [`IntoIterator`] trait).
///
/// # Example
///
/// ```
/// let v = vec![0, 1, 2];
/// let iter: std::vec::IntoIter<_> = v.into_iter();
/// ```
pub struct IntoIter<T, A: Allocator = Global> {
    pub(super) buf: NonNull<T>,
    pub(super) phantom: PhantomData<T>,
    pub(super) cap: usize,
    // the drop impl reconstructs a RawVec from buf, cap and alloc
    // to avoid dropping the allocator twice we need to wrap it into ManuallyDrop
    pub(super) alloc: ManuallyDrop<A>,
    pub(super) ptr: *const T,
    pub(super) end: *const T, // If T is a ZST, this is actually ptr+len. This encoding is picked so that
                              // ptr == end is a quick test for the Iterator being empty, that works
                              // for both ZST and non-ZST.
}

impl<T: fmt::Debug, A: Allocator> fmt::Debug for IntoIter<T, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IntoIter").field(&self.as_slice()).finish()
    }
}

impl<T, A: Allocator> IntoIter<T, A> {
    /// Returns the remaining items of this iterator as a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// let vec = vec!['a', 'b', 'c'];
    /// let mut into_iter = vec.into_iter();
    /// assert_eq!(into_iter.as_slice(), &['a', 'b', 'c']);
    /// let _ = into_iter.next().unwrap();
    /// assert_eq!(into_iter.as_slice(), &['b', 'c']);
    /// ```
    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.ptr, self.len()) }
    }

    /// Returns the remaining items of this iterator as a mutable slice.
    ///
    /// # Examples
    ///
    /// ```
    /// let vec = vec!['a', 'b', 'c'];
    /// let mut into_iter = vec.into_iter();
    /// assert_eq!(into_iter.as_slice(), &['a', 'b', 'c']);
    /// into_iter.as_mut_slice()[2] = 'z';
    /// assert_eq!(into_iter.next().unwrap(), 'a');
    /// assert_eq!(into_iter.next().unwrap(), 'b');
    /// assert_eq!(into_iter.next().unwrap(), 'z');
    /// ```
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { &mut *self.as_raw_mut_slice() }
    }

    /// Returns a reference to the underlying allocator.
    #[inline]
    pub fn allocator(&self) -> &A {
        &self.alloc
    }

    fn as_raw_mut_slice(&mut self) -> *mut [T] {
        ptr::slice_from_raw_parts_mut(self.ptr as *mut T, self.len())
    }

    /// Forgets to Drop the remaining elements while still allowing the backing allocation to be freed.
    #[cfg(rune_nightly)]
    pub(crate) fn forget_remaining_elements(&mut self) {
        // For th ZST case, it is crucial that we mutate `end` here, not `ptr`.
        // `ptr` must stay aligned, while `end` may be unaligned.
        self.end = self.ptr;
    }
}

impl<T, A: Allocator> AsRef<[T]> for IntoIter<T, A> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

unsafe impl<T: Send, A: Allocator + Send> Send for IntoIter<T, A> {}
unsafe impl<T: Sync, A: Allocator + Sync> Sync for IntoIter<T, A> {}

impl<T, A: Allocator> Iterator for IntoIter<T, A> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
        if self.ptr == self.end {
            None
        } else if T::IS_ZST {
            // `ptr` has to stay where it is to remain aligned, so we reduce the length by 1 by
            // reducing the `end`.
            self.end = ptr::wrapping_byte_sub(self.end, 1);

            // Make up a value of this ZST.
            Some(unsafe { mem::zeroed() })
        } else {
            let old = self.ptr;
            self.ptr = unsafe { self.ptr.add(1) };

            Some(unsafe { ptr::read(old) })
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let exact = if T::IS_ZST {
            ptr::addr(self.end).wrapping_sub(ptr::addr(self.ptr))
        } else {
            unsafe { ptr::sub_ptr(self.end, self.ptr) }
        };
        (exact, Some(exact))
    }

    #[inline]
    fn count(self) -> usize {
        self.len()
    }
}

impl<T, A: Allocator> DoubleEndedIterator for IntoIter<T, A> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        if self.end == self.ptr {
            None
        } else if T::IS_ZST {
            // See above for why 'ptr.offset' isn't used
            self.end = ptr::wrapping_byte_sub(self.end, 1);

            // Make up a value of this ZST.
            Some(unsafe { mem::zeroed() })
        } else {
            self.end = unsafe { self.end.sub(1) };

            Some(unsafe { ptr::read(self.end) })
        }
    }
}

impl<T, A: Allocator> ExactSizeIterator for IntoIter<T, A> {}

impl<T, A: Allocator> FusedIterator for IntoIter<T, A> {}

impl<T, A> Default for IntoIter<T, A>
where
    A: Allocator + Default,
{
    /// Creates an empty `vec::IntoIter`.
    ///
    /// ```
    /// # use std::vec;
    /// let iter: vec::IntoIter<u8> = Default::default();
    /// assert_eq!(iter.len(), 0);
    /// assert_eq!(iter.as_slice(), &[]);
    /// ```
    fn default() -> Self {
        super::Vec::new_in(Default::default()).into_iter()
    }
}

#[doc(hidden)]
pub trait NonDrop {}

// T: Copy as approximation for !Drop since get_unchecked does not advance self.ptr
// and thus we can't implement drop-handling
impl<T: Copy> NonDrop for T {}

#[cfg(rune_nightly)]
unsafe impl<#[may_dangle] T, A: Allocator> Drop for IntoIter<T, A> {
    fn drop(&mut self) {
        struct DropGuard<'a, T, A: Allocator>(&'a mut IntoIter<T, A>);

        impl<T, A: Allocator> Drop for DropGuard<'_, T, A> {
            fn drop(&mut self) {
                unsafe {
                    // `IntoIter::alloc` is not used anymore after this and will be dropped by RawVec
                    let alloc = ManuallyDrop::take(&mut self.0.alloc);
                    // RawVec handles deallocation
                    let _ = RawVec::from_raw_parts_in(self.0.buf.as_ptr(), self.0.cap, alloc);
                }
            }
        }

        let guard = DropGuard(self);
        // destroy the remaining elements
        unsafe {
            ptr::drop_in_place(guard.0.as_raw_mut_slice());
        }
        // now `guard` will be dropped and do the rest
    }
}

#[cfg(not(rune_nightly))]
impl<T, A: Allocator> Drop for IntoIter<T, A> {
    fn drop(&mut self) {
        struct DropGuard<'a, T, A: Allocator>(&'a mut IntoIter<T, A>);

        impl<T, A: Allocator> Drop for DropGuard<'_, T, A> {
            fn drop(&mut self) {
                unsafe {
                    // `IntoIter::alloc` is not used anymore after this and will be dropped by RawVec
                    let alloc = ManuallyDrop::take(&mut self.0.alloc);
                    // RawVec handles deallocation
                    let _ = RawVec::from_raw_parts_in(self.0.buf.as_ptr(), self.0.cap, alloc);
                }
            }
        }

        let guard = DropGuard(self);
        // destroy the remaining elements
        unsafe {
            ptr::drop_in_place(guard.0.as_raw_mut_slice());
        }
        // now `guard` will be dropped and do the rest
    }
}
