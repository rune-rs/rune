//! Definitions of a bunch of iterators for `[T]`.

#![allow(unused_unsafe)]

#[macro_use]
mod macros;

use core::fmt;
use core::iter::FusedIterator;
use core::marker::{Send, Sized, Sync};
use core::slice::{from_raw_parts, from_raw_parts_mut};

use crate::alloc::SizedTypeProperties;
use crate::hint::assume;
use crate::ptr::{self, invalid, invalid_mut, NonNull};

/// Immutable slice iterator
///
/// This struct is created by the [`iter`] method on [slices].
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use rune::alloc::try_vec;
///
/// // First, we declare a type which has `iter` method to get the `Iter` struct (`&[usize]` here):
/// let vec = try_vec![1, 2, 3];
///
/// // Then, we iterate over it:
/// unsafe {
///     for element in vec.raw_iter() {
///         println!("{}", *element);
///     }
/// }
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// [`iter`]: slice::iter
/// [slices]: slice
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct RawIter<T> {
    /// The pointer to the next element to return, or the past-the-end location
    /// if the iterator is empty.
    ///
    /// This address will be used for all ZST elements, never changed.
    ptr: NonNull<T>,
    /// For non-ZSTs, the non-null pointer to the past-the-end element.
    ///
    /// For ZSTs, this is `ptr::invalid(len)`.
    end_or_len: *const T,
}

impl<T: fmt::Debug> fmt::Debug for RawIter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Iter").finish()
    }
}

unsafe impl<T: Sync> Sync for RawIter<T> {}
unsafe impl<T: Sync> Send for RawIter<T> {}

impl<T> RawIter<T> {
    #[inline]
    pub(crate) fn new(slice: &[T]) -> Self {
        let ptr = slice.as_ptr();
        // SAFETY: Similar to `IterMut::new`.
        unsafe {
            let end_or_len = if T::IS_ZST {
                invalid(slice.len())
            } else {
                ptr.add(slice.len())
            };

            Self {
                ptr: NonNull::new_unchecked(ptr as *mut T),
                end_or_len,
            }
        }
    }

    /// Views the underlying data as a subslice of the original data.
    ///
    /// This has the same lifetime as the original slice, and so the
    /// iterator can continue to be used while this exists.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// // First, we declare a type which has the `iter` method to get the `Iter`
    /// // struct (`&[usize]` here):
    /// let slice = try_vec![1, 2, 3];
    ///
    /// unsafe {
    ///     // Then, we get the iterator:
    ///     let mut iter = slice.raw_iter();
    ///
    ///     // So if we print what `as_slice` method returns here, we have "[1, 2, 3]":
    ///     println!("{:?}", iter.as_slice());
    ///
    ///     // Next, we move to the second element of the slice:
    ///     iter.next();
    ///     // Now `as_slice` returns "[2, 3]":
    ///     println!("{:?}", iter.as_slice());
    /// }
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[must_use]
    #[inline]
    pub unsafe fn as_slice<'a>(&self) -> &'a [T] {
        self.make_slice()
    }
}

iterator! {struct RawIter -> *const T, *const T, const, {/* no mut */}, as_ref, {}}

impl<T> Clone for RawIter<T> {
    #[inline]
    fn clone(&self) -> Self {
        RawIter {
            ptr: self.ptr,
            end_or_len: self.end_or_len,
        }
    }
}

/// Mutable slice iterator.
///
/// This struct is created by the [`iter_mut`] method on [slices].
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use rune::alloc::try_vec;
///
/// // First, we declare a type which has `iter_mut` method to get the `IterMut`
/// // struct (`&[usize]` here):
/// let mut slice = try_vec![1, 2, 3];
///
/// // Then, we iterate over it and increment each element value:
/// unsafe {
///     for element in slice.raw_iter_mut() {
///         *element += 1;
///     }
/// }
///
/// // We now have "[2, 3, 4]":
/// println!("{slice:?}");
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// [`iter_mut`]: slice::iter_mut
/// [slices]: slice
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct RawIterMut<T> {
    /// The pointer to the next element to return, or the past-the-end location
    /// if the iterator is empty.
    ///
    /// This address will be used for all ZST elements, never changed.
    ptr: NonNull<T>,
    /// For non-ZSTs, the non-null pointer to the past-the-end element.
    ///
    /// For ZSTs, this is `ptr::invalid_mut(len)`.
    end_or_len: *mut T,
}

