use core::cell::{Cell, UnsafeCell};
use core::fmt;
use core::mem::{transmute, ManuallyDrop};
use core::ptr::{self, NonNull};

use crate::alloc::prelude::*;
use crate::alloc::{self, Box};

use super::{
    Access, AccessError, BorrowMut, BorrowRef, Mut, RawAnyGuard, Ref, RefVtable, Snapshot,
};

/// A shared value.
#[repr(transparent)]
pub(crate) struct Shared<T: ?Sized> {
    inner: NonNull<SharedBox<T>>,
}

impl<T> Shared<T> {
    /// Construct a new shared value.
    pub(crate) fn new(data: T) -> alloc::Result<Self> {
        let shared = SharedBox {
            access: Access::new(),
            count: Cell::new(1),
            data: data.into(),
        };

        let inner = Box::leak(Box::try_new(shared)?);

        Ok(Self {
            inner: inner.into(),
        })
    }

    /// Test if the value is sharable.
    pub(crate) fn is_readable(&self) -> bool {
        // Safety: Since we have a reference to this shared, we know that the
        // inner is available.
        unsafe { self.inner.as_ref().access.is_shared() }
    }

    /// Test if the value is exclusively accessible.
    pub(crate) fn is_writable(&self) -> bool {
        // Safety: Since we have a reference to this shared, we know that the
        // inner is available.
        unsafe { self.inner.as_ref().access.is_exclusive() }
    }

    /// Get access snapshot of shared value.
    pub(crate) fn snapshot(&self) -> Snapshot {
        unsafe { self.inner.as_ref().access.snapshot() }
    }

    /// Take the interior value, if we have exlusive access to it and there
    /// are no other live exlusive or shared references.
    ///
    /// A value that has been taken can no longer be accessed.
    pub(crate) fn take(self) -> Result<T, AccessError> {
        // Safety: We know that interior value is alive since this container is
        // alive.
        //
        // Appropriate access is checked when constructing the guards.
        unsafe {
            let inner = self.inner.as_ref();

            // Try to take the interior value, this should *only* work if the
            // access is exclusively available.
            inner.access.try_take()?;

            // Read the pointer out without dropping the inner structure.
            // The data field will be invalid at this point, which should be
            // flagged through a `taken` access flag.
            //
            // Future access is forever prevented since we never release
            // the access (see above).
            Ok(ptr::read(inner.data.get()))
        }
    }

    /// Get a reference to the interior value while checking for shared access
    /// that holds onto a reference count of the inner value.
    ///
    /// This prevents other exclusive accesses from being performed while the
    /// guard returned from this function is live.
    pub(crate) fn into_ref(self) -> Result<Ref<T>, AccessError> {
        // Safety: We know that interior value is alive since this container is
        // alive.
        //
        // Appropriate access is checked when constructing the guards.
        unsafe {
            self.inner.as_ref().access.try_shared()?;
            let this = ManuallyDrop::new(self);
            Ok(ref_from_shared(this.inner))
        }
    }

    /// Get a reference to the interior value while checking for exclusive
    /// access that holds onto a reference count of the inner value.
    ///
    /// This prevents other exclusive and shared accesses from being performed
    /// while the guard returned from this function is live.
    pub(crate) fn into_mut(self) -> Result<Mut<T>, AccessError> {
        // Safety: We know that interior value is alive since this container is
        // alive.
        //
        // Appropriate access is checked when constructing the guards.
        unsafe {
            self.inner.as_ref().access.try_exclusive()?;
            let this = ManuallyDrop::new(self);
            Ok(mut_from_shared(this.inner))
        }
    }
}

impl<T: ?Sized> Shared<T> {
    /// Get a reference to the interior value while checking for shared access.
    ///
    /// This prevents other exclusive accesses from being performed while the
    /// guard returned from this function is live.
    pub(crate) fn borrow_ref(&self) -> Result<BorrowRef<'_, T>, AccessError> {
        // Safety: We know that interior value is alive since this container is
        // alive.
        //
        // Appropriate access is checked when constructing the guards.
        unsafe {
            let inner = self.inner.as_ref();
            let guard = inner.access.shared()?;

            Ok(BorrowRef::new(
                NonNull::new_unchecked(inner.data.get()),
                guard.into_raw(),
            ))
        }
    }

    /// Get a reference to the interior value while checking for exclusive access.
    ///
    /// This prevents other shared or exclusive accesses from being performed
    /// while the guard returned from this function is live.
    pub(crate) fn borrow_mut(&self) -> Result<BorrowMut<'_, T>, AccessError> {
        // Safety: We know that interior value is alive since this container is
        // alive.
        //
        // Appropriate access is checked when constructing the guards.
        unsafe {
            let inner = self.inner.as_ref();
            let guard = inner.access.exclusive()?;

            Ok(BorrowMut::new(
                NonNull::new_unchecked(inner.data.get()),
                guard.into_raw(),
            ))
        }
    }
}

