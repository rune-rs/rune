use core::any::TypeId;
use core::cell::Cell;
use core::fmt;
use core::mem::ManuallyDrop;
use core::ptr::{self, NonNull};

use crate::alloc::alloc::Global;
use crate::alloc::{self, Box};
use crate::{Any, Hash};

use super::{
    Access, AccessError, AccessErrorKind, AnyTypeInfo, BorrowMut, BorrowRef, RawAccessGuard,
    RawStr, Snapshot, TypeInfo,
};

/// Guard which decrements and releases shared storage for the guarded reference.
struct AnyRefDecShared {
    shared: NonNull<Shared>,
}

impl Drop for AnyRefDecShared {
    fn drop(&mut self) {
        // Safety: We know that the inner value is live in this instance.
        unsafe {
            Shared::dec(self.shared);
        }
    }
}

/// Guard which decrements and releases shared storage for the guarded reference.
pub(crate) struct AnyRefDrop {
    _shared: NonNull<Shared>,
}

impl Drop for AnyRefDrop {
    fn drop(&mut self) {
        // Safety: We know that the inner value is live in this instance.
        unsafe {
            let guard = self
                ._shared
                .as_ref()
                .access
                .take()
                .expect("expected exclusive access");

            let _guard = ManuallyDrop::new(guard);
            Shared::dec(self._shared);
        }
    }
}

pub(crate) struct RawAnyRefGuard {
    #[allow(unused)]
    guard: RawAccessGuard,
    #[allow(unused)]
    dec_shared: AnyRefDecShared,
}

/// A type-erased wrapper for a reference, whether it is mutable or not.
pub struct AnyRef {
    shared: NonNull<Shared>,
}

impl AnyRef {
    /// Construct an Any that wraps a pointer.
    ///
    /// # Safety
    ///
    /// Caller must ensure that the returned `AnyObj` doesn't outlive the
    /// reference it is wrapping.
    ///
    /// This would be an example of incorrect use:
    ///
    /// ```no_run
    /// use rune::Any;
    /// use rune::runtime::AnyRef;
    ///
    /// #[derive(Any)]
    /// struct Foo(u32);
    ///
    /// let mut v = Foo(1u32);
    /// let any = unsafe { AnyRef::from_ref(&v)? };
    ///
    /// drop(v);
    ///
    /// // any use of `any` beyond here is undefined behavior.
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Any;
    /// use rune::runtime::AnyRef;
    ///
    /// #[derive(Any)]
    /// struct Foo(u32);
    ///
    /// let mut v = Foo(1u32);
    ///
    /// let any = unsafe { AnyRef::from_ref(&v)? };
    /// let b = any.downcast_borrow_ref::<Foo>()?;
    /// assert_eq!(b.0, 1u32);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub unsafe fn from_ref<T>(data: *const T) -> alloc::Result<Self>
    where
        T: Any,
    {
        let shared = Shared {
            data: NonNull::new_unchecked(data.cast_mut()).cast(),
            access: Access::new(),
            count: Cell::new(1),
            vtable: &Vtable {
                kind: Kind::Ref,
                type_id: TypeId::of::<T>,
                debug: debug_ref_impl::<T>,
                type_name: type_name_impl::<T>,
                type_hash: T::type_hash,
            },
        };

        let shared = NonNull::from(Box::leak(Box::try_new(shared)?));

        Ok(Self { shared })
    }

