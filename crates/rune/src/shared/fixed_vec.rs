use core::fmt;
use core::mem::{self, ManuallyDrop, MaybeUninit};
use core::ops::{Deref, DerefMut};
use core::ptr;
use core::slice;

/// An error raised when we are at capacity.
#[derive(Debug)]
#[non_exhaustive]
pub(crate) struct CapacityError;

impl fmt::Display for CapacityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Out of capacity when constructing array")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CapacityError {}

/// A fixed capacity vector allocated on the stack.
pub(crate) struct FixedVec<T, const N: usize> {
    data: [MaybeUninit<T>; N],
    len: usize,
}

impl<T, const N: usize> FixedVec<T, N> {
    /// Construct a new empty fixed vector.
    pub(crate) const fn new() -> FixedVec<T, N> {
        unsafe {
            FixedVec {
                data: MaybeUninit::uninit().assume_init(),
                len: 0,
            }
        }
    }

    #[inline]
    pub(crate) fn as_ptr(&self) -> *const T {
        self.data.as_ptr() as *const T
    }

    #[inline]
    pub(crate) fn as_mut_ptr(&mut self) -> *mut T {
        self.data.as_mut_ptr() as *mut T
    }

    #[inline]
    pub(crate) fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.as_ptr(), self.len) }
    }

    #[inline]
    pub(crate) fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), self.len) }
    }

    /// Try to push an element onto the fixed vector.
    pub(crate) fn try_push(&mut self, element: T) -> Result<(), CapacityError> {
        if self.len >= N {
            return Err(CapacityError);
        }

        unsafe {
            ptr::write(self.as_mut_ptr().wrapping_add(self.len), element);
            self.len += 1;
        }

        Ok(())
    }

    pub(crate) fn clear(&mut self) {
        if self.len == 0 {
            return;
        }

        let len = mem::take(&mut self.len);

        if mem::needs_drop::<T>() {
            unsafe {
                let tail = slice::from_raw_parts_mut(self.as_mut_ptr(), len);
                ptr::drop_in_place(tail);
            }
        }
    }

    pub(crate) fn into_inner(self) -> [T; N] {
        assert!(
            self.len == N,
            "into_inner: length mismatch, expected {N} but got {}",
            self.len
        );

        // SAFETY: We've asserted that the length is initialized just above.
        unsafe {
            let this = ManuallyDrop::new(self);
            ptr::read(this.data.as_ptr() as *const [T; N])
        }
    }
}

impl<T, const N: usize> Deref for FixedVec<T, N> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T, const N: usize> DerefMut for FixedVec<T, N> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T, const N: usize> Drop for FixedVec<T, N> {
    #[inline]
    fn drop(&mut self) {
        self.clear()
    }
}
