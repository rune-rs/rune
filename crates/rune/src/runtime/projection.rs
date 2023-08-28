use core::ptr;

use crate::runtime::{FullTypeOf, MaybeTypeOf, RawMut, RawRef, TypeInfo, VmResult};

use crate::{AnyRef, Hash, TypeHash, Value};

use super::{TypeOf, UnsafeToMut, UnsafeToRef};

/// A type that can be accessed by reference, where both the reference and
/// lifetime of the type are constrained by the caller. This is necessary for
/// external types containing lifetimes to be accessed in a safe manner.
pub trait Projectable {
    /// The type to be returned.
    type Item<'a>
    where
        Self: 'a;

    /// Get a reference to Self with a constrained lifetime.
    fn get<'a>(&'a self) -> &'a Self::Item<'a>;

    /// Get a mutable reference to Self with a constrained lifetime.
    fn get_mut<'a>(&'a mut self) -> &'a mut Self::Item<'a>;
}

/// A pointer to an external type which holds references to other external
/// types.
pub struct Projection<T: AnyRef>(ptr::NonNull<T>);

impl<T: AnyRef> Projection<T> {
    pub(crate) fn new(value: &T) -> Self {
        Self(unsafe { ptr::NonNull::new_unchecked(value as *const _ as *mut _) })
    }
}

impl<T: AnyRef> Projectable for Projection<T> {
    type Item<'a> = T
    where
        T: 'a;

    fn get<'a>(&'a self) -> &'a Self::Item<'a> {
        unsafe { self.0.as_ref() }
    }

    fn get_mut<'a>(&'a mut self) -> &'a mut Self::Item<'a> {
        unsafe { self.0.as_mut() }
    }
}

impl<T: AnyRef> MaybeTypeOf for Projection<T> {
    fn maybe_type_of() -> Option<FullTypeOf> {
        None
    }
}

impl<T: AnyRef> TypeOf for Projection<T> {
    fn type_hash() -> Hash {
        <T as TypeHash>::type_hash()
    }

    fn type_info() -> TypeInfo {
        T::type_info()
    }
}

impl<T: AnyRef> TypeHash for Projection<T> {
    fn type_hash() -> Hash {
        <T as TypeHash>::type_hash()
    }
}

impl<T: AnyRef> UnsafeToRef for Projection<T> {
    type Guard = RawRef;

    unsafe fn unsafe_to_ref<'a>(value: Value) -> VmResult<(&'a Self, Self::Guard)> {
        let (ptr, guard) = vm_try!(value.into_projection());
        VmResult::Ok((ptr.as_ref(), guard))
    }
}

impl<T: AnyRef> UnsafeToMut for Projection<T> {
    type Guard = RawMut;

    unsafe fn unsafe_to_mut<'a>(value: Value) -> VmResult<(&'a mut Self, Self::Guard)> {
        let (mut ptr, guard) = vm_try!(value.into_projection_mut());
        VmResult::Ok((ptr.as_mut(), guard))
    }
}
