use crate::access::AccessError;
use std::any;
use std::fmt;

/// A wrapped raw pointer which has associated type information and knows if
/// it's exclusive or shared.
pub struct RawPtr {
    vtable: &'static Vtable,
    data: *const (),
}

impl RawPtr {
    /// Construct a new shared raw pointer.
    ///
    /// This has no immediate safety implications, but future use of the now raw
    /// pointer are unsafe since the data it points to might have been freed.
    pub fn from_ref<T>(data: &T) -> Self
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

    /// Construct a new exclusive raw pointer.
    ///
    /// This has no immediate safety implications, but future use of the now raw
    /// pointer are unsafe since the data it points to might have been freed.
    pub fn from_mut<T>(data: &mut T) -> Self
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
    ///
    /// # Safety
    ///
    /// The validity of the pointer can only be relied on during the running of
    /// the virtual machine.
    /// At other times, the caller is responsible for making sure that the
    /// pointee is alive.
    pub unsafe fn downcast_borrow_ref<T>(&self) -> Result<*const T, AccessError>
    where
        T: any::Any,
    {
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

    /// Get the exclusive cell as an exclusive reference.
    ///
    /// # Safety
    ///
    /// The validity of the pointer can only be relied on during the running of
    /// the virtual machine.
    /// At other times, the caller is responsible for making sure that the
    /// pointee is alive.
    pub unsafe fn downcast_borrow_mut<T>(&self) -> Result<*mut T, AccessError>
    where
        T: any::Any,
    {
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

impl fmt::Debug for RawPtr {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "RawPtr({})", self.type_name())
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
