//! Helper types for a holder of data.

use crate::{Any, Hash};
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
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "AnyObj({})", self.type_name())
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
                drop: drop_impl::<T>,
                as_ptr: as_ptr_impl::<T>,
                as_mut: as_mut_impl::<T>,
                take: as_mut_impl::<T>,
                type_name: any::type_name::<T>,
                type_hash: Hash::from_any::<T>,
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
                drop: noop_drop_impl::<T>,
                as_ptr: as_ptr_impl::<T>,
                // Raw pointers cannot be casted to mutable pointers.
                as_mut: not_supported::<T, _, _>,
                // Pointers cannot be "taken", because they're not owned by the
                // any.
                take: not_supported::<T, _, _>,
                type_name: any::type_name::<T>,
                type_hash: Hash::from_any::<T>,
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
                drop: noop_drop_impl::<T>,
                as_ptr: as_ptr_impl::<T>,
                as_mut: as_mut_impl::<T>,
                // Pointers cannot be "taken", because they're not owned by the
                // any.
                take: not_supported::<T, _, _>,
                type_name: any::type_name::<T>,
                type_hash: Hash::from_any::<T>,
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
        // Safety: invariants are checked at construction time.
        // We have mutable access to the inner value because we have mutable
        // access to the `Any`.
        unsafe { (self.vtable.as_mut)(self.data, expected) }
    }

    /// Attempt to perform a conversion to a raw mutable pointer with the intent
    /// of taking it.
    ///
    /// If the conversion is not possible, we return a reconstructed `Any` as
    /// the error variant.
    pub fn raw_take(self, expected: Hash) -> Result<*mut (), Self> {
        let this = ManuallyDrop::new(self);

        // Safety: invariants are checked at construction time.
        // We have mutable access to the inner value because we have mutable
        // access to the `Any`.
        match unsafe { (this.vtable.take)(this.data, expected) } {
            Some(data) => Ok(data),
            None => Err(ManuallyDrop::into_inner(this)),
        }
    }

    /// Access the underlying type name for the data.
    pub fn type_name(&self) -> &'static str {
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

/// The signature of a pointer coercion function.
pub type AsMutFn = unsafe fn(*const (), expected: Hash) -> Option<*mut ()>;

/// The signature of a pointer coercion function.
pub type TakeFn = unsafe fn(*const (), expected: Hash) -> Option<*mut ()>;

/// The signature of a descriptive type name function.
pub type TypeNameFn = fn() -> &'static str;

/// The signature of a type hash function.
pub type TypeHashFn = fn() -> Hash;

/// The vtable for any type stored in the virtual machine.
///
/// This can be implemented manually assuming it obeys the constraints of the
/// type. Otherwise we rely _heavily_ on the invariants provided by
/// `std::any::Any` which are checked at construction-time for this type.
#[repr(C)]
pub struct AnyObjVtable {
    /// The underlying drop implementation for the stored type.
    drop: DropFn,
    /// Punt the inner pointer to the type corresponding to the type hash.
    as_ptr: AsPtrFn,
    /// Punt the inner pointer to the type corresponding to the type hash.
    as_mut: AsMutFn,
    /// Punt the inner pointer to the type corresponding to the type hash.
    take: TakeFn,
    /// Type information for diagnostics.
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

fn as_mut_impl<T>(this: *const (), expected: Hash) -> Option<*mut ()>
where
    T: Any,
{
    if expected == Hash::from_type_id(any::TypeId::of::<T>()) {
        Some(this as *mut ())
    } else {
        None
    }
}

fn not_supported<T, P, O>(_: P, _: Hash) -> Option<O>
where
    T: Any,
{
    None
}
