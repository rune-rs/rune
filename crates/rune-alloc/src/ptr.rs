//! This module contains (hopefully sound) re-implementations of unstable
//! `core::ptr` APIs.

pub(crate) use self::unique::Unique;
mod unique;

use core::mem;
pub(crate) use core::ptr::NonNull;

// Stable re-exports.
pub(crate) use core::ptr::{
    addr_of, addr_of_mut, copy, copy_nonoverlapping, drop_in_place, read, slice_from_raw_parts_mut,
    write,
};

pub(crate) const unsafe fn nonnull_add<T>(this: NonNull<T>, delta: usize) -> NonNull<T>
where
    T: Sized,
{
    // SAFETY: We require that the delta stays in-bounds of the object, and
    // thus it cannot become null, as that would require wrapping the
    // address space, which no legal objects are allowed to do.
    // And the caller promised the `delta` is sound to add.
    let pointer = this.as_ptr();
    unsafe { NonNull::new_unchecked(pointer.add(delta)) }
}

pub(crate) const unsafe fn nonnull_sub<T>(this: NonNull<T>, delta: usize) -> NonNull<T>
where
    T: Sized,
{
    // SAFETY: We require that the delta stays in-bounds of the object, and
    // thus it cannot become null, as that would require wrapping the
    // address space, which no legal objects are allowed to do.
    // And the caller promised the `delta` is sound to add.
    let pointer = this.as_ptr();
    unsafe { NonNull::new_unchecked(pointer.sub(delta)) }
}

#[inline(always)]
#[allow(clippy::useless_transmute)]
pub const fn invalid<T>(addr: usize) -> *const T {
    // FIXME(strict_provenance_magic): I am magic and should be a compiler intrinsic.
    // We use transmute rather than a cast so tools like Miri can tell that this
    // is *not* the same as from_exposed_addr.
    // SAFETY: every valid integer is also a valid pointer (as long as you don't dereference that
    // pointer).
    unsafe { mem::transmute(addr) }
}

#[inline(always)]
#[allow(clippy::useless_transmute)]
pub const fn invalid_mut<T>(addr: usize) -> *mut T {
    // FIXME(strict_provenance_magic): I am magic and should be a compiler intrinsic.
    // We use transmute rather than a cast so tools like Miri can tell that this
    // is *not* the same as from_exposed_addr.
    // SAFETY: every valid integer is also a valid pointer (as long as you don't dereference that
    // pointer).
    unsafe { mem::transmute(addr) }
}

cfg_if! {
    if #[cfg(rune_nightly)] {
        #[inline(always)]
        pub(crate) const fn wrapping_byte_sub<T>(ptr: *const T, count: usize) -> *const T {
            ptr.cast::<u8>().wrapping_sub(count).with_metadata_of(ptr)
        }
    } else {
        #[inline(always)]
        pub(crate) const fn wrapping_byte_sub<T>(ptr: *const T, count: usize) -> *const T {
            // TODO: We need to use metadata.
            ptr.cast::<u8>().wrapping_sub(count) as *const T
        }
    }
}

cfg_if! {
    if #[cfg(rune_nightly)] {
        #[inline(always)]
        pub(crate) unsafe fn sub_ptr<T>(from: *const T, to: *const T) -> usize {
            from.sub_ptr(to)
        }
    } else {
        #[inline(always)]
        pub(crate) unsafe fn sub_ptr<T>(from: *const T, to: *const T) -> usize {
            let pointee_size = mem::size_of::<T>();
            assert!(0 < pointee_size && pointee_size <= isize::MAX as usize);
            debug_assert!(addr(from) >= addr(to));
            addr(from).wrapping_sub(addr(to)).saturating_div(pointee_size)
        }
    }
}

cfg_if! {
    if #[cfg(rune_nightly)] {
        #[inline(always)]
        pub(crate) fn addr<T>(from: *const T) -> usize {
            from.addr()
        }
    } else {
        #[inline(always)]
        pub(crate) fn addr<T>(from: *const T) -> usize {
            from as usize
        }
    }
}

cfg_if! {
    if #[cfg(rune_nightly)] {
        #[inline(always)]
        pub(crate) fn slice_len<T>(from: *const [T]) -> usize {
            from.len()
        }
    } else {
        #[inline(always)]
        pub(crate) fn slice_len<T>(from: *const [T]) -> usize {
            // SAFETY: This is *a bit* tricky, but the raw pointer contains the
            // length and *should* be safe to dereference like this. However,
            // walking through the dereferenced `[T]` is not necessarily
            // correct.
            unsafe { (*from).len() }
        }
    }
}
