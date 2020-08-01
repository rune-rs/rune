use std::any;
use std::fmt;

/// Trait for types stored in the VM.
pub trait Any: any::Any + Send + Sync + fmt::Debug + private::Sealed {
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

mod private {
    use std::any;
    use std::fmt;

    /// Trait used to seal the [AnyExt][super::AnyExt] trait.
    pub trait Sealed {}

    impl<T> Sealed for T where T: any::Any + Send + Sync + fmt::Debug {}
}
