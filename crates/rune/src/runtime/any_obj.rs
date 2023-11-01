//! Helper types for a holder of data.

use core::any::TypeId;
use core::fmt;
use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};
use core::ptr;

use crate::alloc::alloc::Global;
use crate::alloc::{self, Box};
use crate::any::Any;
use crate::hash::Hash;
use crate::runtime::{AnyTypeInfo, FullTypeOf, MaybeTypeOf, RawStr, TypeInfo};

/// Errors raised during casting operations.
#[derive(Debug)]
#[allow(missing_docs)]
#[non_exhaustive]
pub enum AnyObjError {
    RefAsMut { name: RawStr },
    RefAsOwned { name: RawStr },
    MutAsOwned { name: RawStr },
    Cast,
}

impl fmt::Display for AnyObjError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AnyObjError::RefAsMut { name } => write!(
                f,
                "Cannot borrow a shared reference `&{name}` mutably as `&mut {name}`",
            ),
            AnyObjError::RefAsOwned { name } => {
                write!(f, "Cannot take ownership of a shared reference `&{name}`",)
            }
            AnyObjError::MutAsOwned { name } => write!(
                f,
                "Cannot take ownership of a mutable reference `&mut {name}`",
            ),
            AnyObjError::Cast {} => write!(f, "Cast failed"),
        }
    }
}

cfg_std! {
    impl std::error::Error for AnyObjError {}
}

/// Our own private dynamic Any implementation.
///
/// In contrast to `Box<dyn std::any::Any>`, this allows for storing a raw
/// pointer directly in the object to avoid one level of indirection. Otherwise
/// it's equivalent.
#[repr(C)]
pub struct AnyObj {
    vtable: &'static AnyObjVtable,
    data: ptr::NonNull<()>,
}

impl fmt::Debug for AnyObj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug(f)
    }
}

impl AnyObj {
    /// Construct a new any from the original any.
    pub fn new<T>(data: T) -> alloc::Result<Self>
    where
        T: Any,
    {
        let data = unsafe {
            let (ptr, Global) = Box::into_raw_with_allocator(Box::try_new_in(data, Global)?);
            ptr::NonNull::new_unchecked(ptr.cast())
        };

        Ok(Self {
            vtable: &AnyObjVtable {
                kind: AnyObjKind::Owned,
                drop: drop_impl::<T>,
                as_ptr: as_ptr_impl::<T>,
                debug: debug_impl::<T>,
                type_name: type_name_impl::<T>,
                type_hash: type_hash_impl::<T>,
            },
            data,
        })
    }

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
    /// use rune::runtime::AnyObj;
    ///
    /// #[derive(Any)]
    /// struct Foo(u32);
    ///
    /// let mut v = Foo(1u32);
    /// let any = unsafe { AnyObj::from_ref(&v) };
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
    /// use rune::runtime::AnyObj;
    ///
    /// #[derive(Any)]
    /// struct Foo(u32);
    ///
    /// let mut v = Foo(1u32);
    ///
    /// let any = unsafe { AnyObj::from_ref(&mut v) };
    /// let b = any.downcast_borrow_ref::<Foo>().unwrap();
    /// assert_eq!(b.0, 1u32);
    /// ```
    pub unsafe fn from_ref<T>(data: &T) -> Self
    where
        T: Any,
    {
        let data = ptr::NonNull::new_unchecked(data as *const _ as *mut _);

        Self {
            vtable: &AnyObjVtable {
                kind: AnyObjKind::RefPtr,
                drop: noop_drop_impl::<T>,
                as_ptr: as_ptr_impl::<T>,
                debug: debug_ref_impl::<T>,
                type_name: type_name_impl::<T>,
                type_hash: type_hash_impl::<T>,
            },
            data,
        }
    }

