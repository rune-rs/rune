//! Helper types for a holder of data.

use core::any::TypeId;
use core::fmt;
use core::mem::ManuallyDrop;
use core::ptr;

use crate::alloc::alloc::Global;
use crate::alloc::{self, Box};
use crate::any::Any;
use crate::compile::meta;
use crate::hash::Hash;
use crate::runtime::{AnyTypeInfo, MaybeTypeOf, RawStr, TypeInfo};

/// Our own private dynamic Any implementation.
///
/// In contrast to `Box<dyn std::any::Any>`, this allows for storing a raw
/// pointer directly in the object to avoid one level of indirection. Otherwise
/// it's equivalent.
#[repr(C)]
pub struct AnyObj {
    data: ptr::NonNull<()>,
    vtable: &'static Vtable,
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
            data,
            vtable: &Vtable {
                drop: drop_impl::<T>,
                type_id: TypeId::of::<T>,
                debug: debug_impl::<T>,
                type_name: type_name_impl::<T>,
                type_hash: type_hash_impl::<T>,
            },
        })
    }

    /// Returns stored value if it is of type `T`, or `None` if it isn't.
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
    /// let any = AnyObj::new(Thing(1u32))?;
    ///
    /// let Ok(thing) = any.downcast::<Thing>() else {
    ///     panic!("Conversion failed");
    /// };
    ///
    /// assert_eq!(thing, Thing(1));
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn downcast<T>(self) -> Result<T, Self>
    where
        T: Any,
    {
        let this = ManuallyDrop::new(self);

        unsafe {
            if (this.vtable.type_id)() != TypeId::of::<T>() {
                let this = ManuallyDrop::into_inner(this);
                return Err(this);
            };

            Ok(Box::into_inner(Box::from_raw_in(
                this.data.cast::<T>().as_ptr(),
                rune_alloc::alloc::Global,
            )))
        }
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
    /// assert_eq!(Some(&Thing(1u32)), any.downcast_ref::<Thing>());
    /// assert_eq!(None, any.downcast_ref::<Other>());
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: Any,
    {
        unsafe {
            if (self.vtable.type_id)() != TypeId::of::<T>() {
                return None;
            }

            Some(self.data.cast::<T>().as_ref())
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
    /// any.downcast_mut::<Thing>().unwrap().0 = 2;
    /// assert_eq!(Some(&Thing(2u32)), any.downcast_ref::<Thing>());
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn downcast_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Any,
    {
        unsafe {
            if (self.vtable.type_id)() != TypeId::of::<T>() {
                return None;
            }

            Some(self.data.cast::<T>().as_mut())
        }
    }

    /// Debug format the current any type.
    pub(crate) fn debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.vtable.debug)(f)
    }

    /// Access the underlying type name for the data.
    pub(crate) fn type_name(&self) -> RawStr {
        (self.vtable.type_name)()
    }

    /// Access the underlying type id for the data.
    pub(crate) fn type_hash(&self) -> Hash {
        (self.vtable.type_hash)()
    }

    /// Access full type info for type.
    pub(crate) fn type_info(&self) -> TypeInfo {
        TypeInfo::Any(AnyTypeInfo::__private_new(
            (self.vtable.type_name)(),
            (self.vtable.type_hash)(),
        ))
    }
}

impl MaybeTypeOf for AnyObj {
    #[inline]
    fn maybe_type_of() -> alloc::Result<meta::DocType> {
        Ok(meta::DocType::empty())
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
type DropFn = unsafe fn(*mut ());

/// The signature of a pointer coercion function.
type TypeIdFn = fn() -> TypeId;

/// The signature of a descriptive type name function.
type DebugFn = fn(&mut fmt::Formatter<'_>) -> fmt::Result;

/// Get the type name.
type TypeNameFn = fn() -> RawStr;

/// The signature of a type hash function.
type TypeHashFn = fn() -> Hash;

/// The vtable for any type stored in the virtual machine.
///
/// This can be implemented manually assuming it obeys the constraints of the
/// type. Otherwise we rely _heavily_ on the invariants provided by
/// `std::any::Any` which are checked at construction-time for this type.
struct Vtable {
    /// The underlying drop implementation for the stored type.
    drop: DropFn,
    /// Punt the inner pointer to the type corresponding to the type hash.
    type_id: TypeIdFn,
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

fn debug_impl<T>(f: &mut fmt::Formatter<'_>) -> fmt::Result
where
    T: Any,
{
    write!(f, "{}", T::BASE_NAME)
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
