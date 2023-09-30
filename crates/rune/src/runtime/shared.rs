use core::any::{self, TypeId};
use core::cell::{Cell, UnsafeCell};
use core::fmt;
use core::future::Future;
use core::mem::{self, transmute, ManuallyDrop};
use core::ops;
use core::pin::Pin;
use core::ptr;
use core::task::{Context, Poll};

#[cfg(feature = "alloc")]
use ::rust_alloc::rc::Rc;
#[cfg(feature = "alloc")]
use ::rust_alloc::sync::Arc;

use crate::alloc::prelude::*;
use crate::alloc::{self, Box};
use crate::runtime::{
    Access, AccessError, AccessKind, AnyObj, AnyObjError, BorrowMut, BorrowRef, RawAccessGuard,
};
use crate::Any;

/// A shared value.
pub struct Shared<T: ?Sized> {
    inner: ptr::NonNull<SharedBox<T>>,
}

impl<T> Shared<T> {
    /// Construct a new shared value.
    pub fn new(data: T) -> alloc::Result<Self> {
        let shared = SharedBox {
            access: Access::new(false),
            count: Cell::new(1),
            data: data.into(),
        };

        let inner = Box::leak(Box::try_new(shared)?);

        Ok(Self {
            inner: inner.into(),
        })
    }

