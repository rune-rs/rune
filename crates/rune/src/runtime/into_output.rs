use core::convert::Infallible;

use crate::alloc;
use crate::runtime::{AnySequence, Inline, Rtti, Value};
use crate::sync::Arc;

pub(crate) mod sealed {
    use crate::runtime::{AnySequence, Inline, Rtti, Value};
    use crate::sync::Arc;

    use super::IntoOutput;

    /// Sealed trait to prevent external implementations.
    pub trait Sealed {}

    impl<F, O> Sealed for F
    where
        F: FnOnce() -> O,
        O: IntoOutput,
    {
    }

    impl<T, E> Sealed for Result<T, E>
    where
        T: IntoOutput,
        E: From<T::Error>,
    {
    }

    impl Sealed for () {}
    impl Sealed for Value {}
    impl Sealed for Inline {}
    impl Sealed for AnySequence<Arc<Rtti>, Value> {}
    impl Sealed for &[u8] {}
    impl Sealed for &str {}
}

/// Trait used to coerce values into outputs.
pub trait IntoOutput: self::sealed::Sealed {
    /// The error type produced by output coercion.
    type Error;

    /// Coerce the current value into an output.
    fn into_output(self) -> Result<Value, Self::Error>;
}

impl<F, O> IntoOutput for F
where
    F: FnOnce() -> O,
    O: IntoOutput,
{
    type Error = O::Error;

    #[inline]
    fn into_output(self) -> Result<Value, Self::Error> {
        self().into_output()
    }
}

impl<T, E> IntoOutput for Result<T, E>
where
    T: IntoOutput,
    E: From<T::Error>,
{
    type Error = E;

    #[inline]
    fn into_output(self) -> Result<Value, Self::Error> {
        Ok(self?.into_output()?)
    }
}

impl IntoOutput for Value {
    type Error = Infallible;

    #[inline]
    fn into_output(self) -> Result<Value, Self::Error> {
        Ok(self)
    }
}

impl IntoOutput for Inline {
    type Error = Infallible;

    #[inline]
    fn into_output(self) -> Result<Value, Self::Error> {
        Ok(Value::from(self))
    }
}

impl IntoOutput for AnySequence<Arc<Rtti>, Value> {
    type Error = Infallible;

    #[inline]
    fn into_output(self) -> Result<Value, Self::Error> {
        Ok(Value::from(self))
    }
}

impl IntoOutput for &[u8] {
    type Error = alloc::Error;

    #[inline]
    fn into_output(self) -> Result<Value, Self::Error> {
        Value::try_from(self)
    }
}

impl IntoOutput for &str {
    type Error = alloc::Error;

    #[inline]
    fn into_output(self) -> alloc::Result<Value> {
        Value::try_from(self)
    }
}

impl IntoOutput for () {
    type Error = Infallible;

    #[inline]
    fn into_output(self) -> Result<Value, Self::Error> {
        Ok(Value::from(()))
    }
}