    /// Construct an Any that wraps a mutable pointer.
    ///
    /// # Safety
    ///
    /// Caller must ensure that the returned `AnyObj` doesn't outlive the
    /// reference it is wrapping.
    ///
    /// This would be an example of incorrect use:
    ///
    /// ```no_run
    /// use rune::Any;
    /// use rune::runtime::AnyRef;
    ///
    /// #[derive(Any)]
    /// struct Foo(u32);
    ///
    /// let mut v = Foo(1u32);
    /// let any = unsafe { AnyRef::from_mut(&mut v) };
    ///
    /// drop(v);
    ///
    /// // any use of `any` beyond here is undefined behavior.
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Any;
    /// use rune::runtime::{AnyRef, VmResult};
    ///
    /// #[derive(Any)]
    /// struct Foo(u32);
    ///
    /// let mut v = Foo(1u32);
    ///
    /// {
    ///     let mut any = unsafe { AnyRef::from_mut(&mut v)? };
    ///
    ///     let Ok(mut v) = any.downcast_borrow_mut::<Foo>() else {
    ///         panic!("downcast failed");
    ///     };
    ///
    ///     v.0 += 1;
    /// }
    ///
    /// assert_eq!(v.0, 2);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub unsafe fn from_mut<T>(data: *mut T) -> alloc::Result<Self>
    where
        T: Any,
    {
        let shared = Shared {
            data: NonNull::new_unchecked(data).cast(),
            access: Access::new(),
            count: Cell::new(1),
            vtable: &Vtable {
                kind: Kind::Mut,
                type_id: TypeId::of::<T>,
                debug: debug_mut_impl::<T>,
                type_name: type_name_impl::<T>,
                type_hash: T::type_hash,
            },
        };

        let shared = NonNull::from(Box::leak(Box::try_new(shared)?));

        Ok(Self { shared })
    }

    /// Get a reference to the interior value while checking for shared access.
    ///
    /// This prevents other exclusive accesses from being performed while the
    /// guard returned from this function is live.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Any;
    /// use rune::runtime::AnyRef;
    ///
    /// #[derive(Debug, PartialEq, Eq, Any)]
    /// struct Thing(u32);
    ///
    /// let mut t = Thing(1u32);
    ///
    /// let mut any = unsafe { AnyRef::from_ref(&t)? };
    /// assert_eq!(Ok(&Thing(1u32)), any.downcast_borrow_ref::<Thing>().as_deref());
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn downcast_borrow_ref<T>(&self) -> Result<BorrowRef<'_, T>, AccessError>
    where
        T: Any,
    {
        // Safety: We know that interior value is alive since this container is
        // alive.
        //
        // Appropriate access is checked when constructing the guards.
        unsafe {
            let shared = self.shared.as_ref();

            if (shared.vtable.type_id)() != TypeId::of::<T>() {
                return Err(AccessError::new(AccessErrorKind::Cast));
            }

            let guard = shared.access.shared()?;
            Ok(BorrowRef::new(shared.data.cast::<T>().as_ref(), guard))
        }
    }

    /// Get a reference to the interior value while checking for shared access.
    ///
    /// This prevents other exclusive accesses from being performed while the
    /// guard returned from this function is live.
    pub(crate) fn downcast_borrow_ptr<T>(self) -> Result<(NonNull<T>, RawAnyRefGuard), AccessError>
    where
        T: Any,
    {
        // Safety: We know that interior value is alive since this container is
        // alive.
        //
        // Appropriate access is checked when constructing the guards.
        unsafe {
            let shared = self.shared.as_ref();

            if (shared.vtable.type_id)() != TypeId::of::<T>() {
                return Err(AccessError::new(AccessErrorKind::Cast));
            }

            let guard = shared.access.shared()?;

            let guard = RawAnyRefGuard {
                guard: guard.into_raw(),
                dec_shared: AnyRefDecShared {
                    shared: self.shared,
                },
            };

            Ok((shared.data.cast::<T>(), guard))
        }
    }

    /// Returns some mutable reference to the boxed value if it is of type `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Any;
    /// use rune::runtime::AnyRef;
    ///
    /// #[derive(Debug, PartialEq, Eq, Any)]
    /// struct Thing(u32);
    ///
    /// let mut t = Thing(1u32);
    ///
    /// let mut any = unsafe { AnyRef::from_mut(&mut t)? };
    /// any.downcast_borrow_mut::<Thing>().unwrap().0 = 2;
    /// assert_eq!(Ok(&Thing(2u32)), any.downcast_borrow_ref::<Thing>().as_deref());
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn downcast_borrow_mut<T>(&self) -> Result<BorrowMut<'_, T>, AccessError>
    where
        T: Any,
    {
        unsafe {
            let shared = self.shared.as_ref();

            if (shared.vtable.type_id)() != TypeId::of::<T>() {
                return Err(AccessError::new(AccessErrorKind::Cast));
            }

            if let Kind::Ref = shared.vtable.kind {
                return Err(AccessError::new(AccessErrorKind::RefAsMut {
                    name: self.type_name(),
                }));
            }

            let guard = shared.access.exclusive()?;
            Ok(BorrowMut::new(shared.data.cast::<T>().as_mut(), guard))
        }
    }

