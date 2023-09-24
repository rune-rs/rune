use core::alloc::Layout;

use crate::alloc::{AllocError, Allocator};
use crate::ptr::{invalid_mut, NonNull};

#[cfg(feature = "alloc")]
use ::rust_alloc::alloc::{alloc, alloc_zeroed, dealloc};

/// Creates a `NonNull` that is dangling, but well-aligned for this Layout.
///
/// Note that the pointer value may potentially represent a valid pointer, which
/// means this must not be used as a "not yet initialized" sentinel value. Types
/// that lazily allocate must track initialization by some other means.
pub(crate) const fn dangling<T>(layout: &Layout) -> NonNull<T> {
    unsafe { NonNull::new_unchecked(invalid_mut::<T>(layout.align())) }
}

/// The default global allocator for Rune.
///
/// This supports enforcing thread-local memory limits through the [`limit`]
/// module.
///
/// [`limit`]: crate::limit
#[derive(Default, Debug, Clone, Copy)]
pub struct Global;

impl Global {
    /// Release the specified memory from being accounted for.
    pub(crate) fn release(&self, layout: Layout) {
        crate::limit::release(layout.size());
    }

    /// Acquire the specified memory.
    pub(crate) fn take(&self, layout: Layout) -> Result<(), AllocError> {
        if !crate::limit::take(layout.size()) {
            return Err(AllocError { layout });
        }

        Ok(())
    }

    #[inline]
    fn alloc_impl(&self, layout: Layout, zeroed: bool) -> Result<NonNull<[u8]>, AllocError> {
        self.take(layout)?;

        match layout.size() {
            0 => Ok(NonNull::slice_from_raw_parts(dangling(&layout), 0)),
            // SAFETY: `layout` is non-zero in size,
            size => unsafe {
                let raw_ptr = if zeroed {
                    alloc_zeroed(layout)
                } else {
                    alloc(layout)
                };

                let Some(ptr) = NonNull::new(raw_ptr) else {
                    return Err(AllocError { layout });
                };

                Ok(NonNull::slice_from_raw_parts(ptr, size))
            },
        }
    }
}

unsafe impl Allocator for Global {
    #[inline]
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.alloc_impl(layout, false)
    }

    #[inline]
    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.alloc_impl(layout, true)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        if layout.size() != 0 {
            // SAFETY: `layout` is non-zero in size,
            // other conditions must be upheld by the caller
            dealloc(ptr.as_ptr(), layout);
            self.release(layout);
        }
    }
}
