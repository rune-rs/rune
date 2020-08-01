use std::any;
use std::fmt;

/// Trait for types stored in the VM.
///
/// We extend [std::any::Any] to assert that they implement [Debug].
pub trait Any: 'static + any::Any + Send + Sync + fmt::Debug + private::Sealed {
    /// Coerce into a reference Any.
    fn as_any(&self) -> &dyn any::Any;

    /// Coerce into a mutable Any.
    fn as_any_mut(&mut self) -> &mut dyn any::Any;

    /// Coerce this external into a pointer iff it matches the expected type.
    fn as_ptr(&self, expected_type: any::TypeId) -> Option<*const ()>;

    /// Coerce this external into a mutable pointer iff it matches the expected
    /// type.
    fn as_mut_ptr(&mut self, expected_type: any::TypeId) -> Option<*mut ()>;
}

impl<T> Any for T
where
    T: any::Any + Send + Sync + fmt::Debug,
{
    fn as_any(&self) -> &dyn any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn any::Any {
        self
    }

    fn as_ptr(&self, expected_type: any::TypeId) -> Option<*const ()> {
        if expected_type == any::TypeId::of::<T>() {
            Some(self as *const _ as *const ())
        } else {
            None
        }
    }

    fn as_mut_ptr(&mut self, expected_type: any::TypeId) -> Option<*mut ()> {
        if expected_type == any::TypeId::of::<T>() {
            Some(self as *mut _ as *mut ())
        } else {
            None
        }
    }
}

/// Forward implementations to [std::any::Any] variants for convenience.
impl dyn Any {
    pub fn is<T: any::Any>(&self) -> bool {
        any::Any::is::<T>(self.as_any())
    }

    pub fn downcast_ref<T: any::Any>(&self) -> Option<&T> {
        any::Any::downcast_ref::<T>(self.as_any())
    }

    pub fn downcast_mut<T: any::Any>(&mut self) -> Option<&mut T> {
        any::Any::downcast_mut::<T>(self.as_any_mut())
    }
}

mod private {
    use std::any;
    use std::fmt;

    /// Trait used to seal the [AnyExt][super::AnyExt] trait.
    pub trait Sealed {}

    impl<T> Sealed for T where T: any::Any + Send + Sync + fmt::Debug {}
}