    /// Return a debug formatter, that when printed will display detailed
    /// diagnostics of this shared type.
    pub fn debug(&self) -> SharedDebug<'_, T> {
        SharedDebug { shared: self }
    }

    /// Test if the value is sharable.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Shared;
    ///
    /// let shared = Shared::new(1u32)?;
    /// assert!(shared.is_readable());
    ///
    /// {
    ///     let guard = shared.borrow_ref().unwrap();
    ///     assert!(shared.is_readable()); // Note: still readable.
    /// }
    ///
    /// {
    ///     let guard = shared.borrow_mut().unwrap();
    ///     assert!(!shared.is_readable());
    /// }
    ///
    /// assert!(shared.is_readable());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// # Taking inner value
    ///
    /// ```
    /// use rune::runtime::Shared;
    ///
    /// let shared = Shared::new(1u32)?;
    /// let shared2 = shared.clone();
    /// assert!(shared.is_readable());
    /// shared.take().unwrap();
    /// assert!(!shared2.is_readable());
    /// assert!(shared2.take().is_err());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn is_readable(&self) -> bool {
        // Safety: Since we have a reference to this shared, we know that the
        // inner is available.
        unsafe { self.inner.as_ref().access.is_shared() }
    }

    /// Test if the value is exclusively accessible.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Shared;
    ///
    /// let shared = Shared::new(1u32)?;
    /// assert!(shared.is_writable());
    ///
    /// {
    ///     let guard = shared.borrow_ref().unwrap();
    ///     assert!(!shared.is_writable());
    /// }
    ///
    /// assert!(shared.is_writable());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn is_writable(&self) -> bool {
        // Safety: Since we have a reference to this shared, we know that the
        // inner is available.
        unsafe { self.inner.as_ref().access.is_exclusive() }
    }

    /// Take the interior value, if we have exlusive access to it and there
    /// are no other live exlusive or shared references.
    ///
    /// A value that has been taken can no longer be accessed.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Shared;
    ///
    /// #[derive(Debug)]
    /// struct Foo {
    ///     counter: isize,
    /// }
    ///
    /// let a = Shared::new(Foo { counter: 0 })?;
    /// let b = a.clone();
    ///
    /// {
    ///     let mut a = a.borrow_mut().unwrap();
    ///     // NB: this is prevented since we have a live reference.
    ///     assert!(b.take().is_err());
    ///     a.counter += 1;
    /// }
    ///
    /// let a = a.take().unwrap();
    /// assert_eq!(a.counter, 1);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn take(self) -> Result<T, AccessError> {
        // Safety: We know that interior value is alive since this container is
        // alive.
        //
        // Appropriate access is checked when constructing the guards.
        unsafe {
            let inner = self.inner.as_ref();

            // NB: don't drop guard to avoid yielding access back.
            // This will prevent the value from being dropped in the shared
            // destructor and future illegal access of any kind.
            let _ = ManuallyDrop::new(inner.access.take(AccessKind::Any)?);

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
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Shared;
    ///
    /// #[derive(Debug)]
    /// struct Foo {
    ///     counter: isize,
    /// }
    ///
    /// let a = Shared::new(Foo { counter: 0 })?;
    /// let b = a.clone();
    ///
    /// b.borrow_mut().unwrap().counter += 1;
    ///
    /// {
    ///     // Consumes `a`.
    ///     let mut a = a.into_ref().unwrap();
    ///     assert_eq!(a.counter, 1);
    ///     assert!(b.borrow_mut().is_err());
    /// }
    ///
    /// let mut b = b.borrow_mut().unwrap();
    /// b.counter += 1;
    /// assert_eq!(b.counter, 2);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn into_ref(self) -> Result<Ref<T>, AccessError> {
        // NB: we default to a "safer" mode with `AccessKind::Owned`, where
        // references cannot be converted to an `Mut<T>` in order to avoid
        // a potential soundness panic.
        self.internal_into_ref(AccessKind::Owned)
    }

    /// Internal implementation of into_ref.
    pub(crate) fn internal_into_ref(self, kind: AccessKind) -> Result<Ref<T>, AccessError> {
        // Safety: We know that interior value is alive since this container is
        // alive.
        //
        // Appropriate access is checked when constructing the guards.
        unsafe {
            let guard = self.inner.as_ref().access.shared(kind)?.into_raw();

            // NB: we need to prevent the Drop impl for Shared from being called,
            // since we are deconstructing its internals.
            let this = ManuallyDrop::new(self);

            Ok(Ref {
                data: ptr::NonNull::new_unchecked(this.inner.as_ref().data.get()),
                guard: Some(guard),
                inner: RawDrop::decrement_shared_box(this.inner),
            })
        }
    }

    /// Get a reference to the interior value while checking for exclusive
    /// access that holds onto a reference count of the inner value.
    ///
    /// This prevents other exclusive and shared accesses from being performed
    /// while the guard returned from this function is live.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Shared;
    ///
    /// #[derive(Debug)]
    /// struct Foo {
    ///     counter: isize,
    /// }
    ///
    /// let a = Shared::new(Foo { counter: 0 })?;
    /// let b = a.clone();
    ///
    /// {
    ///     // Consumes `a`.
    ///     let mut a = a.into_mut().unwrap();
    ///     a.counter += 1;
    ///
    ///     assert!(b.borrow_ref().is_err());
    /// }
    ///
    /// assert_eq!(b.borrow_ref().unwrap().counter, 1);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn into_mut(self) -> Result<Mut<T>, AccessError> {
        // NB: we default to a "safer" mode with `AccessKind::Owned`, where
        // references cannot be converted to an `Mut<T>` in order to avoid
        // a potential soundness panic.
        self.internal_into_mut(AccessKind::Owned)
    }

    /// Internal implementation of into_mut.
    pub(crate) fn internal_into_mut(self, kind: AccessKind) -> Result<Mut<T>, AccessError> {
        // Safety: We know that interior value is alive since this container is
        // alive.
        //
        // Appropriate access is checked when constructing the guards.
        unsafe {
            let guard = self.inner.as_ref().access.exclusive(kind)?.into_raw();

            // NB: we need to prevent the Drop impl for Shared from being called,
            // since we are deconstructing its internals.
            let this = ManuallyDrop::new(self);

            Ok(Mut {
                data: ptr::NonNull::new_unchecked(this.inner.as_ref().data.get()),
                guard: Some(guard),
                inner: RawDrop::decrement_shared_box(this.inner),
            })
        }
    }
}