impl<T: ?Sized> fmt::Pointer for Shared<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.as_ptr(), fmt)
    }
}

impl<T: ?Sized> TryClone for Shared<T> {
    #[inline]
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(self.clone())
    }
}

impl<T: ?Sized> Clone for Shared<T> {
    #[inline]
    fn clone(&self) -> Self {
        // SAFETY: We know that the inner value is live in this instance.
        unsafe {
            SharedBox::inc(self.inner);
        }

        Self { inner: self.inner }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        if ptr::eq(self.inner.as_ptr(), source.inner.as_ptr()) {
            return;
        }

        // SAFETY: We know that the inner value is live in both instances.
        unsafe {
            SharedBox::dec(self.inner);
            SharedBox::inc(source.inner);
        }

        self.inner = source.inner;
    }
}

impl<T: ?Sized> Drop for Shared<T> {
    fn drop(&mut self) {
        unsafe {
            SharedBox::dec(self.inner);
        }
    }
}

impl<T: ?Sized> fmt::Debug for Shared<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Safety: by virtue of holding onto a shared we can safely access
        // `inner` because it must outlive any `Shared` instances.
        unsafe {
            let inner = self.inner.as_ref();

            if !inner.access.is_shared() {
                write!(fmt, "*not accessible*")
            } else {
                write!(fmt, "{:?}", &&*inner.data.get())
            }
        }
    }
}

/// The boxed internals of [Shared].
#[repr(C)]
struct SharedBox<T: ?Sized> {
    /// The access of the shared data.
    access: Access,
    /// The number of strong references to the shared data.
    count: Cell<usize>,
    /// The value being held. Guarded by the `access` field to determine if it
    /// can be access shared or exclusively.
    data: UnsafeCell<T>,
}

impl<T: ?Sized> SharedBox<T> {
    /// Increment the reference count of the inner value.
    unsafe fn inc(this: NonNull<Self>) {
        let count_ref = &*ptr::addr_of!((*this.as_ptr()).count);
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
    unsafe fn dec(this: NonNull<Self>) -> bool {
        let count_ref = &*ptr::addr_of!((*this.as_ptr()).count);

        let count = count_ref.get();

        debug_assert_ne!(
            count, 0,
            "Reference count of zero should only happen if Shared is incorrectly implemented"
        );

        let count = count - 1;
        count_ref.set(count);

        if count != 0 {
            return false;
        }

        let this = Box::from_raw_in(this.as_ptr(), rune_alloc::alloc::Global);

        if this.access.is_taken() {
            // NB: This prevents the inner `T` from being dropped in case it
            // has already been taken (as indicated by `is_taken`).
            //
            // If it has been taken, the shared box contains invalid memory.
            let this = transmute::<Box<SharedBox<T>>, Box<SharedBox<ManuallyDrop<T>>>>(this);
            drop(this);
        } else {
            // NB: At the point of the final drop, no on else should be using
            // this.
            debug_assert!(
                this.access.is_exclusive(),
                "expected exclusive, but was: {:?}",
                this.access
            );
        }

        true
    }
}

unsafe fn drop_shared<T>(data: NonNull<()>) {
    let data = data.cast::<SharedBox<T>>();
    data.as_ref().access.release();
    SharedBox::dec(data);
}

unsafe fn ref_from_shared<T>(data: NonNull<SharedBox<T>>) -> Ref<T> {
    let value = &*ptr::addr_of!((*data.as_ptr()).data);
    let value = NonNull::new_unchecked(value.get()).cast();

    let guard = RawAnyGuard::new(
        data.cast(),
        &RefVtable {
            drop: drop_shared::<T>,
        },
    );

    Ref::new(value, guard)
}

unsafe fn mut_from_shared<T>(data: NonNull<SharedBox<T>>) -> Mut<T> {
    let value = &*ptr::addr_of_mut!((*data.as_ptr()).data);
    let value = NonNull::new_unchecked(value.get()).cast();

    let guard = RawAnyGuard::new(
        data.cast(),
        &RefVtable {
            drop: drop_shared::<T>,
        },
    );

    Mut::new(value, guard)
}
