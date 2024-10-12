use core::any::TypeId;
use core::cell::Cell;
use core::fmt;
use core::mem::{needs_drop, offset_of, replace, ManuallyDrop};
use core::ptr::{self, addr_of, addr_of_mut, drop_in_place, NonNull};

use crate::alloc::alloc::Global;
use crate::alloc::{self, Box};
use crate::{Any, Hash};

use super::{
    Access, AccessError, AnyTypeInfo, BorrowMut, BorrowRef, Mut, RawAccessGuard, RawAnyGuard,
    RawStr, Ref, RefVtable, Snapshot, TypeInfo, VmErrorKind,
};

/// Errors caused by casting an any reference.
#[cfg_attr(test, derive(Debug, PartialEq))]
pub(crate) enum AnyObjError {
    Cast(TypeInfo),
    AccessError(AccessError),
}

impl<T> From<T> for AnyObjError
where
    AccessError: From<T>,
{
    fn from(value: T) -> Self {
        AnyObjError::AccessError(AccessError::from(value))
    }
}

/// Guard which decrements and releases shared storage for the guarded reference.
struct AnyObjDecShared {
    shared: NonNull<Shared>,
}

impl Drop for AnyObjDecShared {
    fn drop(&mut self) {
        // Safety: We know that the inner value is live in this instance.
        unsafe {
            Shared::dec(self.shared);
        }
    }
}

/// Guard which decrements and releases shared storage for the guarded reference.
pub(crate) struct AnyObjDrop {
    #[allow(unused)]
    shared: NonNull<Shared>,
}

impl Drop for AnyObjDrop {
    fn drop(&mut self) {
        // Safety: We know that the inner value is live in this instance.
        unsafe {
            self.shared.as_ref().access.take();

            Shared::dec(self.shared);
        }
    }
}

pub(crate) struct RawAnyObjGuard {
    #[allow(unused)]
    guard: RawAccessGuard,
    #[allow(unused)]
    dec_shared: AnyObjDecShared,
}

/// A type-erased wrapper for a reference, whether it is mutable or not.
pub struct AnyObj {
    shared: NonNull<Shared>,
}

