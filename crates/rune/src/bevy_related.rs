use core::any::TypeId;
use core::fmt;

use bevy::ecs::change_detection::MutUntyped;
use bevy::prelude::Mut;

use crate::alloc::alloc::Global;
use crate::alloc::{self, Box};
use crate::any::Any;
use crate::compile::Named;
use crate::hash::Hash;
use crate::runtime::{
    AnyObj, AnyObjKind, AnyObjVtable, RawStr, Shared, SharedPointerGuard, Value, VmResult,
};

/// Unsafely convert a bevy mutable reference into a value and a guard.
///
/// # Safety
///
/// The value returned must not be used after the guard associated with it has
/// been dropped.
pub unsafe fn bevy_mut_to_value<T>(this: Mut<'_, T>) -> VmResult<(Value, SharedPointerGuard)>
where
    T: Any,
{
    let (shared, guard) = vm_try!(from_bevy_mut(this));
    VmResult::Ok((Value::from(shared), guard))
}

/// Construct a `Shared<Any>` from a bevy specific mutable pointer that does change detection, this will be "taken"
/// once the returned guard is dropped.
///
/// # Safety
///
/// The reference must be valid for the duration of the guard.
///
/// # Examples
///
unsafe fn from_bevy_mut<T>(
    data: bevy::prelude::Mut<'_, T>,
) -> alloc::Result<(Shared<AnyObj>, SharedPointerGuard)>
where
    T: Any,
{
    Shared::unsafe_from_any_pointer(anyobj_from_bevy_mut(data))
}

/// Construct an Any that wraps a bevy specific mutable pointer that does change
/// detection.
///
/// # Safety
///
/// Caller must ensure that the returned `AnyObj` doesn't outlive the reference
/// it is wrapping.
unsafe fn anyobj_from_bevy_mut<T>(data: Mut<'_, T>) -> AnyObj
where
    T: Any,
{
    let untyped = MutUntyped::from(data);
    let (ptr, _) = Box::into_raw_with_allocator(Box::try_new(untyped).unwrap());
    let data = ptr as *const _ as *const ();

    let vtable = &AnyObjVtable {
        kind: AnyObjKind::MutPtr,
        drop: bevy_mut_drop::<T>,
        as_ptr: as_bevy_ptr_impl::<T>,
        as_ptr_mut: as_bevy_ptr_mut_impl::<T>,
        debug: debug_mut_impl::<T>,
        type_name: type_name_impl::<T>,
        type_hash: type_hash_impl::<T>,
    };

    AnyObj::new_raw(vtable, data)
}

fn bevy_mut_drop<T>(this: *mut ()) {
    unsafe {
        drop(Box::from_raw_in(this as *mut MutUntyped<'static>, Global));
    }
}

fn as_bevy_ptr_impl<T>(this: *const (), expected: TypeId) -> Option<*const ()>
where
    T: ?Sized + 'static,
{
    if expected == TypeId::of::<T>() {
        unsafe {
            let this = this as *const () as *const MutUntyped<'static>;
            Some((*this).as_ref().as_ptr() as *const _ as *const ())
        }
    } else {
        None
    }
}

fn as_bevy_ptr_mut_impl<T>(this: *mut (), expected: TypeId) -> Option<*mut ()>
where
    T: ?Sized + 'static,
{
    if expected == TypeId::of::<T>() {
        unsafe {
            let this = this as *mut () as *mut MutUntyped<'static>;
            // NB: `as_mut` calls `set_changed`.
            Some((*this).as_mut().as_ptr() as *mut ())
        }
    } else {
        None
    }
}

fn debug_mut_impl<T>(f: &mut fmt::Formatter<'_>) -> fmt::Result
where
    T: ?Sized + Named,
{
    write!(f, "&mut {}", T::BASE_NAME)
}

fn type_name_impl<T>() -> RawStr
where
    T: ?Sized + Named,
{
    T::BASE_NAME
}

fn type_hash_impl<T>() -> Hash
where
    T: ?Sized + Any,
{
    T::type_hash()
}
