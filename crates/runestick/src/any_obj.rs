//! Helper types for a holder of data.

use crate::{Any, Hash, RawStr};
use std::any;
use std::fmt;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use thiserror::Error;

/// Errors raised during casting operations.
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum AnyObjError {
    #[error("cannot borrow a shared reference `&{name}` mutably as `&mut {name}`")]
    RefAsMut { name: RawStr },
    #[error("cannot take ownership of a shared reference `&{name}`")]
    RefAsOwned { name: RawStr },
    #[error("cannot take ownership of a mutable reference `&mut {name}`")]
    MutAsOwned { name: RawStr },
    #[error("cast failed")]
    Cast,
}

/// Our own private dynamic Any implementation.
///
/// In contrast to `Box<dyn std::any::Any>`, this allows for storing a raw
/// pointer directly in the object to avoid one level of indirection. Otherwise
/// it's equivalent.
#[repr(C)]
pub struct AnyObj {
    vtable: &'static AnyObjVtable,
    data: *const (),
}

impl fmt::Debug for AnyObj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.debug(f)
    }
}

impl AnyObj {
    /// Construct a new any from the original any.
    pub fn new<T>(data: T) -> Self
    where
        T: Any,
    {
        let data = Box::into_raw(Box::new(data));

        Self {
            vtable: &AnyObjVtable {
                kind: AnyObjKind::Owned,
                drop: drop_impl::<T>,
                as_ptr: as_ptr_impl::<T>,
                debug: debug_impl::<T>,
                type_name: type_name_impl::<T>,
                type_hash: type_hash_impl::<T>,
            },
            data: data as *mut (),
        }
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
    /// use runestick::{Any, AnyObj};
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
    /// use runestick::{Any, AnyObj};
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
        Self {
            vtable: &AnyObjVtable {
                kind: AnyObjKind::RefPtr,
                drop: noop_drop_impl::<T>,
                as_ptr: as_ptr_impl::<T>,
                debug: debug_ref_impl::<T>,
                type_name: type_name_impl::<T>,
                type_hash: type_hash_impl::<T>,
            },
            data: data as *const _ as *const (),
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
    /// use runestick::{Any, AnyObj};
    /// use std::cell::RefCell;
    ///
    /// #[derive(Any)]
    /// struct Foo(u32);
    ///
    /// let mut v = RefCell::new(Foo(1u32));
    /// let mut guard = v.borrow();
    ///
    /// let any = unsafe { AnyObj::from_deref(guard) };
    ///
    /// let b = any.downcast_borrow_ref::<Foo>().unwrap();
    /// assert_eq!(b.0, 1u32);
    /// ```
    pub unsafe fn from_deref<T, U: Deref<Target = T>>(data: U) -> Self
    where
        T: Any,
    {
        let boxed_guard = Box::into_raw(Box::new(data));
        Self {
            vtable: &AnyObjVtable {
                kind: AnyObjKind::RefPtr,
                drop: drop_impl::<U>,
                as_ptr: as_ptr_deref_impl::<T, U>,
                debug: debug_ref_impl::<T>,
                type_name: type_name_impl::<T>,
                type_hash: type_hash_impl::<T>,
            },
            data: boxed_guard as *const _ as *const (),
        }
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
    /// use runestick::{Any, AnyObj};
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
    /// use runestick::{Any, AnyObj};
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
        Self {
            vtable: &AnyObjVtable {
                kind: AnyObjKind::MutPtr,
                drop: noop_drop_impl::<T>,
                as_ptr: as_ptr_impl::<T>,
                debug: debug_mut_impl::<T>,
                type_name: type_name_impl::<T>,
                type_hash: type_hash_impl::<T>,
            },
            data: data as *const _ as *const (),
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
    /// use runestick::{Any, AnyObj};
    /// use std::cell::RefCell;
    ///
    /// #[derive(Any)]
    /// struct Foo(u32);
    ///
    /// let mut v = RefCell::new(Foo(1u32));
    /// let mut guard = v.borrow_mut();
    ///
    /// let any = unsafe { AnyObj::from_deref_mut(guard) };
    ///
    /// let b = any.downcast_borrow_ref::<Foo>().unwrap();
    /// assert_eq!(b.0, 1u32);
    /// ```
    pub unsafe fn from_deref_mut<T, U: DerefMut<Target = T>>(data: U) -> Self
    where
        T: Any,
    {
        let boxed_guard = Box::into_raw(Box::new(data));

        Self {
            vtable: &AnyObjVtable {
                kind: AnyObjKind::MutPtr,
                drop: drop_impl::<U>,
                as_ptr: as_ptr_deref_mut_impl::<T, U>,
                debug: debug_mut_impl::<T>,
                type_name: type_name_impl::<T>,
                type_hash: type_hash_impl::<T>,
            },
            data: boxed_guard as *const _ as *const (),
        }
    }

    /// Construct a new any with the specified raw components.
    ///
    /// ### Safety
    ///
    /// The caller must ensure that the vtable matches up with the data pointer
    /// provided. This is primarily public for use in a C ffi.
    pub unsafe fn new_raw(vtable: &'static AnyObjVtable, data: *const ()) -> Self {
        Self { vtable, data }
    }

    /// Returns `true` if the boxed type is the same as `T`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use runestick::Any;
    ///
    /// #[derive(Debug, Any)]
    /// struct Foo;
    ///
    /// #[derive(Debug, Any)]
    /// struct Other;
    ///
    /// let any = runestick::AnyObj::new(Foo);
    ///
    /// assert!(any.is::<Foo>());
    /// assert!(!any.is::<Other>());
    /// ```
    #[inline]
    pub fn is<T>(&self) -> bool
    where
        T: Any,
    {
        Hash::from_any::<T>() == self.type_hash()
    }

    /// Returns some reference to the boxed value if it is of type `T`, or
    /// `None` if it isn't.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use runestick::Any;
    ///
    /// #[derive(Debug, PartialEq, Eq, Any)]
    /// struct Thing(u32);
    ///
    /// #[derive(Debug, PartialEq, Eq, Any)]
    /// struct Other;
    ///
    /// let any = runestick::AnyObj::new(Thing(1u32));
    /// assert_eq!(Some(&Thing(1u32)), any.downcast_borrow_ref::<Thing>());
    /// assert_eq!(None, any.downcast_borrow_ref::<Other>());
    /// ```
    #[inline]
    pub fn downcast_borrow_ref<T>(&self) -> Option<&T>
    where
        T: Any,
    {
        unsafe { (self.vtable.as_ptr)(self.data, Hash::from_any::<T>()).map(|v| &*(v as *const _)) }
    }

    /// Returns some mutable reference to the boxed value if it is of type `T`, or
    /// `None` if it isn't.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use runestick::Any;
    ///
    /// #[derive(Debug, PartialEq, Eq, Any)]
    /// struct Thing(u32);
    ///
    /// let mut any = runestick::AnyObj::new(Thing(1u32));
    /// any.downcast_borrow_mut::<Thing>().unwrap().0 = 2;
    /// assert_eq!(Some(&Thing(2u32)), any.downcast_borrow_ref::<Thing>());
    /// ```
    #[inline]
    pub fn downcast_borrow_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Any,
    {
        unsafe {
            (self.vtable.as_ptr)(self.data, Hash::from_any::<T>()).map(|v| &mut *(v as *mut _))
        }
    }

    /// Attempt to perform a conversion to a raw pointer.
    pub(crate) fn raw_as_ptr(&self, expected: Hash) -> Result<*const (), AnyObjError> {
        // Safety: invariants are checked at construction time.
        match unsafe { (self.vtable.as_ptr)(self.data, expected) } {
            Some(ptr) => Ok(ptr),
            None => Err(AnyObjError::Cast),
        }
    }

    /// Attempt to perform a conversion to a raw mutable pointer.
    pub(crate) fn raw_as_mut(&mut self, expected: Hash) -> Result<*mut (), AnyObjError> {
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
        match unsafe { (self.vtable.as_ptr)(self.data, expected) } {
            Some(ptr) => Ok(ptr as *mut ()),
            None => Err(AnyObjError::Cast),
        }
    }

    /// Attempt to perform a conversion to a raw mutable pointer with the intent
    /// of taking it.
    ///
    /// If the conversion is not possible, we return a reconstructed `Any` as
    /// the error variant.
    pub(crate) fn raw_take(self, expected: Hash) -> Result<*mut (), (AnyObjError, Self)> {
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
            match (this.vtable.as_ptr)(this.data, expected) {
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
}

impl Drop for AnyObj {
    fn drop(&mut self) {
        // Safety: The safety of the called implementation is guaranteed at
        // compile time.
        unsafe {
            (self.vtable.drop)(self.data);
        }
    }
}

/// The signature of a drop function.
pub type DropFn = unsafe fn(*const ());

/// The signature of a pointer coercion function.
pub type AsPtrFn = unsafe fn(this: *const (), expected: Hash) -> Option<*const ()>;

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

unsafe fn drop_impl<T>(this: *const ()) {
    Box::from_raw(this as *mut () as *mut T);
}

fn as_ptr_impl<T>(this: *const (), expected: Hash) -> Option<*const ()>
where
    T: Any,
{
    if expected == Hash::from_type_id(any::TypeId::of::<T>()) {
        Some(this)
    } else {
        None
    }
}

fn as_ptr_deref_impl<T, U: std::ops::Deref<Target = T>>(
    this: *const (),
    expected: Hash,
) -> Option<*const ()>
where
    T: Any,
{
    if expected == Hash::from_type_id(any::TypeId::of::<T>()) {
        let guard = this as *const U;
        unsafe { Some((*guard).deref() as *const _ as *const ()) }
    } else {
        None
    }
}

fn as_ptr_deref_mut_impl<T, U: std::ops::DerefMut<Target = T>>(
    this: *const (),
    expected: Hash,
) -> Option<*const ()>
where
    T: Any,
{
    if expected == Hash::from_type_id(any::TypeId::of::<T>()) {
        let guard = this as *mut U;
        unsafe { Some((*guard).deref_mut() as *const _ as *const ()) }
    } else {
        None
    }
}

fn noop_drop_impl<T>(_: *const ()) {}

fn debug_impl<T>(f: &mut fmt::Formatter<'_>) -> fmt::Result
where
    T: Any,
{
    write!(f, "{}", T::BASE_NAME)
}

fn debug_ref_impl<T>(f: &mut fmt::Formatter<'_>) -> fmt::Result
where
    T: Any,
{
    write!(f, "&{}", T::BASE_NAME)
}

fn debug_mut_impl<T>(f: &mut fmt::Formatter<'_>) -> fmt::Result
where
    T: Any,
{
    write!(f, "&mut {}", T::BASE_NAME)
}

fn type_name_impl<T>() -> RawStr
where
    T: Any,
{
    T::BASE_NAME
}

fn type_hash_impl<T>() -> Hash
where
    T: Any,
{
    T::type_hash()
}