impl<T: fmt::Debug> fmt::Debug for RawIterMut<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("IterMut").finish()
    }
}

unsafe impl<T: Sync> Sync for RawIterMut<T> {}
unsafe impl<T: Send> Send for RawIterMut<T> {}

impl<T> RawIterMut<T> {
    #[inline]
    pub(crate) fn new(slice: &mut [T]) -> Self {
        let ptr = slice.as_mut_ptr();
        // SAFETY: There are several things here:
        //
        // `ptr` has been obtained by `slice.as_ptr()` where `slice` is a valid
        // reference thus it is non-NUL and safe to use and pass to
        // `NonNull::new_unchecked` .
        //
        // Adding `slice.len()` to the starting pointer gives a pointer
        // at the end of `slice`. `end` will never be dereferenced, only checked
        // for direct pointer equality with `ptr` to check if the iterator is
        // done.
        //
        // In the case of a ZST, the end pointer is just the length.  It's never
        // used as a pointer at all, and thus it's fine to have no provenance.
        //
        // See the `next_unchecked!` and `is_empty!` macros as well as the
        // `post_inc_start` method for more information.
        unsafe {
            let end_or_len = if T::IS_ZST {
                invalid_mut(slice.len())
            } else {
                ptr.add(slice.len())
            };

            Self {
                ptr: NonNull::new_unchecked(ptr),
                end_or_len,
            }
        }
    }

    /// Views the underlying data as a subslice of the original data.
    ///
    /// To avoid creating `&mut [T]` references that alias, the returned slice
    /// borrows its lifetime from the iterator the method is applied on.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let mut slice = try_vec![1, 2, 3];
    ///
    /// unsafe {
    ///     // First, we get the iterator:
    ///     let mut iter = slice.raw_iter_mut();
    ///
    ///     // So if we check what the `as_slice` method returns here, we have "[1, 2, 3]":
    ///     assert_eq!(iter.as_slice(), &[1, 2, 3]);
    ///
    ///     // Next, we move to the second element of the slice:
    ///     iter.next();
    ///     // Now `as_slice` returns "[2, 3]":
    ///     assert_eq!(iter.as_slice(), &[2, 3]);
    /// }
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[must_use]
    #[inline]
    pub unsafe fn as_slice<'a>(&self) -> &'a [T] {
        self.make_slice()
    }

    /// Views the underlying data as a mutable subslice of the original data.
    ///
    /// To avoid creating `&mut [T]` references that alias, the returned slice
    /// borrows its lifetime from the iterator the method is applied on.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::try_vec;
    ///
    /// let mut slice = try_vec![1, 2, 3];
    ///
    /// unsafe {
    ///     // First, we get the iterator:
    ///     let mut iter = slice.raw_iter_mut();
    ///     // Then, we get a mutable slice from it:
    ///     let mut_slice = iter.as_mut_slice();
    ///     // So if we check what the `as_mut_slice` method returned, we have "[1, 2, 3]":
    ///     assert_eq!(mut_slice, &mut [1, 2, 3]);
    ///
    ///     // We can use it to mutate the slice:
    ///     mut_slice[0] = 4;
    ///     mut_slice[2] = 5;
    ///
    ///     // Next, we can move to the second element of the slice, checking that
    ///     // it yields the value we just wrote:
    ///     assert!(iter.next().is_some());
    ///     // Now `as_mut_slice` returns "[2, 5]":
    ///     assert_eq!(iter.as_mut_slice(), &mut [2, 5]);
    /// }
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[must_use]
    pub unsafe fn as_mut_slice<'a>(&mut self) -> &'a mut [T] {
        from_raw_parts_mut(self.ptr.as_ptr(), len!(self))
    }
}

iterator! {struct RawIterMut -> *mut T, *mut T, mut, {mut}, as_mut, {}}