impl<T: ?Sized> Shared<T> {
    /// Get a reference to the interior value while checking for shared access.
    ///
    /// This prevents other exclusive accesses from being performed while the
    /// guard returned from this function is live.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Shared;
    ///
    /// #[derive(Debug)]
    /// struct Foo {
    ///     counter: isize,
    /// }
    ///
    /// let a = Shared::new(Foo { counter: 0 })?;
    ///
    /// a.borrow_mut().unwrap().counter += 1;
    ///
    /// {
    ///     let mut a_ref = a.borrow_ref().unwrap();
    ///     assert_eq!(a_ref.counter, 1);
    ///     assert!(a.borrow_mut().is_err());
    ///     assert!(a.borrow_ref().is_ok());
    /// }
    ///
    /// let mut a = a.borrow_mut().unwrap();
    /// a.counter += 1;
    /// assert_eq!(a.counter, 2);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn borrow_ref(&self) -> Result<BorrowRef<'_, T>, AccessError> {
        // Safety: We know that interior value is alive since this container is
        // alive.
        //
        // Appropriate access is checked when constructing the guards.
        unsafe {
            let inner = self.inner.as_ref();
            let guard = inner.access.shared(AccessKind::Any)?;
            mem::forget(guard);
            Ok(BorrowRef::new(&*inner.data.get(), &inner.access))
        }
    }

    /// Get a reference to the interior value while checking for exclusive access.
    ///
    /// This prevents other shared or exclusive accesses from being performed
    /// while the guard returned from this function is live.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Shared;
    ///
    /// #[derive(Debug)]
    /// struct Foo {
    ///     counter: isize,
    /// }
    ///
    /// let a = Shared::new(Foo { counter: 0 })?;
    ///
    /// {
    ///     let mut a_mut = a.borrow_mut().unwrap();
    ///     a_mut.counter += 1;
    ///     assert_eq!(a_mut.counter, 1);
    ///     assert!(a.borrow_ref().is_err());
    /// }
    ///
    /// let a = a.borrow_ref().unwrap();
    /// assert_eq!(a.counter, 1);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn borrow_mut(&self) -> Result<BorrowMut<'_, T>, AccessError> {
        // Safety: We know that interior value is alive since this container is
        // alive.
        //
        // Appropriate access is checked when constructing the guards.
        unsafe {
            let inner = self.inner.as_ref();
            let guard = inner.access.exclusive(AccessKind::Any)?;
            mem::forget(guard);
            Ok(BorrowMut::new(&mut *inner.data.get(), &inner.access))
        }
    }
}

impl Shared<AnyObj> {
    /// Construct a `Shared<Any>` from a pointer, that will be "taken" once the
    /// returned guard is dropped.
    ///
    /// # Safety
    ///
    /// The reference must be valid for the duration of the guard.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Any;
    /// use rune::runtime::Shared;
    ///
    /// #[derive(Any)]
    /// struct Thing(u32);
    ///
    /// let value = Thing(10u32);
    ///
    /// unsafe {
    ///     let (shared, guard) = Shared::from_ref(&value)?;
    ///     assert!(shared.downcast_borrow_mut::<Thing>().is_err());
    ///     assert_eq!(10u32, shared.downcast_borrow_ref::<Thing>().unwrap().0);
    ///
    ///     drop(guard);
    ///
    ///     assert!(shared.downcast_borrow_mut::<Thing>().is_err());
    ///     assert!(shared.downcast_borrow_ref::<Thing>().is_err());
    /// }
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub unsafe fn from_ref<T>(data: &T) -> alloc::Result<(Self, SharedPointerGuard)>
    where
        T: Any,
    {
        Self::unsafe_from_any_pointer(AnyObj::from_ref(data))
    }

    /// Construct a `Shared<Any>` from a mutable pointer, that will be "taken"
    /// once the returned guard is dropped.
    ///
    /// # Safety
    ///
    /// The reference must be valid for the duration of the guard.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Any;
    /// use rune::runtime::Shared;
    ///
    /// #[derive(Any)]
    /// struct Thing(u32);
    ///
    /// let mut value = Thing(10u32);
    ///
    /// unsafe {
    ///     let (shared, guard) = Shared::from_mut(&mut value)?;
    ///     shared.downcast_borrow_mut::<Thing>().unwrap().0 = 20;
    ///
    ///     assert_eq!(20u32, shared.downcast_borrow_mut::<Thing>().unwrap().0);
    ///     assert_eq!(20u32, shared.downcast_borrow_ref::<Thing>().unwrap().0);
    ///
    ///     drop(guard);
    ///
    ///     assert!(shared.downcast_borrow_mut::<Thing>().is_err());
    ///     assert!(shared.downcast_borrow_ref::<Thing>().is_err());
    /// }
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub unsafe fn from_mut<T>(data: &mut T) -> alloc::Result<(Self, SharedPointerGuard)>
    where
        T: Any,
    {
        Self::unsafe_from_any_pointer(AnyObj::from_mut(data))
    }

    /// Construct a `Shared<Any>` from an Any which is expected to wrap a
    /// pointer, that will be "taken" once the returned guard is dropped.
    ///
    /// # Safety
    ///
    /// The reference must be valid for the duration of the guard.
    unsafe fn unsafe_from_any_pointer(any: AnyObj) -> alloc::Result<(Self, SharedPointerGuard)> {
        let shared = SharedBox {
            access: Access::new(true),
            count: Cell::new(2),
            data: any.into(),
        };
        let inner = ptr::NonNull::from(Box::leak(Box::try_new(shared)?));

        let guard = SharedPointerGuard {
            _inner: RawDrop::take_shared_box(inner),
        };

        let value = Self { inner };
        Ok((value, guard))
    }