    /// Construct an Any that wraps a Deref type, behaving as the Target of
    /// the Deref implementation
    ///
    /// # Safety
    ///
    /// Caller must ensure that the returned `AnyObj` doesn't outlive the
    /// dereference target.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Any;
    /// use rune::runtime::AnyObj;
    /// use std::cell::RefCell;
    ///
    /// #[derive(Any)]
    /// struct Foo(u32);
    ///
    /// let mut v = RefCell::new(Foo(1u32));
    /// let mut guard = v.borrow();
    ///
    /// let any = unsafe { AnyObj::from_deref(guard)? };
    ///
    /// let b = any.downcast_borrow_ref::<Foo>().unwrap();
    /// assert_eq!(b.0, 1u32);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub unsafe fn from_deref<T>(data: T) -> alloc::Result<Self>
    where
        T: Deref,
        T::Target: Any,
    {
        let data = {
            let (ptr, Global) = Box::into_raw_with_allocator(Box::try_new_in(data, Global)?);
            ptr::NonNull::new_unchecked(ptr.cast())
        };

        Ok(Self {
            vtable: &AnyObjVtable {
                kind: AnyObjKind::RefPtr,
                drop: drop_impl::<T>,
                as_ptr: as_ptr_deref_impl::<T>,
                debug: debug_ref_impl::<T::Target>,
                type_name: type_name_impl::<T::Target>,
                type_hash: type_hash_impl::<T::Target>,
            },
            data,
        })
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
    /// use rune::runtime::AnyObj;
    ///
    /// #[derive(Any)]
    /// struct Foo(u32);
    ///
    /// let mut v = Foo(1u32);
    /// let any = unsafe { AnyObj::from_mut(&mut v) };
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
    /// use rune::runtime::AnyObj;
    ///
    /// #[derive(Any)]
    /// struct Foo(u32);
    ///
    /// let mut v = Foo(1u32);
    ///
    /// {
    ///     let mut any = unsafe { AnyObj::from_mut(&mut v) };
    ///
    ///     if let Some(v) = any.downcast_borrow_mut::<Foo>() {
    ///         v.0 += 1;
    ///     }
    /// }
    ///
    /// assert_eq!(v.0, 2);
    /// ```
    pub unsafe fn from_mut<T>(data: &mut T) -> Self
    where
        T: Any,
    {
        let data = ptr::NonNull::new_unchecked(data as *mut _ as *mut ());

        Self {
            vtable: &AnyObjVtable {
                kind: AnyObjKind::MutPtr,
                drop: noop_drop_impl::<T>,
                as_ptr: as_ptr_impl::<T>,
                debug: debug_mut_impl::<T>,
                type_name: type_name_impl::<T>,
                type_hash: type_hash_impl::<T>,
            },
            data,
        }
    }

    /// Construct an Any that wraps a DerefMut type, behaving as the Target of
    /// the DerefMut implementation
    ///
    /// # Safety
    ///
    /// Caller must ensure that the returned `AnyObj` doesn't outlive the
    /// dereference target.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Any;
    /// use rune::runtime::AnyObj;
    /// use std::cell::RefCell;
    ///
    /// #[derive(Any)]
    /// struct Foo(u32);
    ///
    /// let mut v = RefCell::new(Foo(1u32));
    /// let mut guard = v.borrow_mut();
    ///
    /// let any = unsafe { AnyObj::from_deref_mut(guard)? };
    ///
    /// let b = any.downcast_borrow_ref::<Foo>().unwrap();
    /// assert_eq!(b.0, 1u32);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub unsafe fn from_deref_mut<T>(data: T) -> alloc::Result<Self>
    where
        T: DerefMut,
        T::Target: Any,
    {
        let data = {
            let (ptr, Global) = Box::into_raw_with_allocator(Box::try_new_in(data, Global)?);
            ptr::NonNull::new_unchecked(ptr.cast())
        };

        Ok(Self {
            vtable: &AnyObjVtable {
                kind: AnyObjKind::MutPtr,
                drop: drop_impl::<T>,
                as_ptr: as_ptr_deref_mut_impl::<T>,
                debug: debug_mut_impl::<T::Target>,
                type_name: type_name_impl::<T::Target>,
                type_hash: type_hash_impl::<T::Target>,
            },
            data,
        })
    }

    /// Construct a new any with the specified raw components.
    ///
    /// ### Safety
    ///
    /// The caller must ensure that the vtable matches up with the data pointer
    /// provided. This is primarily public for use in a C ffi.
    pub unsafe fn new_raw(vtable: &'static AnyObjVtable, data: *const ()) -> Self {
        Self {
            vtable,
            data: ptr::NonNull::new_unchecked(data as *mut ()),
        }
    }

    /// Returns `true` if the boxed type is the same as `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Any;
    /// use rune::runtime::AnyObj;
    ///
    /// #[derive(Debug, Any)]
    /// struct Foo;
    ///
    /// #[derive(Debug, Any)]
    /// struct Other;
    ///
    /// let any = AnyObj::new(Foo)?;
    ///
    /// assert!(any.is::<Foo>());
    /// assert!(!any.is::<Other>());
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn is<T>(&self) -> bool
    where
        T: Any,
    {
        self.raw_as_ptr(TypeId::of::<T>()).is_ok()
    }

    /// Returns some reference to the boxed value if it is of type `T`, or
    /// `None` if it isn't.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Any;
    /// use rune::runtime::AnyObj;
    ///
    /// #[derive(Debug, PartialEq, Eq, Any)]
    /// struct Thing(u32);
    ///
    /// #[derive(Debug, PartialEq, Eq, Any)]
    /// struct Other;
    ///
    /// let any = AnyObj::new(Thing(1u32))?;
    /// assert_eq!(Some(&Thing(1u32)), any.downcast_borrow_ref::<Thing>());
    /// assert_eq!(None, any.downcast_borrow_ref::<Other>());
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn downcast_borrow_ref<T>(&self) -> Option<&T>
    where
        T: Any,
    {
        unsafe {
            (self.vtable.as_ptr)(self.data.as_ptr(), TypeId::of::<T>()).map(|v| &*(v as *const _))
        }
    }

    /// Returns some mutable reference to the boxed value if it is of type `T`, or
    /// `None` if it isn't.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Any;
    /// use rune::runtime::AnyObj;
    ///
    /// #[derive(Debug, PartialEq, Eq, Any)]
    /// struct Thing(u32);
    ///
    /// let mut any = AnyObj::new(Thing(1u32))?;
    /// any.downcast_borrow_mut::<Thing>().unwrap().0 = 2;
    /// assert_eq!(Some(&Thing(2u32)), any.downcast_borrow_ref::<Thing>());
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn downcast_borrow_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Any,
    {
        unsafe {
            (self.vtable.as_ptr)(self.data.as_ptr(), TypeId::of::<T>()).map(|v| &mut *(v as *mut _))
        }
    }

    /// Attempt to perform a conversion to a raw pointer.
    pub(crate) fn raw_as_ptr(&self, expected: TypeId) -> Result<*const (), AnyObjError> {
        // Safety: invariants are checked at construction time.
        match unsafe { (self.vtable.as_ptr)(self.data.as_ptr(), expected) } {
            Some(ptr) => Ok(ptr),
            None => Err(AnyObjError::Cast),
        }
    }

    /// Attempt to perform a conversion to a raw mutable pointer.
    pub(crate) fn raw_as_mut(&mut self, expected: TypeId) -> Result<*mut (), AnyObjError> {
        match self.vtable.kind {
            // Only owned and mutable pointers can be treated as mutable.
            AnyObjKind::Owned | AnyObjKind::MutPtr => (),
            _ => {
                return Err(AnyObjError::RefAsMut {
                    name: self.type_name(),
                })
            }
        }

        // Safety: invariants are checked at construction time.
        // We have mutable access to the inner value because we have mutable
        // access to the `Any`.
        match unsafe { (self.vtable.as_ptr)(self.data.as_ptr(), expected) } {
            Some(ptr) => Ok(ptr as *mut ()),
            None => Err(AnyObjError::Cast),
        }
    }

    /// Attempt to perform a conversion to a raw mutable pointer with the intent
    /// of taking it.
    ///
    /// If the conversion is not possible, we return a reconstructed `Any` as
    /// the error variant.
    pub(crate) fn raw_take(self, expected: TypeId) -> Result<*mut (), (AnyObjError, Self)> {
        match self.vtable.kind {
            // Only owned things can be taken.
            AnyObjKind::Owned => (),
            AnyObjKind::RefPtr => {
                return Err((
                    AnyObjError::RefAsOwned {
                        name: self.type_name(),
                    },
                    self,
                ))
            }
            AnyObjKind::MutPtr => {
                return Err((
                    AnyObjError::MutAsOwned {
                        name: self.type_name(),
                    },
                    self,
                ))
            }
        };

        let this = ManuallyDrop::new(self);

        // Safety: invariants are checked at construction time.
        // We have mutable access to the inner value because we have mutable
        // access to the `Any`.
        unsafe {
            match (this.vtable.as_ptr)(this.data.as_ptr(), expected) {
                Some(data) => Ok(data as *mut ()),
                None => {
                    let this = ManuallyDrop::into_inner(this);
                    Err((AnyObjError::Cast, this))
                }
            }
        }
    }

    /// Debug format the current any type.
    pub fn debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.vtable.debug)(f)
    }

    /// Access the underlying type name for the data.
    pub fn type_name(&self) -> RawStr {
        (self.vtable.type_name)()
    }

    /// Access the underlying type id for the data.
    pub fn type_hash(&self) -> Hash {
        (self.vtable.type_hash)()
    }

    /// Access full type info for type.
    pub fn type_info(&self) -> TypeInfo {
        TypeInfo::Any(AnyTypeInfo::__private_new(
            (self.vtable.type_name)(),
            (self.vtable.type_hash)(),
        ))
    }
}

