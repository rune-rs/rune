use std::any::{Any, TypeId};
use std::fmt;

/// Trait for external types stored in the VM.
pub trait External: Any + Send + Sync + fmt::Debug + private::Sealed {
    /// Coerce external into any.
    fn as_any(&self) -> &dyn Any;

    /// Coerce external into mutable any.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Coerce this external into a mutable pointer iff it matches the expected
    /// type.
    fn as_mut_ptr(&mut self, expected_type: TypeId) -> Option<*mut ()>;
}

impl<T> External for T
where
    T: Any + Send + Sync + fmt::Debug,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn as_mut_ptr(&mut self, expected_type: TypeId) -> Option<*mut ()> {
        if expected_type == TypeId::of::<T>() {
            Some(self as *mut _ as *mut ())
        } else {
            None
        }
    }
}

mod private {
    use std::any::Any;
    use std::fmt;

    /// Trait used to seal the [External][super::External] trait.
    pub trait Sealed {}

    impl<T> Sealed for T where T: Any + Send + Sync + fmt::Debug {}
}
