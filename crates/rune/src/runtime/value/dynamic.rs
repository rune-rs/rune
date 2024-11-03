use core::alloc::{Layout, LayoutError};
use core::cell::Cell;
use core::mem::{align_of, needs_drop, offset_of, replace, size_of};
use core::ptr::{self, addr_of, addr_of_mut, NonNull};

use crate::alloc;
use crate::alloc::alloc::{Allocator, Global};
use crate::runtime::{Access, AccessError, BorrowMut, BorrowRef};

/// A dynamic value defined at runtime.
///
/// This is an allocation-optimized container which allows an interior slice of
/// data `T` to be checked for access and `H` to be immutably accessed inside of
/// a single reference-counted container.
pub struct Dynamic<H, T> {
    shared: NonNull<Shared<H, T>>,
}

impl<H, T> Dynamic<H, T> {
    /// A dynamic value inside of the virtual machine.
    pub(crate) fn new(
        rtti: H,
        it: impl IntoIterator<Item = T, IntoIter: ExactSizeIterator>,
    ) -> alloc::Result<Self> {
        let it = it.into_iter();
        let layout = Shared::<H, T>::layout(it.len())?;

        let shared = Global.allocate(layout)?.cast::<Shared<H, T>>();

        // SAFETY: We've allocated space for both the shared header and the
        // trailing data.
        unsafe {
            shared.write(Shared {
                rtti,
                count: Cell::new(1),
                access: Access::new(),
                len: it.len(),
                data: [],
            });

            let data = shared.byte_add(offset_of!(Shared<H, T>, data)).cast::<T>();

            for (i, value) in it.enumerate() {
                data.add(i).write(value);
            }
        }

        Ok(Self { shared })
    }

    /// Get runtime type information of the dynamic value.
    pub(crate) fn rtti(&self) -> &H {
        // SAFETY: We know that the shared pointer is valid.
        unsafe { &self.shared.as_ref().rtti }
    }

    /// Borrow the interior data array by reference.
    pub(crate) fn borrow_ref(&self) -> Result<BorrowRef<[T]>, AccessError> {
        // SAFETY: We know the layout is valid since it is reference counted.
        unsafe {
            let guard = self.shared.as_ref().access.shared()?;
            let data = Shared::as_data_ptr(self.shared);
            let data = NonNull::slice_from_raw_parts(data, self.shared.as_ref().len);
            Ok(BorrowRef::new(data, guard.into_raw()))
        }
    }

    /// Borrow the interior data array by mutable reference.
    pub(crate) fn borrow_mut(&self) -> Result<BorrowMut<[T]>, AccessError> {
        // SAFETY: We know the layout is valid since it is reference counted.
        unsafe {
            let guard = self.shared.as_ref().access.exclusive()?;
            let data = Shared::as_data_ptr(self.shared);
            let data = NonNull::slice_from_raw_parts(data, self.shared.as_ref().len);
            Ok(BorrowMut::new(data, guard.into_raw()))
        }
    }
}

impl<H, T> Drop for Dynamic<H, T> {
    fn drop(&mut self) {
        // Decrement a shared value.
        unsafe {
            Shared::dec(self.shared);
        }
    }
}

impl<H, T> Clone for Dynamic<H, T> {
    #[inline]
    fn clone(&self) -> Self {
        // SAFETY: We know that the inner value is live in this instance.
        unsafe {
            Shared::inc(self.shared);
        }

        Self {
            shared: self.shared,
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        if ptr::eq(self.shared.as_ptr(), source.shared.as_ptr()) {
            return;
        }

        let old = replace(&mut self.shared, source.shared);

        // SAFETY: We know that the inner value is live in both instances.
        unsafe {
            Shared::dec(old);
            Shared::inc(self.shared);
        }
    }
}

struct Shared<H, T> {
    /// Run time type information of the shared value.
    rtti: H,
    /// Reference count.
    count: Cell<usize>,
    /// Access flags.
    access: Access,
    /// The size of the dynamic value.
    len: usize,
    /// Start of data pointer. Only used for alignment.
    data: [T; 0],
}

impl<H, T> Shared<H, T> {
    fn layout(len: usize) -> Result<Layout, LayoutError> {
        let array = Layout::array::<T>(len)?;
        Layout::from_size_align(
            size_of::<Shared<H, T>>() + array.size(),
            align_of::<Shared<H, T>>(),
        )
    }

    /// Get the rtti pointer in the shared container.
    unsafe fn as_rtti_ptr(this: NonNull<Self>) -> NonNull<H> {
        NonNull::new_unchecked(addr_of_mut!((*this.as_ptr()).rtti))
    }

    /// Get the data pointer in the shared container.
    unsafe fn as_data_ptr(this: NonNull<Self>) -> NonNull<T> {
        NonNull::new_unchecked(addr_of_mut!((*this.as_ptr()).data)).cast::<T>()
    }

    /// Increment the reference count of the inner value.
    unsafe fn inc(this: NonNull<Self>) {
        let count_ref = &*addr_of!((*this.as_ptr()).count);
        let count = count_ref.get();

        debug_assert_ne!(
            count, 0,
            "Reference count of zero should only happen if Shared is incorrectly implemented"
        );

        if count == usize::MAX {
            crate::alloc::abort();
        }

        count_ref.set(count + 1);
    }

    /// Decrement the reference count in inner, and free the underlying data if
    /// it has reached zero.
    ///
    /// # Safety
    ///
    /// ProtocolCaller needs to ensure that `this` is a valid pointer.
    unsafe fn dec(this: NonNull<Self>) {
        let count_ref = &*addr_of!((*this.as_ptr()).count);
        let count = count_ref.get();

        debug_assert_ne!(
            count, 0,
            "Reference count of zero should only happen if Shared is incorrectly implemented"
        );

        let count = count - 1;
        count_ref.set(count);

        if count != 0 {
            return;
        }

        let len = (*this.as_ptr()).len;

        let Ok(layout) = Self::layout(len) else {
            unreachable!();
        };

        if needs_drop::<T>() {
            let data = Self::as_data_ptr(this);
            NonNull::slice_from_raw_parts(data, len).drop_in_place();
        }

        if needs_drop::<H>() {
            Self::as_rtti_ptr(this).drop_in_place();
        }

        Global.deallocate(this.cast(), layout);
    }
}
