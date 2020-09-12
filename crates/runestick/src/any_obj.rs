//! Helper types for a holder of data.

use crate::{Any, Hash, RawStr};
use std::any;
use std::fmt;
use std::mem::ManuallyDrop;

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
    pub fn from_ref<T>(data: &T) -> Self
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

    /// Construct an Any that wraps a mutable pointer.
    pub fn from_mut<T>(data: &mut T) -> Self
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
            data: data as *mut _ as *mut () as *const (),
        }
    }

    /// Construct a new any with the specified raw components.
    ///
    /// ### Safety
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
        if self.is::<T>() {
            unsafe { Some(&*(self.data as *const T)) }
        } else {
            None
        }
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
        if self.is::<T>() {
            unsafe { Some(&mut *(self.data as *mut () as *mut T)) }
        } else {
            None
        }
    }

    /// Attempt to perform a conversion to a raw pointer.
    pub fn raw_as_ptr(&self, expected: Hash) -> Option<*const ()> {
        // Safety: invariants are checked at construction time.
        unsafe { (self.vtable.as_ptr)(self.data, expected) }
    }

    /// Attempt to perform a conversion to a raw mutable pointer.
    pub fn raw_as_mut(&mut self, expected: Hash) -> Option<*mut ()> {
        match self.vtable.kind {
            // Only owned and mutable pointers can be treated as mutable.
            AnyObjKind::Owned | AnyObjKind::MutPtr => (),
            _ => return None,
        }

        // Safety: invariants are checked at construction time.
        // We have mutable access to the inner value because we have mutable
        // access to the `Any`.
        unsafe {
            let ptr = (self.vtable.as_ptr)(self.data, expected)?;
            Some(ptr as *mut ())
        }
    }

    /// Attempt to perform a conversion to a raw mutable pointer with the intent
    /// of taking it.
    ///
    /// If the conversion is not possible, we return a reconstructed `Any` as
    /// the error variant.
    pub fn raw_take(self, expected: Hash) -> Result<*mut (), Self> {
        match self.vtable.kind {
            // Only owned things can be taken.
            AnyObjKind::Owned => (),
            _ => return Err(self),
        };

        let this = ManuallyDrop::new(self);

        // Safety: invariants are checked at construction time.
        // We have mutable access to the inner value because we have mutable
        // access to the `Any`.
        unsafe {
            match (this.vtable.as_ptr)(this.data, expected) {
                Some(data) => Ok(data as *mut ()),
                None => Err(ManuallyDrop::into_inner(this)),
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
pub type AsPtrFn = unsafe fn(*const (), expected: Hash) -> Option<*const ()>;

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

fn noop_drop_impl<T>(_: *const ()) {}

fn debug_impl<T>(f: &mut fmt::Formatter<'_>) -> fmt::Result
where
    T: Any,
{
    write!(f, "{}", T::NAME)
}

fn debug_ref_impl<T>(f: &mut fmt::Formatter<'_>) -> fmt::Result
where
    T: Any,
{
    write!(f, "&{}", T::NAME)
}

fn debug_mut_impl<T>(f: &mut fmt::Formatter<'_>) -> fmt::Result
where
    T: Any,
{
    write!(f, "&mut {}", T::NAME)
}

fn type_name_impl<T>() -> RawStr
where
    T: Any,
{
    T::NAME
}

fn type_hash_impl<T>() -> Hash
where
    T: Any,
{
    T::type_hash()
}
