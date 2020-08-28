use crate::access::AccessError;
use std::any;
use std::fmt;

/// A container for a value which is solely being checked for access.
pub struct SharedPtr {
    vtable: &'static Vtable,
    data: *const (),
}

impl SharedPtr {
    /// Construct a new any from a pointer.
    ///
    /// # Safety
    ///
    /// It is up to the caller to make sure that whatever data is pointed to is
    /// valid for the duration of the shared pointer.
    pub unsafe fn from_ptr<T>(data: &T) -> Self
    where
        T: any::Any,
    {
        Self {
            vtable: &Vtable {
                as_ptr: as_ptr_impl::<T>,
                as_mut_ptr: unsupported_as_mut::<T>,
                type_name: any::type_name::<&T>,
                type_id: any::TypeId::of::<T>,
            },
            data: data as *const T as *const (),
        }
    }

    /// Construct a new any from a exclusive pointer.
    ///
    /// # Safety
    ///
    /// It is up to the caller to make sure that whatever data is pointed to is
    /// valid for the duration of the shared pointer.
    pub unsafe fn from_mut_ptr<T>(data: &mut T) -> Self
    where
        T: any::Any,
    {
        Self {
            vtable: &Vtable {
                as_ptr: as_ptr_impl::<T>,
                as_mut_ptr: as_mut_ptr_impl::<T>,
                type_name: any::type_name::<&mut T>,
                type_id: any::TypeId::of::<T>,
            },
            data: data as *mut T as *const T as *const (),
        }
    }

    /// Get the type name contained in the shared ptr.
    pub fn type_name(&self) -> &'static str {
        (self.vtable.type_name)()
    }

    /// Get the type id contained in the shared ptr.
    pub fn type_id(&self) -> any::TypeId {
        (self.vtable.type_id)()
    }

    /// Get the shared cell as a reference.
    pub fn downcast_borrow_ref<T>(&self) -> Result<*const T, AccessError>
    where
        T: any::Any,
    {
        unsafe {
            let result = (self.vtable.as_ptr)(self.data, any::TypeId::of::<T>());

            let data = match result {
                Some(data) => data,
                None => {
                    return Err(AccessError::UnexpectedType {
                        expected: any::type_name::<T>(),
                        actual: self.type_name(),
                    });
                }
            };

            Ok(data as *const T)
        }
    }

    /// Get the exclusive cell as an exclusive reference.
    pub fn downcast_borrow_mut<T>(&self) -> Result<*mut T, AccessError>
    where
        T: any::Any,
    {
        unsafe {
            let result = (self.vtable.as_mut_ptr)(self.data, any::TypeId::of::<T>());

            let data = match result {
                Some(data) => data,
                None => {
                    return Err(AccessError::UnexpectedType {
                        expected: any::type_name::<T>(),
                        actual: self.type_name(),
                    });
                }
            };

            Ok(data as *mut T)
        }
    }
}

type AsPtrFn = unsafe fn(*const (), expected: any::TypeId) -> Option<*const ()>;
type AsMutPtrFn = unsafe fn(*const (), expected: any::TypeId) -> Option<*mut ()>;
type TypeNameFn = fn() -> &'static str;
type TypeIdFn = fn() -> any::TypeId;

/// The vtable for any reference type stored in the virtual machine.
///
/// We rely _heavily_ on the invariants provided by `std::any::Any` which are
/// checked at construction-time for this type.
#[repr(C)]
struct Vtable {
    /// Conversion to pointer.
    as_ptr: AsPtrFn,
    /// Conversion to mutable pointer.
    as_mut_ptr: AsMutPtrFn,
    /// Type information for diagnostics.
    type_name: TypeNameFn,
    /// The inner type identifier.
    type_id: TypeIdFn,
}

impl fmt::Debug for SharedPtr {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "SharedPtr({})", self.type_name())
    }
}

fn as_ptr_impl<T>(this: *const (), expected: any::TypeId) -> Option<*const ()>
where
    T: any::Any,
{
    if expected == any::TypeId::of::<T>() {
        Some(this)
    } else {
        None
    }
}

fn as_mut_ptr_impl<T>(this: *const (), expected: any::TypeId) -> Option<*mut ()>
where
    T: any::Any,
{
    if expected == any::TypeId::of::<T>() {
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