    /// Take the interior value, if we have exlusive access to it and there
    /// exist no other references.
    pub fn take_downcast<T>(self) -> Result<T, AccessError>
    where
        T: Any,
    {
        // Safety: We know that interior value is alive since this container is
        // alive.
        //
        // Appropriate access is checked when constructing the guards.
        unsafe {
            let inner = self.inner.as_ref();

            // NB: don't drop guard to avoid yielding access back.
            // This will prevent the value from being dropped in the shared
            // destructor and future illegal access of any kind.
            let guard = ManuallyDrop::new(inner.access.take(AccessKind::Any)?);

            // Read the pointer out without dropping the inner structure.
            // Note that the data field will after this point be invalid.
            //
            // Future access is forever prevented since we never release
            // exclusive access (see above).
            let any = ptr::read(inner.data.get());

            let expected = TypeId::of::<T>();

            let (e, any) = match any.raw_take(expected) {
                Ok(value) => {
                    return Ok(Box::into_inner(Box::from_raw_in(
                        value as *mut T,
                        rune_alloc::alloc::Global,
                    )))
                }
                Err((AnyObjError::Cast, any)) => {
                    let actual = any.type_name();

                    let e = AccessError::UnexpectedType {
                        actual,
                        expected: any::type_name::<T>().into(),
                    };

                    (e, any)
                }
                Err((e, any)) => (e.into(), any),
            };

            // At this point type coercion has failed for one reason or another,
            // so we reconstruct the state of the Shared container so that it
            // can be more cleanly dropped.

            // Drop the guard to release exclusive access.
            drop(ManuallyDrop::into_inner(guard));

            // Write the potentially modified value back so that it can be used
            // by other `Shared<T>` users pointing to the same value. This
            // conveniently also avoids dropping `any` which will be done by
            // `Shared` as appropriate.
            ptr::write(inner.data.get(), any);
            Err(e)
        }
    }

    /// Get an shared, downcasted reference to the contained value.
    pub fn downcast_borrow_ref<T>(&self) -> Result<BorrowRef<'_, T>, AccessError>
    where
        T: Any,
    {
        unsafe {
            let inner = self.inner.as_ref();
            let guard = inner.access.shared(AccessKind::Any)?;
            let expected = TypeId::of::<T>();

            let data = match (*inner.data.get()).raw_as_ptr(expected) {
                Ok(data) => data,
                Err(AnyObjError::Cast) => {
                    return Err(AccessError::UnexpectedType {
                        expected: any::type_name::<T>().into(),
                        actual: (*inner.data.get()).type_name(),
                    });
                }
                Err(e) => {
                    return Err(e.into());
                }
            };

            mem::forget(guard);
            Ok(BorrowRef::new(&*(data as *const T), &inner.access))
        }
    }

    /// Get an exclusive, downcasted reference to the contained value.
    pub fn downcast_borrow_mut<T>(&self) -> Result<BorrowMut<'_, T>, AccessError>
    where
        T: Any,
    {
        unsafe {
            let inner = self.inner.as_ref();
            let guard = inner.access.exclusive(AccessKind::Any)?;
            let expected = TypeId::of::<T>();

            let data = match (*inner.data.get()).raw_as_mut(expected) {
                Ok(data) => data,
                Err(AnyObjError::Cast) => {
                    return Err(AccessError::UnexpectedType {
                        expected: any::type_name::<T>().into(),
                        actual: (*inner.data.get()).type_name(),
                    });
                }
                Err(e) => {
                    return Err(e.into());
                }
            };

            mem::forget(guard);
            Ok(BorrowMut::new(&mut *(data as *mut T), &inner.access))
        }
    }

    /// Get a shared value and downcast.
    pub fn downcast_into_ref<T>(self) -> Result<Ref<T>, AccessError>
    where
        T: Any,
    {
        // NB: we default to a "safer" mode with `AccessKind::Owned`, where
        // references cannot be converted to an `Mut<T>` in order to avoid
        // a potential soundness panic.
        self.internal_downcast_into_ref(AccessKind::Owned)
    }

