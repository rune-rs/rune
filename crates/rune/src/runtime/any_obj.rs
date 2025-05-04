use core::cell::Cell;
use core::fmt;
use core::mem::{replace, ManuallyDrop};
use core::ptr::{self, addr_of, NonNull};

use crate::alloc::clone::TryClone;
use crate::alloc::{self, Box};
use crate::{Any, Hash};

use super::{
    Access, AccessError, AnyObjVtable, AnyTypeInfo, BorrowMut, BorrowRef, FromValue, Mut,
    RawAccessGuard, RawAnyGuard, Ref, RefVtable, RuntimeError, Shared, Snapshot, ToValue, TypeInfo,
    Value,
};

/// A type-erased wrapper for a reference.
pub struct AnyObj {
    shared: NonNull<AnyObjData>,
}

impl AnyObj {
    /// Construct a new typed object.
    ///
    /// # Safety
    ///
    /// Caller must ensure that the type is of the value `T`.
    #[inline]
    pub(super) const unsafe fn from_raw(shared: NonNull<AnyObjData>) -> Self {
        Self { shared }
    }

    /// Construct an Any that wraps an owned object.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Value;
    /// use rune::runtime::AnyObj;
    /// use rune::alloc::String;
    ///
    /// let string = String::try_from("Hello World")?;
    /// let string = AnyObj::new(string)?;
    /// let string = Value::from(string);
    ///
    /// let string = string.into_shared::<String>()?;
    /// assert_eq!(string.borrow_ref()?.as_str(), "Hello World");
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn new<T>(data: T) -> alloc::Result<Self>
    where
        T: Any,
    {
        let shared = AnyObjData {
            access: Access::new(),
            count: Cell::new(1),
            vtable: AnyObjVtable::owned::<T>(),
            data,
        };

        let shared = NonNull::from(Box::leak(Box::try_new(shared)?)).cast();
        Ok(Self { shared })
    }

    /// Construct an Any that wraps a pointer.
    ///
    /// # Safety
    ///
    /// Caller must ensure that the returned `AnyObj` doesn't outlive the
    /// reference it is wrapping.
    #[inline]
    pub(crate) unsafe fn from_ref<T>(data: *const T) -> alloc::Result<Self>
    where
        T: Any,
    {
        let shared = AnyObjData {
            access: Access::new(),
            count: Cell::new(1),
            vtable: AnyObjVtable::from_ref::<T>(),
            data: NonNull::new_unchecked(data.cast_mut()),
        };

        let shared = NonNull::from(Box::leak(Box::try_new(shared)?)).cast();
        Ok(Self { shared })
    }

    /// Construct an Any that wraps a mutable pointer.
    ///
    /// # Safety
    ///
    /// Caller must ensure that the returned `AnyObj` doesn't outlive the
    /// reference it is wrapping.
    #[inline]
    pub(crate) unsafe fn from_mut<T>(data: *mut T) -> alloc::Result<Self>
    where
        T: Any,
    {
        let shared = AnyObjData {
            access: Access::new(),
            count: Cell::new(1),
            vtable: AnyObjVtable::from_mut::<T>(),
            data: NonNull::new_unchecked(data),
        };

        let shared = NonNull::from(Box::leak(Box::try_new(shared)?)).cast();
        Ok(Self { shared })
    }

    /// Coerce into a typed object.
    pub(crate) fn into_shared<T>(self) -> Result<Shared<T>, AnyObjError>
    where
        T: Any,
    {
        let vtable = vtable(&self);

        if !vtable.is::<T>() {
            return Err(AnyObjError::new(AnyObjErrorKind::Cast(
                T::ANY_TYPE_INFO,
                vtable.type_info(),
            )));
        }

        // SAFETY: We've typed checked for the appropriate type just above.
        unsafe { Ok(self.unsafe_into_shared()) }
    }

    /// Coerce into a typed object.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the type being convert into is correct.
    #[inline]
    pub(crate) unsafe fn unsafe_into_shared<T>(self) -> Shared<T>
    where
        T: Any,
    {
        let this = ManuallyDrop::new(self);
        Shared::from_raw(this.shared.cast())
    }