impl AnyObj {
    /// Construct an Any that wraps an owned object.
    pub(crate) fn new<T>(data: T) -> alloc::Result<Self>
    where
        T: Any,
    {
        let vtable = &Vtable {
            kind: Kind::Own,
            type_id: TypeId::of::<T>,
            debug: debug_ref_impl::<T>,
            type_name: type_name_impl::<T>,
            type_hash: T::type_hash,
            drop_value: const {
                if needs_drop::<T>() {
                    Some(drop_value::<T>)
                } else {
                    None
                }
            },
            drop: drop_box::<ManuallyDrop<T>>,
            clone: clone_own::<T>,
        };

        let shared = Shared {
            access: Access::new(),
            count: Cell::new(1),
            vtable,
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
    pub(crate) unsafe fn from_ref<T>(data: *const T) -> alloc::Result<Self>
    where
        T: Any,
    {
        let vtable = &Vtable {
            kind: Kind::Ref,
            type_id: TypeId::of::<T>,
            debug: debug_ref_impl::<T>,
            type_name: type_name_impl::<T>,
            type_hash: T::type_hash,
            drop_value: None,
            drop: drop_box::<NonNull<T>>,
            clone: clone_ref::<T>,
        };

        let shared = Shared {
            access: Access::new(),
            count: Cell::new(1),
            vtable,
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
    pub(crate) unsafe fn from_mut<T>(data: *mut T) -> alloc::Result<Self>
    where
        T: Any,
    {
        let vtable = &Vtable {
            kind: Kind::Mut,
            type_id: TypeId::of::<T>,
            debug: debug_mut_impl::<T>,
            type_name: type_name_impl::<T>,
            type_hash: T::type_hash,
            drop_value: None,
            drop: drop_box::<NonNull<T>>,
            clone: clone_mut::<T>,
        };

        let shared = Shared {
            access: Access::new(),
            count: Cell::new(1),
            vtable,
            data: NonNull::new_unchecked(data),
        };

        let shared = NonNull::from(Box::leak(Box::try_new(shared)?)).cast();
        Ok(Self { shared })
    }

    /// Downcast into an owned value of type `T`.
    pub(crate) fn downcast<T>(self) -> Result<T, AnyObjError>
    where
        T: Any,
    {
        let vtable = vtable(&self);

        if (vtable.type_id)() != TypeId::of::<T>() {
            return Err(AnyObjError::Cast(vtable.type_info()));
        }

        if !matches!(vtable.kind, Kind::Own) {
            return Err(AnyObjError::AccessError(AccessError::not_owned(
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
    pub(crate) fn drop(self) -> Result<(), AccessError> {
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

    /// Take the interior value and return a handle to the taken value.
    pub(crate) fn take(self) -> Result<Self, VmErrorKind> {
        let vtable = vtable(&self);

        if !matches!(vtable.kind, Kind::Own) {
            return Err(VmErrorKind::from(AccessError::not_owned(
                vtable.type_info(),
            )));
        }

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
    pub(crate) fn downcast_ref<T>(self) -> Result<Ref<T>, AnyObjError>
    where
        T: Any,
    {
        let vtable = vtable(&self);

        if (vtable.type_id)() != TypeId::of::<T>() {
            return Err(AnyObjError::Cast(vtable.type_info()));
        }

        if !matches!(vtable.kind, Kind::Own) {
            return Err(AnyObjError::AccessError(AccessError::not_owned(
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
                    let shared = shared.cast::<Shared>();
                    shared.as_ref().access.release();
                    Shared::dec(shared)
                },
            };

            let guard = RawAnyGuard::new(this.shared.cast(), vtable);
            Ok(Ref::new(data, guard))
        }
    }

    /// Downcast into an owned value of type [`Ref<T>`].
    ///
    /// # Errors
    ///
    /// This errors in case the underlying value is not owned, non-owned
    /// references cannot be coerced into [`Ref<T>`].
    pub(crate) fn downcast_mut<T>(self) -> Result<Mut<T>, AnyObjError>
    where
        T: Any,
    {
        let vtable = vtable(&self);

        if (vtable.type_id)() != TypeId::of::<T>() {
            return Err(AnyObjError::Cast(vtable.type_info()));
        }

        if !matches!(vtable.kind, Kind::Own) {
            return Err(AnyObjError::AccessError(AccessError::not_owned(
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
                    let shared = shared.cast::<Shared>();
                    shared.as_ref().access.release();
                    Shared::dec(shared)
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
    pub(crate) fn downcast_borrow_ref<T>(&self) -> Result<BorrowRef<'_, T>, AnyObjError>
    where
        T: Any,
    {
        let vtable = vtable(self);

        if (vtable.type_id)() != TypeId::of::<T>() {
            return Err(AnyObjError::Cast(vtable.type_info()));
        }

        // SAFETY: We've checked for the appropriate type just above.
        unsafe {
            let guard = self.shared.as_ref().access.shared()?;
            let data = vtable.as_ptr(self.shared);
            Ok(BorrowRef::new(data, guard))
        }
    }

    /// Get a reference to the interior value while checking for shared access.
    ///
    /// This prevents other exclusive accesses from being performed while the
    /// guard returned from this function is live.
    pub(crate) fn downcast_borrow_ptr<T>(self) -> Result<(NonNull<T>, RawAnyObjGuard), AnyObjError>
    where
        T: Any,
    {
        let vtable = vtable(&self);

        if (vtable.type_id)() != TypeId::of::<T>() {
            return Err(AnyObjError::Cast(vtable.type_info()));
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
    pub(crate) fn downcast_borrow_mut<T>(&self) -> Result<BorrowMut<'_, T>, AnyObjError>
    where
        T: Any,
    {
        let vtable = vtable(self);

        if (vtable.type_id)() != TypeId::of::<T>() {
            return Err(AnyObjError::Cast(vtable.type_info()));
        }

        if matches!(vtable.kind, Kind::Ref) {
            return Err(AnyObjError::Cast(vtable.type_info()));
        }

        // SAFETY: We've checked for the appropriate type just above.
        unsafe {
            let guard = self.shared.as_ref().access.exclusive()?;
            let data = vtable.as_ptr(self.shared);
            Ok(BorrowMut::new(data, guard))
        }
    }

    /// Returns some mutable reference to the boxed value if it is of type `T`.
    pub(crate) fn downcast_borrow_mut_ptr<T>(
        self,
    ) -> Result<(NonNull<T>, RawAnyObjGuard), AnyObjError>
    where
        T: Any,
    {
        let vtable = vtable(&self);

        if (vtable.type_id)() != TypeId::of::<T>() {
            return Err(AnyObjError::Cast(vtable.type_info()));
        }

        if matches!(vtable.kind, Kind::Ref) {
            return Err(AnyObjError::Cast(vtable.type_info()));
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
        Shared::inc(self.shared);

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
            !matches!(shared.vtable.kind, Kind::Ref) && shared.access.is_exclusive()
        }
    }

    /// Get access snapshot of shared value.
    pub(crate) fn snapshot(&self) -> Snapshot {
        unsafe { self.shared.as_ref().access.snapshot() }
    }

    /// Debug format the current any type.
    pub(crate) fn debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (vtable(self).debug)(f)
    }

    /// Access the underlying type id for the data.
    pub(crate) fn type_hash(&self) -> Hash {
        (vtable(self).type_hash)()
    }

    /// Access full type info for type.
    pub(crate) fn type_info(&self) -> TypeInfo {
        unsafe { self.shared.as_ref().type_info() }
    }
}

impl Clone for AnyObj {
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

impl fmt::Debug for AnyObj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug(f)
    }
}

impl Drop for AnyObj {
    fn drop(&mut self) {
        // Safety: We know that the inner value is live in this instance.
        unsafe {
            Shared::dec(self.shared);
        }
    }
}

/// The signature of a pointer coercion function.
type TypeIdFn = fn() -> TypeId;

/// The signature of a descriptive type name function.
type DebugFn = fn(&mut fmt::Formatter<'_>) -> fmt::Result;

/// Get the type name.
type TypeNameFn = fn() -> RawStr;

/// The signature of a type hash function.
type TypeHashFn = fn() -> Hash;

/// The kind of the stored value in the `AnyObj`.
enum Kind {
    /// Underlying access is shared.
    Ref,
    /// Underlying access is exclusive.
    Mut,
    /// Underlying access is owned.
    Own,
}

struct Vtable {
    /// The statically known kind of reference being stored.
    kind: Kind,
    /// Punt the inner pointer to the type corresponding to the type hash.
    type_id: TypeIdFn,
    /// Type information for diagnostics.
    debug: DebugFn,
    /// Type name accessor.
    type_name: TypeNameFn,
    /// Get the type hash of the stored type.
    type_hash: TypeHashFn,
    /// Value drop implementation. Set to `None` if the underlying value does
    /// not need to be dropped.
    drop_value: Option<unsafe fn(NonNull<Shared>)>,
    /// Only drop the box implementation.
    drop: unsafe fn(NonNull<Shared>),
    /// Clone the literal content of the shared value.
    clone: unsafe fn(NonNull<Shared>) -> alloc::Result<AnyObj>,
}

impl Vtable {
    fn as_ptr<T>(&self, base: NonNull<Shared>) -> NonNull<T> {
        if matches!(self.kind, Kind::Own) {
            unsafe { base.byte_add(offset_of!(Shared<T>, data)).cast() }
        } else {
            unsafe {
                base.byte_add(offset_of!(Shared<NonNull<T>>, data))
                    .cast()
                    .read()
            }
        }
    }

    /// Construct type information.
    fn type_info(&self) -> TypeInfo {
        TypeInfo::Any(AnyTypeInfo::__private_new(
            (self.type_name)(),
            (self.type_hash)(),
        ))
    }
}

#[repr(C)]
struct Shared<T = ()> {
    /// The currently handed out access to the shared data.
    access: Access,
    /// The number of strong references to the shared data.
    count: Cell<usize>,
    /// Vtable of the shared value.
    vtable: &'static Vtable,
    /// Data of the shared reference.
    data: T,
}

impl Shared {
    /// Construct type information.
    fn type_info(&self) -> TypeInfo {
        self.vtable.type_info()
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

fn debug_ref_impl<T>(f: &mut fmt::Formatter<'_>) -> fmt::Result
where
    T: ?Sized + Any,
{
    write!(f, "&{}", T::BASE_NAME)
}

fn debug_mut_impl<T>(f: &mut fmt::Formatter<'_>) -> fmt::Result
where
    T: ?Sized + Any,
{
    write!(f, "&mut {}", T::BASE_NAME)
}

fn type_name_impl<T>() -> RawStr
where
    T: ?Sized + Any,
{
    T::BASE_NAME
}

unsafe fn drop_value<T>(this: NonNull<Shared>) {
    let data = addr_of_mut!((*this.cast::<Shared<T>>().as_ptr()).data);
    drop_in_place(data);
}

unsafe fn drop_box<T>(this: NonNull<Shared>) {
    drop(Box::from_raw_in(this.cast::<Shared<T>>().as_ptr(), Global))
}

unsafe fn clone_own<T>(this: NonNull<Shared>) -> alloc::Result<AnyObj>
where
    T: Any,
{
    // NB: We read the value without deallocating it from the previous location,
    // since that would cause the returned value to be invalid.
    let value = addr_of_mut!((*this.cast::<Shared<T>>().as_ptr()).data).read();
    AnyObj::new(value)
}

unsafe fn clone_ref<T>(this: NonNull<Shared>) -> alloc::Result<AnyObj>
where
    T: Any,
{
    let value = addr_of_mut!((*this.cast::<Shared<NonNull<T>>>().as_ptr()).data).read();
    AnyObj::from_ref(value.as_ptr().cast_const())
}

unsafe fn clone_mut<T>(this: NonNull<Shared>) -> alloc::Result<AnyObj>
where
    T: Any,
{
    let value = addr_of_mut!((*this.cast::<Shared<NonNull<T>>>().as_ptr()).data).read();
    AnyObj::from_mut(value.as_ptr())
}

#[inline]
fn vtable(any: &AnyObj) -> &'static Vtable {
    unsafe { addr_of!((*any.shared.as_ptr()).vtable).read() }
}
