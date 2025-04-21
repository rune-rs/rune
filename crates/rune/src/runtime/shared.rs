use core::fmt;
use core::marker::PhantomData;
use core::mem::ManuallyDrop;
use core::ptr::{addr_of, NonNull};

use crate::{Any, Hash};

use super::any_obj::{AnyObjData, AnyObjError, AnyObjErrorKind, Kind, Vtable};
use super::{AccessError, BorrowMut, BorrowRef, Mut, RawAnyGuard, Ref, RefVtable, TypeInfo};

/// A typed wrapper for a reference.
///
/// This is identical in layout to [`AnyObj`], but provides a statically
/// type-checked value.
///
/// [`AnyObj`]: super::AnyObj
pub struct Shared<T> {
    /// The shared value.
    shared: NonNull<AnyObjData>,
    /// The statically known type of the value.
    _marker: PhantomData<T>,
}

impl<T> Shared<T>
where
    T: Any,
{
    /// Construct a new typed object.
    ///
    /// # Safety
    ///
    /// Caller must ensure that the type is of the value `T`.
    pub(super) unsafe fn new(shared: NonNull<AnyObjData<T>>) -> Self {
        Self {
            shared: shared.cast(),
            _marker: PhantomData,
        }
    }

    /// Take the owned value of type `T`.
    ///
    /// This consumes any live references of the value and accessing them in the
    /// future will result in an error.
    ///
    /// # Errors
    ///
    /// This errors if the underlying value is not owned.
    pub fn take(self) -> Result<T, AnyObjError> {
        let vtable = vtable(&self);

        if !matches!(vtable.kind, Kind::Own) {
            return Err(AnyObjError::from(AccessError::not_owned(
                vtable.type_info(),
            )));
        }

        // SAFETY: We've checked for the appropriate type just above.
        unsafe {
            self.shared.as_ref().access.try_take()?;
            let data = vtable.as_ptr::<T>(self.shared);
            Ok(data.read())
        }
    }

    /// Drop the value.
    ///
    /// This consumes any live references of the value and accessing them in the
    /// future will result in an error.
    pub fn drop(self) -> Result<(), AccessError> {
        let vtable = vtable(&self);

        if !matches!(vtable.kind, Kind::Own) {
            return Err(AccessError::not_owned(vtable.type_info()));
        }

        // SAFETY: We've checked for the appropriate type just above.
        unsafe {
            self.shared.as_ref().access.try_take()?;

            if let Some(drop_value) = vtable.drop_value {
                drop_value(self.shared);
            }

            Ok(())
        }
    }

    /// Downcast into an owned value of type [`Ref<T>`].
    ///
    /// # Errors
    ///
    /// This errors in case the underlying value is not owned, non-owned
    /// references cannot be coerced into [`Ref<T>`].
    pub fn into_ref(self) -> Result<Ref<T>, AnyObjError> {
        let vtable = vtable(&self);

        if !matches!(vtable.kind, Kind::Own) {
            return Err(AnyObjError::from(AccessError::not_owned(
                vtable.type_info(),
            )));
        }

        // SAFETY: We've checked for the appropriate type just above.
        unsafe {
            self.shared.as_ref().access.try_shared()?;
            let this = ManuallyDrop::new(self);
            let data = vtable.as_ptr(this.shared);

            let vtable = &RefVtable {
                drop: |shared: NonNull<()>| {
                    let shared = shared.cast::<AnyObjData>();
                    shared.as_ref().access.release();
                    AnyObjData::dec(shared)
                },
            };

            let guard = RawAnyGuard::new(this.shared.cast(), vtable);
            Ok(Ref::new(data, guard))
        }
    }

    /// Downcast into an owned value of type [`Mut<T>`].
    ///
    /// # Errors
    ///
    /// This errors in case the underlying value is not owned, non-owned
    /// references cannot be coerced into [`Mut<T>`].
    pub fn into_mut(self) -> Result<Mut<T>, AnyObjError> {
        let vtable = vtable(&self);

        if !matches!(vtable.kind, Kind::Own) {
            return Err(AnyObjError::from(AccessError::not_owned(
                vtable.type_info(),
            )));
        }

        // SAFETY: We've checked for the appropriate type just above.
        unsafe {
            self.shared.as_ref().access.try_exclusive()?;
            let this = ManuallyDrop::new(self);
            let data = vtable.as_ptr(this.shared);

            let vtable = &RefVtable {
                drop: |shared: NonNull<()>| {
                    let shared = shared.cast::<AnyObjData>();
                    shared.as_ref().access.release();
                    AnyObjData::dec(shared)
                },
            };

            let guard = RawAnyGuard::new(this.shared.cast(), vtable);
            Ok(Mut::new(data, guard))
        }
    }

    /// Borrow a shared reference to the value while checking for shared access.
    ///
    /// This prevents other exclusive accesses from being performed while the
    /// guard returned from this function is live.
    pub fn borrow_ref(&self) -> Result<BorrowRef<'_, T>, AnyObjError> {
        let vtable = vtable(self);

        // SAFETY: We've checked for the appropriate type just above.
        unsafe {
            let guard = self.shared.as_ref().access.shared()?;
            let data = vtable.as_ptr(self.shared);
            Ok(BorrowRef::new(data, guard.into_raw()))
        }
    }

    /// Try to borrow an shared reference to the value.
    ///
    /// Returns `None` if the value is not `T`.
    ///
    /// This prevents other exclusive accesses from being performed while the
    /// guard returned from this function is live.
    pub fn try_borrow_ref(&self) -> Result<Option<BorrowRef<'_, T>>, AccessError> {
        let vtable = vtable(self);

        // SAFETY: We've checked for the appropriate type just above.
        unsafe {
            let guard = self.shared.as_ref().access.shared()?;
            let data = vtable.as_ptr(self.shared);
            Ok(Some(BorrowRef::new(data, guard.into_raw())))
        }
    }

    /// Borrow an exclusive reference to the value.
    ///
    /// This prevents other accesses from being performed while the guard
    /// returned from this function is live.
    pub fn borrow_mut(&self) -> Result<BorrowMut<'_, T>, AnyObjError> {
        let vtable = vtable(self);

        if matches!(vtable.kind, Kind::Ref) {
            return Err(AnyObjError::new(AnyObjErrorKind::Cast(
                T::ANY_TYPE_INFO,
                vtable.type_info(),
            )));
        }

        // SAFETY: We've checked for the appropriate type just above.
        unsafe {
            let guard = self.shared.as_ref().access.exclusive()?;
            let data = vtable.as_ptr(self.shared);
            Ok(BorrowMut::new(data, guard.into_raw()))
        }
    }

    /// Try to borrow an exlucisve reference to the value.
    ///
    /// Returns `None` if the value is not `T`.
    ///
    /// This prevents other exclusive accesses from being performed while the
    /// guard returned from this function is live.
    pub fn try_borrow_mut(&self) -> Result<Option<BorrowMut<'_, T>>, AccessError> {
        let vtable = vtable(self);

        // SAFETY: We've checked for the appropriate type just above.
        unsafe {
            let guard = self.shared.as_ref().access.exclusive()?;
            let data = vtable.as_ptr(self.shared);
            Ok(Some(BorrowMut::new(data, guard.into_raw())))
        }
    }

    /// Test if the value is sharable.
    pub fn is_readable(&self) -> bool {
        // Safety: Since we have a reference to this shared, we know that the
        // inner is available.
        unsafe { self.shared.as_ref().access.is_shared() }
    }

    /// Test if the value is exclusively accessible.
    pub fn is_writable(&self) -> bool {
        unsafe {
            let shared = self.shared.as_ref();
            !matches!(shared.vtable.kind, Kind::Ref) && shared.access.is_exclusive()
        }
    }

    /// Debug format the current any type.
    pub(crate) fn debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (vtable(self).debug)(f)
    }

    /// Access the underlying type id for the data.
    pub fn type_hash(&self) -> Hash {
        vtable(self).type_hash
    }

    /// Access full type info for the underlying type.
    pub fn type_info(&self) -> TypeInfo {
        TypeInfo::any_type_info(vtable(self).type_info)
    }
}

impl<T> fmt::Debug for Shared<T>
where
    T: Any,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug(f)
    }
}

impl<T> Drop for Shared<T> {
    fn drop(&mut self) {
        // Safety: We know that the inner value is live in this instance.
        unsafe {
            AnyObjData::dec(self.shared);
        }
    }
}

#[inline]
pub(super) fn vtable<T>(any: &Shared<T>) -> &'static Vtable {
    unsafe { addr_of!((*any.shared.as_ptr()).vtable).read() }
}
