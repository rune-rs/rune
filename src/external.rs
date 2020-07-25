use std::any::Any;
use std::fmt;

/// Trait for external types stored in the VM.
pub trait External: Any + Send + Sync + fmt::Debug + private::Sealed {
    fn as_any(&self) -> &dyn Any;
}

impl<T> External for T
where
    T: Any + Send + Sync + fmt::Debug + Clone,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}

mod private {
    use std::any::Any;
    use std::fmt;

    /// Trait used to seal the [External][super::External] trait.
    pub trait Sealed {}

    impl<T> Sealed for T where T: Any + Send + Sync + fmt::Debug + Clone {}
}