    /// Internal implementation of `downcast_into_ref`.
    pub(crate) fn internal_downcast_into_ref<T>(
        self,
        kind: AccessKind,
    ) -> Result<Ref<T>, AccessError>
    where
        T: Any,
    {
        unsafe {
            let (data, guard) = {
                let inner = self.inner.as_ref();
                let guard = inner.access.shared(kind)?;
                let expected = TypeId::of::<T>();

                match (*inner.data.get()).raw_as_ptr(expected) {
                    Ok(data) => (data, guard),
                    Err(AnyObjError::Cast) => {
                        return Err(AccessError::UnexpectedType {
                            expected: any::type_name::<T>().into(),
                            actual: (*inner.data.get()).type_name(),
                        });
                    }
                    Err(e) => {
                        return Err(e.into());
                    }
                }
            };

            let guard = guard.into_raw();
            // NB: we need to prevent the Drop impl for Shared from being called,
            // since we are deconstructing its internals.
            let this = ManuallyDrop::new(self);

            Ok(Ref {
                data: ptr::NonNull::new_unchecked(data as *const T as *mut T),
                guard: Some(guard),
                inner: RawDrop::decrement_shared_box(this.inner),
            })
        }
    }

    /// Get an exclusive value and downcast.
    pub fn downcast_into_mut<T>(self) -> Result<Mut<T>, AccessError>
    where
        T: Any,
    {
        // NB: we default to a "safer" mode with `AccessKind::Owned`, where
        // references cannot be converted to an `Mut<T>` in order to avoid
        // a potential soundness panic.
        self.internal_downcast_into_mut(AccessKind::Owned)
    }

    /// Internal implementation of `downcast_into_mut`.
    pub(crate) fn internal_downcast_into_mut<T>(
        self,
        kind: AccessKind,
    ) -> Result<Mut<T>, AccessError>
    where
        T: Any,
    {
        unsafe {
            let (data, guard) = {
                let inner = self.inner.as_ref();
                let guard = inner.access.exclusive(kind)?;
                let expected = TypeId::of::<T>();

                match (*inner.data.get()).raw_as_mut(expected) {
                    Ok(data) => (data, guard),
                    Err(AnyObjError::Cast) => {
                        return Err(AccessError::UnexpectedType {
                            expected: any::type_name::<T>().into(),
                            actual: (*inner.data.get()).type_name(),
                        });
                    }
                    Err(e) => {
                        return Err(e.into());
                    }
                }
            };

            let guard = guard.into_raw();
            // NB: we need to prevent the Drop impl for Shared from being called,
            // since we are deconstructing its internals.
            let this = ManuallyDrop::new(self);

            Ok(Mut {
                data: ptr::NonNull::new_unchecked(data as *mut T),
                guard: Some(guard),
                inner: RawDrop::decrement_shared_box(this.inner),
            })
        }
    }
}

impl<T: ?Sized> TryClone for Shared<T> {
    #[inline]
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(self.clone())
    }
}

impl<T: ?Sized> Clone for Shared<T> {
    fn clone(&self) -> Self {
        unsafe {
            SharedBox::inc(self.inner.as_ptr());
        }

        Self { inner: self.inner }
    }
}

