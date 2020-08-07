//! Helper types for a holder of data.

use std::any;
use std::fmt;

/// Our own private dynamic Any implementation.
///
/// In contrast to `Box<dyn std::any::Any>`, this allows for storing a raw
/// pointer directly in the object to avoid one level of indirection. Otherwise
/// it's equivalent.
#[repr(C)]
pub struct Any {
    data: *const (),
    vtable: &'static Vtable,
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

        return Any {
            vtable: &Vtable {
                drop: drop_impl::<T>,
                as_ptr: as_ptr_impl::<T>,
                as_mut_ptr: as_mut_ptr_impl::<T>,
                take_mut_ptr: as_mut_ptr_impl::<T>,
                type_name: any::type_name::<T>,
                type_id: any::TypeId::of::<T>,
            },
            data: data as *mut (),
        };

        unsafe fn drop_impl<T>(this: *const ()) {
            Box::from_raw(this as *mut () as *mut T);
        }
    }

    /// Construct a new any from a pointer.
    ///
    /// # Safety
    ///
    /// It is up to the caller to make sure that whatever data is pointed to is
    /// valid for the duration of the `Any`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let value = 1u32;
    /// let any = unsafe { runestick::Any::from_ptr(&value) };
    /// assert!(any.is::<u32>());
    /// assert_eq!(Some(&1u32), any.downcast_ref());
    /// ```
    pub unsafe fn from_ptr<T>(data: *const T) -> Self
    where
        T: any::Any,
    {
        Any {
            vtable: &Vtable {
                drop: noop_drop_impl::<T>,
                as_ptr: as_ptr_impl::<T>,
                as_mut_ptr: unsupported_as_mut::<T>,
                take_mut_ptr: unsupported_as_mut::<T>,
                type_name: any::type_name::<T>,
                type_id: any::TypeId::of::<T>,
            },
            data: data as *const (),
        }
    }

    /// Construct a new any from a mutable pointer.
    ///
    /// # Safety
    ///
    /// It is up to the caller to make sure that whatever data is pointed to is
    /// valid for the duration of the `Any`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut value = 1u32;
    /// let mut any = unsafe { runestick::Any::from_mut_ptr(&mut value) };
    /// assert!(any.is::<u32>());
    /// *any.downcast_mut::<u32>().unwrap() = 2;
    /// assert_eq!(Some(&2u32), any.downcast_ref());
    /// ```
    pub unsafe fn from_mut_ptr<T>(data: *mut T) -> Self
    where
        T: any::Any,
    {
        Any {
            vtable: &Vtable {
                drop: noop_drop_impl::<T>,
                as_ptr: as_ptr_impl::<T>,
                as_mut_ptr: as_mut_ptr_impl::<T>,
                take_mut_ptr: unsupported_as_mut::<T>,
                type_name: any::type_name::<T>,
                type_id: any::TypeId::of::<T>,
            },
            data: data as *mut (),
        }
    }

    /// Returns `true` if the boxed type is the same as `T`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let any = runestick::Any::new(1u32);
    /// assert!(any.is::<u32>());
    /// ```
    #[inline]
    pub fn is<T>(&self) -> bool
    where
        T: any::Any,
    {
        any::TypeId::of::<T>() == self.type_id()
    }

    /// Returns some reference to the boxed value if it is of type `T`, or
    /// `None` if it isn't.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let any = runestick::Any::new(1u32);
    /// assert_eq!(Some(&1u32), any.downcast_ref::<u32>());
    /// assert_eq!(None, any.downcast_ref::<&u32>());
    /// ```
    #[inline]
    pub fn downcast_ref<T>(&self) -> Option<&T>
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
    /// *any.downcast_mut::<u32>().unwrap() = 2;
    /// assert_eq!(Some(&2u32), any.downcast_ref::<u32>());
    /// ```
    #[inline]
    pub fn downcast_mut<T>(&mut self) -> Option<&mut T>
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
    pub fn as_ptr(&self, expected_type: any::TypeId) -> Option<*const ()> {
        // Safety: invariants are checked at construction time.
        unsafe { (self.vtable.as_ptr)(self.data, expected_type) }
    }

    /// Attempt to perform a conversion to a raw mutable pointer.
    pub fn as_mut_ptr(&mut self, expected_type: any::TypeId) -> Option<*mut ()> {
        // Safety: invariants are checked at construction time.
        unsafe { (self.vtable.as_mut_ptr)(self.data, expected_type) }
    }

    /// Attempt to perform a conversion to a raw mutable pointer with the intent
    /// of taking it.
    ///
    /// If the conversion is not possible, we return a reconstructed `Any` as
    /// the error variant.
    pub fn take_mut_ptr(self, expected_type: any::TypeId) -> Result<*mut (), Self> {
        use std::mem::ManuallyDrop;

        let this = ManuallyDrop::new(self);

        // Safety: invariants are checked at construction time.
        match unsafe { (this.vtable.take_mut_ptr)(this.data, expected_type) } {
            Some(data) => Ok(data),
            None => Err(ManuallyDrop::into_inner(this)),
        }
    }

    /// Access the underlying type name for the data.
    pub fn type_name(&self) -> &'static str {
        (self.vtable.type_name)()
    }

    /// Access the underlying type id for the data.
    pub fn type_id(&self) -> any::TypeId {
        (self.vtable.type_id)()
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

type DropFn = unsafe fn(*const ());
type AsPtrFn = unsafe fn(*const (), expected_type: any::TypeId) -> Option<*const ()>;
type AsMutPtrFn = unsafe fn(*const (), expected_type: any::TypeId) -> Option<*mut ()>;
type TakeMutPtrFn = unsafe fn(*const (), expected_type: any::TypeId) -> Option<*mut ()>;
type TypeNameFn = fn() -> &'static str;
type TypeIdFn = fn() -> any::TypeId;

/// The vtable for any type stored in the virtual machine.
///
/// We rely _heavily_ on the invariants provided by `std::any::Any` which are
/// checked at construction-time for this type.
#[repr(C)]
struct Vtable {
    /// The underlying drop implementation for the stored type.
    drop: DropFn,
    /// Conversion to pointer.
    as_ptr: AsPtrFn,
    /// Conversion to mutable pointer.
    as_mut_ptr: AsMutPtrFn,
    /// Pointer to the function used to "take" the inner value.
    /// This can optionally be punted into an implementation which always
    /// returns `None` in case taking is not supported, as it would be with
    /// pointers.
    take_mut_ptr: TakeMutPtrFn,
    /// Type information for diagnostics.
    type_name: TypeNameFn,
    /// The inner type identifier.
    type_id: TypeIdFn,
}

unsafe fn noop_drop_impl<T>(_: *const ()) {
    // noop since we have a wrapped pointer that doesn't need to be
    // dropped.
}

fn as_ptr_impl<T>(this: *const (), expected_type: any::TypeId) -> Option<*const ()>
where
    T: any::Any,
{
    if expected_type == any::TypeId::of::<T>() {
        Some(this)
    } else {
        None
    }
}

fn as_mut_ptr_impl<T>(this: *const (), expected_type: any::TypeId) -> Option<*mut ()>
where
    T: any::Any,
{
    if expected_type == any::TypeId::of::<T>() {
        Some(this as *mut ())
    } else {
        None
    }
}

fn unsupported_as_mut<T>(_: *const (), _: any::TypeId) -> Option<*mut ()>
where
    T: any::Any,
{
    None
}