    /// Downcast into an owned value of type `T`.
    pub(crate) fn downcast<T>(self) -> Result<T, AnyObjError>
    where
        T: Any,
    {
        let vtable = vtable(&self);

        if !vtable.is::<T>() {
            return Err(AnyObjError::new(AnyObjErrorKind::Cast(
                T::ANY_TYPE_INFO,
                vtable.type_info(),
            )));
        }

        if !vtable.is_owned() {
            return Err(AnyObjError::new(AnyObjErrorKind::NotOwned(
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

    /// Take the interior value and drop it if necessary.
    pub(crate) fn drop(self) -> Result<(), AnyObjError> {
        let vtable = vtable(&self);

        if !vtable.is_owned() {
            return Err(AnyObjError::new(AnyObjErrorKind::NotOwned(
                vtable.type_info(),
            )));
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

    /// Take the interior value and return a handle to the taken value.
    pub fn take(self) -> Result<Self, AnyObjError> {
        let vtable = vtable(&self);

        // SAFETY: We've checked for the appropriate type just above.
        unsafe {
            self.shared.as_ref().access.try_take()?;
            Ok((vtable.clone)(self.shared)?)
        }
    }

    /// Downcast into an owned value of type [`Ref<T>`].
    ///
    /// # Errors
    ///
    /// This errors in case the underlying value is not owned, non-owned
    /// references cannot be coerced into [`Ref<T>`].
    pub(crate) fn into_ref<T>(self) -> Result<Ref<T>, AnyObjError>
    where
        T: Any,
    {
        let vtable = vtable(&self);

        if !vtable.is::<T>() {
            return Err(AnyObjError::new(AnyObjErrorKind::Cast(
                T::ANY_TYPE_INFO,
                vtable.type_info(),
            )));
        }

        if !vtable.is_owned() {
            return Err(AnyObjError::new(AnyObjErrorKind::NotOwned(
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
    pub(crate) fn into_mut<T>(self) -> Result<Mut<T>, AnyObjError>
    where
        T: Any,
    {
        let vtable = vtable(&self);

        if !vtable.is::<T>() {
            return Err(AnyObjError::new(AnyObjErrorKind::Cast(
                T::ANY_TYPE_INFO,
                vtable.type_info(),
            )));
        }

        if !vtable.is_owned() {
            return Err(AnyObjError::new(AnyObjErrorKind::NotOwned(
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

    /// Get a reference to the interior value while checking for shared access.
    ///
    /// This prevents other exclusive accesses from being performed while the
    /// guard returned from this function is live.
    pub fn borrow_ref<T>(&self) -> Result<BorrowRef<'_, T>, AnyObjError>
    where
        T: Any,
    {
        let vtable = vtable(self);

        if !vtable.is::<T>() {
            return Err(AnyObjError::new(AnyObjErrorKind::Cast(
                T::ANY_TYPE_INFO,
                vtable.type_info(),
            )));
        }

        // SAFETY: We've checked for the appropriate type just above.
        unsafe {
            let guard = self.shared.as_ref().access.shared()?;
            let data = vtable.as_ptr(self.shared);
            Ok(BorrowRef::new(data, guard.into_raw()))
        }
    }

    /// Try to borrow a reference to the interior value while checking for
    /// shared access.
    ///
    /// Returns `None` if the interior type is not `T`.
    ///
    /// This prevents other exclusive accesses from being performed while the
    /// guard returned from this function is alive.
    pub fn try_borrow_ref<T>(&self) -> Result<Option<BorrowRef<'_, T>>, AccessError>
    where
        T: Any,
    {
        let vtable = vtable(self);

        if !vtable.is::<T>() {
            return Ok(None);
        }

        // SAFETY: We've checked for the appropriate type just above.
        unsafe {
            let guard = self.shared.as_ref().access.shared()?;
            let data = vtable.as_ptr(self.shared);
            Ok(Some(BorrowRef::new(data, guard.into_raw())))
        }
    }

    /// Try to borrow a reference to the interior value while checking for
    /// exclusive access.
    ///
    /// Returns `None` if the interior type is not `T`.
    ///
    /// This prevents other exclusive accesses from being performed while the
    /// guard returned from this function is alive.
    pub fn try_borrow_mut<T>(&self) -> Result<Option<BorrowMut<'_, T>>, AccessError>
    where
        T: Any,
    {
        let vtable = vtable(self);

        if !vtable.is::<T>() {
            return Ok(None);
        }

        // SAFETY: We've checked for the appropriate type just above.
        unsafe {
            let guard = self.shared.as_ref().access.exclusive()?;
            let data = vtable.as_ptr(self.shared);
            Ok(Some(BorrowMut::new(data, guard.into_raw())))
        }
    }

    /// Returns some mutable reference to the boxed value if it is of type `T`.
    pub fn borrow_mut<T>(&self) -> Result<BorrowMut<'_, T>, AnyObjError>
    where
        T: Any,
    {
        let vtable = vtable(self);

        if !vtable.is::<T>() || !vtable.is_mutable() {
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

    /// Get a reference to the interior value while checking for shared access.
    ///
    /// This prevents other exclusive accesses from being performed while the
    /// guard returned from this function is live.
    pub(crate) fn borrow_ref_ptr<T>(self) -> Result<(NonNull<T>, RawAnyObjGuard), AnyObjError>
    where
        T: Any,
    {
        let vtable = vtable(&self);

        if !vtable.is::<T>() {
            return Err(AnyObjError::new(AnyObjErrorKind::Cast(
                T::ANY_TYPE_INFO,
                vtable.type_info(),
            )));
        }

        // SAFETY: We've checked for the appropriate type just above.
        unsafe {
            let guard = self.shared.as_ref().access.shared()?.into_raw();
            let this = ManuallyDrop::new(self);

            let data = vtable.as_ptr(this.shared);

            let guard = RawAnyObjGuard {
                guard,
                dec_shared: AnyObjDecShared {
                    shared: this.shared,
                },
            };

            Ok((data, guard))
        }
    }

    /// Returns some mutable reference to the boxed value if it is of type `T`.
    pub(crate) fn borrow_mut_ptr<T>(self) -> Result<(NonNull<T>, RawAnyObjGuard), AnyObjError>
    where
        T: Any,
    {
        let vtable = vtable(&self);

        if !vtable.is::<T>() || !vtable.is_mutable() {
            return Err(AnyObjError::new(AnyObjErrorKind::Cast(
                T::ANY_TYPE_INFO,
                vtable.type_info(),
            )));
        }

        // SAFETY: We've checked for the appropriate type just above.
        unsafe {
            let guard = self.shared.as_ref().access.exclusive()?.into_raw();
            let this = ManuallyDrop::new(self);

            let data = vtable.as_ptr(this.shared);

            let guard = RawAnyObjGuard {
                guard,
                dec_shared: AnyObjDecShared {
                    shared: this.shared,
                },
            };

            Ok((data, guard))
        }
    }

    /// Deconstruct the shared value into a guard and shared box.
    ///
    /// # Safety
    ///
    /// The content of the shared value will be forcibly destructed once the
    /// returned guard is dropped, unchecked use of the shared value after this
    /// point will lead to undefined behavior.
    pub(crate) unsafe fn into_drop_guard(self) -> (Self, AnyObjDrop) {
        // Increment the reference count by one to account for the guard holding
        // onto it.
        AnyObjData::inc(self.shared);

        let guard = AnyObjDrop {
            shared: self.shared,
        };

        (self, guard)
    }

    /// Test if the value is sharable.
    pub(crate) fn is_readable(&self) -> bool {
        // Safety: Since we have a reference to this shared, we know that the
        // inner is available.
        unsafe { self.shared.as_ref().access.is_shared() }
    }

    /// Test if the value is exclusively accessible.
    pub(crate) fn is_writable(&self) -> bool {
        unsafe {
            let shared = self.shared.as_ref();
            shared.vtable.is_mutable() && shared.access.is_exclusive()
        }
    }

    /// Get access snapshot of shared value.
    pub(crate) fn snapshot(&self) -> Snapshot {
        unsafe { self.shared.as_ref().access.snapshot() }
    }

    /// Debug format the current any type.
    pub(crate) fn debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        vtable(self).debug(f)
    }

    /// Access the underlying type id for the data.
    pub(crate) fn type_hash(&self) -> Hash {
        vtable(self).type_hash()
    }

    /// Access full type info for the underlying type.
    pub fn type_info(&self) -> TypeInfo {
        vtable(self).type_info()
    }
}

impl Clone for AnyObj {
    #[inline]
    fn clone(&self) -> Self {
        // SAFETY: We know that the inner value is live in this instance.
        unsafe {
            AnyObjData::inc(self.shared);
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
            AnyObjData::dec(old);
            AnyObjData::inc(self.shared);
        }
    }
}

impl TryClone for AnyObj {
    #[inline]
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(self.clone())
    }

    #[inline]
    fn try_clone_from(&mut self, source: &Self) -> alloc::Result<()> {
        self.clone_from(source);
        Ok(())
    }
}

impl fmt::Debug for AnyObj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug(f)
    }
}

impl Drop for AnyObj {
    fn drop(&mut self) {
        // Safety: We know that the inner value is live in this instance.
        unsafe {
            AnyObjData::dec(self.shared);
        }
    }
}

impl FromValue for AnyObj {
    #[inline]
    fn from_value(value: Value) -> Result<Self, RuntimeError> {
        value.into_any_obj()
    }
}

impl ToValue for AnyObj {
    #[inline]
    fn to_value(self) -> Result<Value, RuntimeError> {
        Ok(Value::from(self))
    }
}

#[repr(C)]
pub(super) struct AnyObjData<T = ()> {
    /// The currently handed out access to the shared data.
    pub(super) access: Access,
    /// The number of strong references to the shared data.
    pub(super) count: Cell<usize>,
    /// Vtable of the shared value.
    pub(super) vtable: &'static AnyObjVtable,
    /// Data of the shared reference.
    pub(super) data: T,
}

impl AnyObjData {
    /// Increment the reference count of the inner value.
    #[inline]
    pub(super) unsafe fn inc(this: NonNull<Self>) {
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
    #[inline]
    pub(super) unsafe fn dec(this: NonNull<Self>) {
        let count_ref = &*addr_of!((*this.as_ptr()).count);
        let count = count_ref.get();

        debug_assert_ne!(
            count, 0,
            "Reference count of zero should only happen if Shared is incorrectly implemented"
        );

        let count = count - 1;
        count_ref.set(count);

        if count == 0 {
            let vtable = *addr_of!((*this.as_ptr()).vtable);

            if let Some(drop_value) = vtable.drop_value {
                let access = &*addr_of!((*this.as_ptr()).access);

                if !access.is_taken() {
                    drop_value(this);
                }
            }

            (vtable.drop)(this);
        }
    }
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub(super) enum AnyObjErrorKind {
    Alloc(alloc::Error),
    Cast(AnyTypeInfo, TypeInfo),
    AccessError(AccessError),
    NotOwned(TypeInfo),
}

/// Errors caused when accessing or coercing an [`AnyObj`].
#[cfg_attr(test, derive(PartialEq))]
pub struct AnyObjError {
    kind: AnyObjErrorKind,
}

impl AnyObjError {
    #[inline]
    pub(super) fn new(kind: AnyObjErrorKind) -> Self {
        Self { kind }
    }

    #[inline]
    pub(super) fn into_kind(self) -> AnyObjErrorKind {
        self.kind
    }
}

impl core::error::Error for AnyObjError {}

impl fmt::Display for AnyObjError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            AnyObjErrorKind::Alloc(error) => error.fmt(f),
            AnyObjErrorKind::Cast(expected, actual) => {
                write!(f, "Failed to cast `{actual}` to `{expected}`")
            }
            AnyObjErrorKind::AccessError(error) => error.fmt(f),
            AnyObjErrorKind::NotOwned(type_info) => {
                write!(f, "Cannot use owned operations for {type_info}")
            }
        }
    }
}

impl fmt::Debug for AnyObjError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl From<alloc::Error> for AnyObjError {
    #[inline]
    fn from(error: alloc::Error) -> Self {
        Self::new(AnyObjErrorKind::Alloc(error))
    }
}

impl From<AccessError> for AnyObjError {
    #[inline]
    fn from(error: AccessError) -> Self {
        Self::new(AnyObjErrorKind::AccessError(error))
    }
}

/// Guard which decrements and releases shared storage for the guarded reference.
pub(super) struct AnyObjDecShared {
    pub(super) shared: NonNull<AnyObjData>,
}

impl Drop for AnyObjDecShared {
    fn drop(&mut self) {
        // Safety: We know that the inner value is live in this instance.
        unsafe {
            AnyObjData::dec(self.shared);
        }
    }
}

/// Guard which decrements and releases shared storage for the guarded reference.
pub(crate) struct AnyObjDrop {
    #[allow(unused)]
    pub(super) shared: NonNull<AnyObjData>,
}

impl Drop for AnyObjDrop {
    #[inline]
    fn drop(&mut self) {
        // Safety: We know that the inner value is live in this instance.
        unsafe {
            self.shared.as_ref().access.take();
            AnyObjData::dec(self.shared);
        }
    }
}

/// The guard returned when dealing with raw pointers.
pub(crate) struct RawAnyObjGuard {
    #[allow(unused)]
    pub(super) guard: RawAccessGuard,
    #[allow(unused)]
    pub(super) dec_shared: AnyObjDecShared,
}

#[inline]
fn vtable(any: &AnyObj) -> &'static AnyObjVtable {
    unsafe { addr_of!((*any.shared.as_ptr()).vtable).read() }
}