impl<T: ?Sized> Drop for Shared<T> {
    fn drop(&mut self) {
        unsafe {
            SharedBox::dec(self.inner.as_ptr());
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

/// A debug helper that prints detailed diagnostics on the type being debugged.
///
/// Constructed using [debug][Shared::debug].
pub struct SharedDebug<'a, T: ?Sized> {
    shared: &'a Shared<T>,
}

impl<T: ?Sized> fmt::Debug for SharedDebug<'_, T>
where
    T: Any + fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Safety: by virtue of holding onto a shared we can safely access
        // `inner` because it must outlive any `Shared` instances.
        unsafe {
            let inner = self.shared.inner.as_ref();
            let mut debug = fmt.debug_struct("Shared");

            debug.field("access", &inner.access);
            debug.field("count", &inner.count.get());

            if !inner.access.is_shared() {
                debug.field("data", &any::type_name::<T>());
            } else {
                debug.field("data", &&*inner.data.get());
            }

            debug.finish()
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
    unsafe fn inc(this: *const Self) {
        let count = (*this).count.get();

        if count == 0 || count == usize::MAX {
            crate::alloc::abort();
        }

        (*this).count.set(count + 1);
    }

    /// Decrement the reference count in inner, and free the underlying data if
    /// it has reached zero.
    ///
    /// # Safety
    ///
    /// ProtocolCaller needs to ensure that `this` is a valid pointer.
    unsafe fn dec(this: *mut Self) -> bool {
        let count = (*this).count.get();

        if count == 0 {
            crate::alloc::abort();
        }

        let count = count - 1;
        (*this).count.set(count);

        if count != 0 {
            return false;
        }

        let this = Box::from_raw_in(this, rune_alloc::alloc::Global);

        if this.access.is_taken() {
            // NB: This prevents the inner `T` from being dropped in case it
            // has already been taken (as indicated by `is_taken`).
            //
            // If it has been taken, the shared box contains invalid memory.
            drop(transmute::<_, Box<SharedBox<ManuallyDrop<T>>>>(this));
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

type DropFn = unsafe fn(*const ());

struct RawDrop {
    data: *const (),
    drop_fn: DropFn,
}

impl RawDrop {
    /// Construct an empty raw drop function.
    const fn empty() -> Self {
        fn drop_fn(_: *const ()) {}

        Self {
            data: ptr::null(),
            drop_fn,
        }
    }

    /// Construct from an atomically reference-counted `Arc<T>`.
    ///
    /// The argument must have been produced using `Arc::into_raw`.
    #[cfg(feature = "alloc")]
    unsafe fn from_rc<T>(data: *const T) -> Self {
        unsafe fn drop_fn<T>(data: *const ()) {
            let _ = Rc::from_raw(data as *const T);
        }

        Self {
            data: data as *const (),
            drop_fn: drop_fn::<T>,
        }
    }

    /// Construct from an atomically reference-counted `Arc<T>`.
    ///
    /// The argument must have been produced using `Arc::into_raw`.
    #[cfg(feature = "alloc")]
    unsafe fn from_arc<T>(data: *const T) -> Self {
        unsafe fn drop_fn<T>(data: *const ()) {
            let _ = Arc::from_raw(data as *const T);
        }

        Self {
            data: data as *const (),
            drop_fn: drop_fn::<T>,
        }
    }

    /// Construct a raw drop that will decrement the shared box when dropped.
    ///
    /// # Safety
    ///
    /// Should only be constructed over a pointer that is lively owned.
    fn decrement_shared_box<T>(inner: ptr::NonNull<SharedBox<T>>) -> Self {
        unsafe fn drop_fn_impl<T>(data: *const ()) {
            let shared = data as *mut () as *mut SharedBox<T>;
            SharedBox::dec(shared);
        }

        Self {
            data: inner.as_ptr() as *const (),
            drop_fn: drop_fn_impl::<T>,
        }
    }

    /// Construct a raw drop that will take the shared box as it's being
    /// dropped.
    ///
    /// # Safety
    ///
    /// Should only be constructed over a pointer that is lively owned.
    fn take_shared_box(inner: ptr::NonNull<SharedBox<AnyObj>>) -> Self {
        unsafe fn drop_fn_impl(data: *const ()) {
            let shared = data as *mut () as *mut SharedBox<AnyObj>;

            // Mark the shared box for exclusive access.
            let _ = ManuallyDrop::new(
                (*shared)
                    .access
                    .take(AccessKind::Any)
                    .expect("raw pointers must not be shared"),
            );

            // Free the inner `Any` structure, and since we have marked the
            // Shared as taken, this will prevent anyone else from doing it.
            drop(ptr::read((*shared).data.get()));

            SharedBox::dec(shared);
        }

        Self {
            data: inner.as_ptr() as *const (),
            drop_fn: drop_fn_impl,
        }
    }
}

impl Drop for RawDrop {
    fn drop(&mut self) {
        // Safety: type and referential safety is guaranteed at construction
        // time, since all constructors are unsafe.
        unsafe {
            (self.drop_fn)(self.data);
        }
    }
}

/// A strong reference to the given type.
pub struct Ref<T: ?Sized> {
    data: ptr::NonNull<T>,
    // Safety: it is important that the guard is dropped before `RawDrop`, since
    // `RawDrop` might deallocate the `Access` instance the guard is referring
    // to. This is guaranteed by: https://github.com/rust-lang/rfcs/pull/1857
    guard: Option<RawAccessGuard>,
    // We need to keep track of the original value so that we can deal with what
    // it means to drop a reference to it.
    inner: RawDrop,
}

#[cfg(feature = "alloc")]
impl<T> From<Rc<T>> for Ref<T> {
    /// Construct from an atomically reference-counted value.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::rc::Rc;
    /// use rune::runtime::Ref;
    ///
    /// let value: Ref<String> = Ref::from(Rc::new(String::from("hello world")));
    /// assert_eq!(value.as_ref(), "hello world");
    /// ```
    fn from(value: Rc<T>) -> Ref<T> {
        let data = Rc::into_raw(value);

        Ref {
            data: unsafe { ptr::NonNull::new_unchecked(data as *mut _) },
            guard: None,
            inner: unsafe { RawDrop::from_rc(data) },
        }
    }
}

#[cfg(feature = "alloc")]
impl<T> From<Arc<T>> for Ref<T> {
    /// Construct from an atomically reference-counted value.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Arc;
    /// use rune::runtime::Ref;
    ///
    /// let value: Ref<String> = Ref::from(Arc::new(String::from("hello world")));
    /// assert_eq!(value.as_ref(), "hello world");
    /// ```
    fn from(value: Arc<T>) -> Ref<T> {
        let data = Arc::into_raw(value);

        Ref {
            data: unsafe { ptr::NonNull::new_unchecked(data as *mut _) },
            guard: None,
            inner: unsafe { RawDrop::from_arc(data) },
        }
    }
}

impl<T: ?Sized> Ref<T> {
    /// Construct a static reference.
    pub const fn from_static(value: &'static T) -> Ref<T> {
        Ref {
            data: unsafe { ptr::NonNull::new_unchecked(value as *const _ as *mut _) },
            guard: None,
            inner: RawDrop::empty(),
        }
    }

    /// Map the interior reference of an owned mutable value.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::{Shared, Ref};
    ///
    /// let vec = Shared::<Vec<u32>>::new(vec![1, 2, 3, 4])?;
    /// let vec = vec.into_ref()?;
    /// let value: Ref<[u32]> = Ref::map(vec, |vec| &vec[0..2]);
    ///
    /// assert_eq!(&*value, &[1u32, 2u32][..]);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn map<U: ?Sized, F>(this: Self, f: F) -> Ref<U>
    where
        F: FnOnce(&T) -> &U,
    {
        let Self {
            data, guard, inner, ..
        } = this;

        // Safety: this follows the same safety guarantees as when the managed
        // ref was acquired. And since we have a managed reference to `T`, we're
        // permitted to do any sort of projection to `U`.
        let data = f(unsafe { data.as_ref() });

        Ref {
            data: data.into(),
            guard,
            inner,
        }
    }

    /// Try to map the reference to a projection.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::{Shared, Ref};
    ///
    /// let vec = Shared::<Vec<u32>>::new(vec![1, 2, 3, 4])?;
    /// let vec = vec.into_ref()?;
    /// let value: Option<Ref<[u32]>> = Ref::try_map(vec, |vec| vec.get(0..2));
    ///
    /// assert_eq!(value.as_deref(), Some(&[1u32, 2u32][..]));
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn try_map<U: ?Sized, F>(this: Self, f: F) -> Option<Ref<U>>
    where
        F: FnOnce(&T) -> Option<&U>,
    {
        let Self {
            data, guard, inner, ..
        } = this;

        // Safety: this follows the same safety guarantees as when the managed
        // ref was acquired. And since we have a managed reference to `T`, we're
        // permitted to do any sort of projection to `U`.
        f(unsafe { data.as_ref() }).map(|data| Ref {
            data: data.into(),
            guard,
            inner,
        })
    }

    /// Convert into a raw pointer and associated raw access guard.
    ///
    /// # Safety
    ///
    /// The returned pointer must not outlive the associated guard, since this
    /// prevents other uses of the underlying data which is incompatible with
    /// the current.
    pub fn into_raw(this: Self) -> (ptr::NonNull<T>, RawRef) {
        let guard = RawRef {
            _guard: this.guard,
            _inner: this.inner,
        };

        (this.data, guard)
    }
}

impl<T: ?Sized> AsRef<T> for Ref<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T: ?Sized> ops::Deref for Ref<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        // Safety: An owned ref holds onto a hard pointer to the data,
        // preventing it from being dropped for the duration of the owned ref.
        unsafe { self.data.as_ref() }
    }
}

impl<T: ?Sized> fmt::Debug for Ref<T>
where
    T: fmt::Debug,
{
    #[inline]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, fmt)
    }
}

/// A raw guard to a [Ref].
pub struct RawRef {
    _guard: Option<RawAccessGuard>,
    _inner: RawDrop,
}

/// A strong mutable reference to the given type.
pub struct Mut<T: ?Sized> {
    data: ptr::NonNull<T>,
    // Safety: it is important that the guard is dropped before `RawDrop`, since
    // `RawDrop` might deallocate the `Access` instance the guard is referring
    // to. This is guaranteed by: https://github.com/rust-lang/rfcs/pull/1857
    guard: Option<RawAccessGuard>,
    inner: RawDrop,
}

impl<T: ?Sized> Mut<T> {
    /// Construct a static mutable reference.
    pub fn from_static(value: &'static mut T) -> Mut<T> {
        Mut {
            data: unsafe { ptr::NonNull::new_unchecked(value as *mut _) },
            guard: None,
            inner: RawDrop::empty(),
        }
    }

    /// Map the interior reference of an owned mutable value.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::{Mut, Shared};
    ///
    /// let vec = Shared::<Vec<u32>>::new(vec![1, 2, 3, 4])?;
    /// let vec = vec.into_mut()?;
    /// let value: Mut<[u32]> = Mut::map(vec, |vec| &mut vec[0..2]);
    ///
    /// assert_eq!(&*value, &mut [1u32, 2u32][..]);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn map<U: ?Sized, F>(this: Self, f: F) -> Mut<U>
    where
        F: FnOnce(&mut T) -> &mut U,
    {
        let Self {
            mut data,
            guard,
            inner,
            ..
        } = this;

        // Safety: this follows the same safety guarantees as when the managed
        // ref was acquired. And since we have a managed reference to `T`, we're
        // permitted to do any sort of projection to `U`.
        let data = f(unsafe { data.as_mut() });

        Mut {
            data: data.into(),
            guard,
            inner,
        }
    }

    /// Try to map the mutable reference to a projection.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::{Mut, Shared};
    ///
    /// let vec = Shared::<Vec<u32>>::new(vec![1, 2, 3, 4])?;
    /// let vec = vec.into_mut()?;
    /// let mut value: Option<Mut<[u32]>> = Mut::try_map(vec, |vec| vec.get_mut(0..2));
    ///
    /// assert_eq!(value.as_deref_mut(), Some(&mut [1u32, 2u32][..]));
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn try_map<U: ?Sized, F>(this: Self, f: F) -> Option<Mut<U>>
    where
        F: FnOnce(&mut T) -> Option<&mut U>,
    {
        let Self {
            mut data,
            guard,
            inner,
            ..
        } = this;

        // Safety: this follows the same safety guarantees as when the managed
        // ref was acquired. And since we have a managed reference to `T`, we're
        // permitted to do any sort of projection to `U`.
        f(unsafe { data.as_mut() }).map(|data| Mut {
            data: data.into(),
            guard,
            inner,
        })
    }

    /// Convert into a raw pointer and associated raw access guard.
    ///
    /// # Safety
    ///
    /// The returned pointer must not outlive the associated guard, since this
    /// prevents other uses of the underlying data which is incompatible with
    /// the current.
    pub fn into_raw(this: Self) -> (ptr::NonNull<T>, RawMut) {
        let guard = RawMut {
            _guard: this.guard,
            _inner: this.inner,
        };

        (this.data, guard)
    }
}

impl<T: ?Sized> AsRef<T> for Mut<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T: ?Sized> AsMut<T> for Mut<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        self
    }
}

impl<T: ?Sized> ops::Deref for Mut<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Safety: An owned mut holds onto a hard pointer to the data,
        // preventing it from being dropped for the duration of the owned mut.
        unsafe { self.data.as_ref() }
    }
}

impl<T: ?Sized> ops::DerefMut for Mut<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Safety: An owned mut holds onto a hard pointer to the data,
        // preventing it from being dropped for the duration of the owned mut.
        unsafe { self.data.as_mut() }
    }
}

impl<T: ?Sized> fmt::Debug for Mut<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, fmt)
    }
}

impl<F> Future for Mut<F>
where
    F: Unpin + Future,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // NB: inner Future is Unpin.
        let this = self.get_mut();
        Pin::new(&mut **this).poll(cx)
    }
}

/// A raw guard to a [Ref].
pub struct RawMut {
    _guard: Option<RawAccessGuard>,
    _inner: RawDrop,
}

/// A guard for an `Any` containing a pointer.
///
/// Constructing using [Shared::from_ref] or [Shared::from_mut].
pub struct SharedPointerGuard {
    _inner: RawDrop,
}
