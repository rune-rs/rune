pub use self::iter::{RawIter, RawIterMut};
pub(crate) mod iter;

use crate::alloc::{Allocator, Global};
use crate::borrow::TryToOwned;
use crate::clone::TryClone;
use crate::error::Error;
use crate::{Box, Vec};

cfg_if! {
    if #[cfg(rune_nightly)] {
        pub(crate) use core::slice::range;
    } else {
        use core::ops;

        #[must_use]
        pub(crate) fn range<R>(range: R, bounds: ops::RangeTo<usize>) -> ops::Range<usize>
        where
            R: ops::RangeBounds<usize>,
        {
            let len = bounds.end;

            let start: ops::Bound<&usize> = range.start_bound();
            let start = match start {
                ops::Bound::Included(&start) => start,
                ops::Bound::Excluded(start) => start
                    .checked_add(1)
                    .unwrap_or_else(|| slice_start_index_overflow_fail()),
                ops::Bound::Unbounded => 0,
            };

            let end: ops::Bound<&usize> = range.end_bound();
            let end = match end {
                ops::Bound::Included(end) => end
                    .checked_add(1)
                    .unwrap_or_else(|| slice_end_index_overflow_fail()),
                ops::Bound::Excluded(&end) => end,
                ops::Bound::Unbounded => len,
            };

            if start > end {
                slice_index_order_fail(start, end);
            }
            if end > len {
                slice_end_index_len_fail(end, len);
            }

            ops::Range { start, end }
        }

        const fn slice_start_index_overflow_fail() -> ! {
            panic!("attempted to index slice from after maximum usize");
        }

        const fn slice_end_index_overflow_fail() -> ! {
            panic!("attempted to index slice up to maximum usize");
        }

        fn slice_index_order_fail(index: usize, end: usize) -> ! {
            panic!("slice index starts at {index} but ends at {end}");
        }

        fn slice_end_index_len_fail(index: usize, len: usize) -> ! {
            panic!("range end index {index} out of range for slice of length {len}");
        }
    }
}

/// Converts `self` into a vector without clones or allocation.
///
/// The resulting vector can be converted back into a box via
/// `Vec<T>`'s `into_boxed_slice` method.
#[inline]
#[doc(hidden)]
pub fn into_vec<T, A: Allocator>(this: Box<[T], A>) -> Vec<T, A> {
    // N.B., see the `hack` module in this file for more details.
    hack::into_vec(this)
}

#[inline]
pub(crate) fn to_vec<T, A: Allocator>(s: &[T], alloc: A) -> Result<Vec<T, A>, Error>
where
    T: TryClone,
{
    hack::to_vec(s, alloc)
}

impl<T> TryToOwned for [T]
where
    T: TryClone,
{
    type Owned = Vec<T, Global>;

    #[inline]
    fn try_to_owned(&self) -> Result<Self::Owned, Error> {
        hack::to_vec(self, Global)
    }
}

// HACK(japaric): With cfg(test) `impl [T]` is not available, these three
// functions are actually methods that are in `impl [T]` but not in
// `core::slice::SliceExt` - we need to supply these functions for the
// `test_permutations` test
pub(crate) mod hack {
    use crate::alloc::Allocator;
    use crate::clone::TryClone;
    use crate::error::Error;
    use crate::{Box, Vec};

    // We shouldn't add inline attribute to this since this is used in `vec!`
    // macro mostly and causes perf regression. See #71204 for discussion and
    // perf results.
    pub(crate) fn into_vec<T, A: Allocator>(b: Box<[T], A>) -> Vec<T, A> {
        unsafe {
            let len = b.len();
            let (b, alloc) = Box::into_raw_with_allocator(b);
            Vec::from_raw_parts_in(b as *mut T, len, len, alloc)
        }
    }

    #[inline]
    pub(crate) fn to_vec<T: ConvertVec, A: Allocator>(
        s: &[T],
        alloc: A,
    ) -> Result<Vec<T, A>, Error> {
        T::to_vec(s, alloc)
    }

    pub(crate) trait ConvertVec {
        fn to_vec<A: Allocator>(s: &[Self], alloc: A) -> Result<Vec<Self, A>, Error>
        where
            Self: Sized;
    }

    impl<T> ConvertVec for T
    where
        T: TryClone,
    {
        default_fn! {
            #[inline]
            fn to_vec<A: Allocator>(s: &[Self], alloc: A) -> Result<Vec<Self, A>, Error> {
                struct DropGuard<'a, T, A: Allocator> {
                    vec: &'a mut Vec<T, A>,
                    num_init: usize,
                }

                impl<'a, T, A: Allocator> Drop for DropGuard<'a, T, A> {
                    #[inline]
                    fn drop(&mut self) {
                        // SAFETY:
                        // items were marked initialized in the loop below
                        unsafe {
                            self.vec.set_len(self.num_init);
                        }
                    }
                }
                let mut vec = Vec::try_with_capacity_in(s.len(), alloc)?;
                let mut guard = DropGuard {
                    vec: &mut vec,
                    num_init: 0,
                };
                let slots = guard.vec.spare_capacity_mut();
                // .take(slots.len()) is necessary for LLVM to remove bounds checks
                // and has better codegen than zip.
                for (i, b) in s.iter().enumerate().take(slots.len()) {
                    guard.num_init = i;
                    slots[i].write(b.try_clone()?);
                }
                core::mem::forget(guard);
                // SAFETY:
                // the vec was allocated and initialized above to at least this length.
                unsafe {
                    vec.set_len(s.len());
                }
                Ok(vec)
            }
        }
    }

    #[cfg(rune_nightly)]
    impl<T: crate::clone::TryCopy> ConvertVec for T {
        #[inline]
        fn to_vec<A: Allocator>(s: &[Self], alloc: A) -> Result<Vec<Self, A>, Error> {
            let mut v = Vec::try_with_capacity_in(s.len(), alloc)?;

            // SAFETY:
            // allocated above with the capacity of `s`, and initialize to `s.len()` in
            // ptr::copy_to_non_overlapping below.
            unsafe {
                s.as_ptr().copy_to_nonoverlapping(v.as_mut_ptr(), s.len());
                v.set_len(s.len());
            }
            Ok(v)
        }
    }
}
