use crate::alloc::{Allocator, Box, Error, Global, TryClone, TryToOwned, Vec};

/// Converts `self` into a vector without clones or allocation.
///
/// The resulting vector can be converted back into a box via
/// `Vec<T>`'s `into_boxed_slice` method.
///
/// # Examples
///
/// ```
/// let s: Box<[i32]> = Box::new([10, 40, 30]);
/// let x = s.into_vec();
/// // `s` cannot be used anymore because it has been converted into `x`.
///
/// assert_eq!(x, vec![10, 40, 30]);
/// ```
#[inline]
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
    use crate::alloc::{Allocator, Box, Error, TryClone, Vec};

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

    #[cfg(rune_nightly)]
    impl<T: TryCopy> ConvertVec for T {
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