impl MaybeTypeOf for AnyObj {
    fn maybe_type_of() -> Option<FullTypeOf> {
        None
    }
}

impl Drop for AnyObj {
    fn drop(&mut self) {
        // Safety: The safety of the called implementation is guaranteed at
        // compile time.
        unsafe {
            (self.vtable.drop)(self.data.as_ptr());
        }
    }
}

/// The signature of a drop function.
pub type DropFn = unsafe fn(*mut ());

/// The signature of a pointer coercion function.
pub type AsPtrFn = unsafe fn(this: *const (), expected: TypeId) -> Option<*const ()>;

/// The signature of a descriptive type name function.
pub type DebugFn = fn(&mut fmt::Formatter<'_>) -> fmt::Result;

/// Get the type name.
pub type TypeNameFn = fn() -> RawStr;

/// The signature of a type hash function.
pub type TypeHashFn = fn() -> Hash;

/// The kind of the stored value in the `AnyObj`.
enum AnyObjKind {
    /// A boxed value that is owned.
    Owned,
    /// A pointer (`*const T`).
    RefPtr,
    /// A mutable pointer (`*mut T`).
    MutPtr,
}

/// The vtable for any type stored in the virtual machine.
///
/// This can be implemented manually assuming it obeys the constraints of the
/// type. Otherwise we rely _heavily_ on the invariants provided by
/// `std::any::Any` which are checked at construction-time for this type.
#[repr(C)]
pub struct AnyObjVtable {
    /// The kind of the object being stored. Determines how it can be accessed.
    kind: AnyObjKind,
    /// The underlying drop implementation for the stored type.
    drop: DropFn,
    /// Punt the inner pointer to the type corresponding to the type hash.
    as_ptr: AsPtrFn,
    /// Type information for diagnostics.
    debug: DebugFn,
    /// Type name accessor.
    type_name: TypeNameFn,
    /// Get the type hash of the stored type.
    type_hash: TypeHashFn,
}

unsafe fn drop_impl<T>(this: *mut ()) {
    drop(Box::from_raw_in(this as *mut T, Global));
}

fn as_ptr_impl<T>(this: *const (), expected: TypeId) -> Option<*const ()>
where
    T: Any,
{
    if expected == TypeId::of::<T>() {
        Some(this)
    } else {
        None
    }
}

fn as_ptr_deref_impl<T: Deref>(this: *const (), expected: TypeId) -> Option<*const ()>
where
    T::Target: Any,
{
    if expected == TypeId::of::<T::Target>() {
        let guard = this as *const T;
        unsafe { Some((*guard).deref() as *const _ as *const ()) }
    } else {
        None
    }
}

fn as_ptr_deref_mut_impl<T: DerefMut>(this: *const (), expected: TypeId) -> Option<*const ()>
where
    T::Target: Any,
{
    if expected == TypeId::of::<T::Target>() {
        let guard = this as *mut T;
        unsafe { Some((*guard).deref_mut() as *const _ as *const ()) }
    } else {
        None
    }
}

fn noop_drop_impl<T>(_: *mut ()) {}

fn debug_impl<T>(f: &mut fmt::Formatter<'_>) -> fmt::Result
where
    T: Any,
{
    write!(f, "{}", T::BASE_NAME)
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

fn type_hash_impl<T>() -> Hash
where
    T: ?Sized + Any,
{
    T::type_hash()
}
