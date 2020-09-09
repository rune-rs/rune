//! Helper types for a holder of data.

use crate::Hash;
use std::any;
use std::fmt;

/// Our own private dynamic Any implementation.
///
/// In contrast to `Box<dyn std::any::Any>`, this allows for storing a raw
/// pointer directly in the object to avoid one level of indirection. Otherwise
/// it's equivalent.
#[repr(C)]
pub struct Any {
    vtable: &'static AnyVtable,
    data: *const (),
}

impl fmt::Debug for Any {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "Any({})", self.type_name())
    }
}

impl Any {
    /// Construct a new any from the original any.
    pub fn new<T>(data: T) -> Self
    where
        T: any::Any,
    {
        let data = Box::into_raw(Box::new(data));

        Self {
            vtable: &AnyVtable {
                drop: drop_impl::<T>,
                as_ptr: as_ptr_impl::<T>,
                type_name: any::type_name::<T>,
                type_hash: Hash::from_any::<T>,
            },
            data: data as *mut (),
        }
    }

    /// Construct a new any with the specified raw components.
    ///
    /// ### Safety
    /// The caller must ensure that the vtable matches up with the data pointer
    /// provided. This is primarily public for use in a C ffi.
    pub unsafe fn new_raw(vtable: &'static AnyVtable, data: *const ()) -> Self {
        Self { vtable, data }
    }

    /// Returns `true` if the boxed type is the same as `T`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let any = runestick::Any::new(1u32);
    /// assert!(any.is::<u32>());
    /// assert!(!any.is::<u64>());
    /// ```
    #[inline]
    pub fn is<T>(&self) -> bool
    where
        T: any::Any,
    {
        Hash::from_any::<T>() == self.type_hash()
    }

    /// Returns some reference to the boxed value if it is of type `T`, or
    /// `None` if it isn't.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let any = runestick::Any::new(1u32);
    /// assert_eq!(Some(&1u32), any.downcast_borrow_ref::<u32>());
    /// assert_eq!(None, any.downcast_borrow_ref::<&u32>());
    /// ```
    #[inline]
    pub fn downcast_borrow_ref<T>(&self) -> Option<&T>
    where
        T: any::Any,
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
    /// let mut any = runestick::Any::new(1u32);
    /// *any.downcast_borrow_mut::<u32>().unwrap() = 2;
    /// assert_eq!(Some(&2u32), any.downcast_borrow_ref::<u32>());
    /// ```
    #[inline]
    pub fn downcast_borrow_mut<T>(&mut self) -> Option<&mut T>
    where
        T: any::Any,
    {
        if self.is::<T>() {
            unsafe { Some(&mut *(self.data as *mut () as *mut T)) }
        } else {
            None
        }
    }

    /// Attempt to perform a conversion to a raw pointer.
    pub fn as_ptr(&self, expected: Hash) -> Option<*const ()> {
        // Safety: invariants are checked at construction time.
        unsafe { (self.vtable.as_ptr)(self.data, expected) }
    }

    /// Attempt to perform a conversion to a raw mutable pointer.
    pub fn as_mut_ptr(&mut self, expected: Hash) -> Option<*mut ()> {
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
    pub fn take_mut_ptr(self, expected: Hash) -> Result<*mut (), Self> {
        use std::mem::ManuallyDrop;

        let mut this = ManuallyDrop::new(self);

        match this.as_mut_ptr(expected) {
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

impl Drop for Any {
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
pub type TypeNameFn = fn() -> &'static str;

/// The signature of a type hash function.
pub type TypeHashFn = fn() -> Hash;

/// The vtable for any type stored in the virtual machine.
///
/// This can be implemented manually assuming it obeys the constraints of the
/// type. Otherwise we rely _heavily_ on the invariants provided by
/// `std::any::Any` which are checked at construction-time for this type.
#[repr(C)]
pub struct AnyVtable {
    /// The underlying drop implementation for the stored type.
    drop: DropFn,
    /// Punt the inner pointere to the type corresponding to the type hash.
    as_ptr: AsPtrFn,
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
    T: any::Any,
{
    if expected == Hash::from_type_id(any::TypeId::of::<T>()) {
        Some(this)
    } else {
        None
    }
}