    /// Returns some mutable reference to the boxed value if it is of type `T`.
    #[inline]
    pub(crate) fn downcast_borrow_mut_ptr<T>(
        self,
    ) -> Result<(NonNull<T>, RawAnyRefGuard), AccessError>
    where
        T: Any,
    {
        unsafe {
            let shared = self.shared.as_ref();

            if (shared.vtable.type_id)() != TypeId::of::<T>() {
                return Err(AccessError::new(AccessErrorKind::Cast));
            }

            if let Kind::Ref = shared.vtable.kind {
                return Err(AccessError::new(AccessErrorKind::RefAsMut {
                    name: self.type_name(),
                }));
            }

            let guard = shared.access.exclusive()?;

            let guard = RawAnyRefGuard {
                guard: guard.into_raw(),
                dec_shared: AnyRefDecShared {
                    shared: self.shared,
                },
            };

            Ok((shared.data.cast::<T>(), guard))
        }
    }

    /// Deconstruct the shared value into a guard and shared box.
    ///
    /// # Safety
    ///
    /// The content of the shared value will be forcibly destructed once the
    /// returned guard is dropped, unchecked use of the shared value after this
    /// point will lead to undefined behavior.
    pub(crate) unsafe fn into_drop_guard(self) -> (Self, AnyRefDrop) {
        // Increment the reference count by one to account for the guard holding
        // onto it.
        Shared::inc(self.shared);

        let guard = AnyRefDrop {
            _shared: self.shared,
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

            // Safety: Since we have a reference to this shared, we know that the
            // inner is available.
            !matches!(shared.vtable.kind, Kind::Ref) && shared.access.is_exclusive()
        }
    }

    /// Get access snapshot of shared value.
    pub(crate) fn snapshot(&self) -> Snapshot {
        unsafe { self.shared.as_ref().access.snapshot() }
    }

    /// Debug format the current any type.
    pub(crate) fn debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe { (self.shared.as_ref().vtable.debug)(f) }
    }

    /// Access the underlying type name for the data.
    pub(crate) fn type_name(&self) -> RawStr {
        unsafe { (self.shared.as_ref().vtable.type_name)() }
    }

    /// Access the underlying type id for the data.
    pub(crate) fn type_hash(&self) -> Hash {
        unsafe { (self.shared.as_ref().vtable.type_hash)() }
    }

    /// Access full type info for type.
    pub(crate) fn type_info(&self) -> TypeInfo {
        unsafe {
            TypeInfo::Any(AnyTypeInfo::__private_new(
                (self.shared.as_ref().vtable.type_name)(),
                (self.shared.as_ref().vtable.type_hash)(),
            ))
        }
    }
}

impl Clone for AnyRef {
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

        // SAFETY: We know that the inner value is live in both instances.
        unsafe {
            Shared::dec(self.shared);
            Shared::inc(source.shared);
        }

        self.shared = source.shared;
    }
}

impl fmt::Debug for AnyRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug(f)
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
    /// A pointer (`*const T`).
    Ref,
    /// A mutable pointer (`*mut T`).
    Mut,
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
}

struct Shared {
    /// Data of the shared reference.
    data: NonNull<()>,
    /// The currently handed out access to the shared data.
    access: Access,
    /// The number of strong references to the shared data.
    count: Cell<usize>,
    /// Vtable of the shared value.
    vtable: &'static Vtable,
}

impl Shared {
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

        let this = Box::from_raw_in(this.as_ptr(), Global);

        if this.access.is_taken() {
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
