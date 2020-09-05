//! Trait implementations for `Result` types.

use crate::{
    FromValue, OwnedRef, Panic, RawOwnedRef, Shared, ToValue, UnsafeFromValue, Value, VmError,
    VmErrorKind,
};
use std::fmt;
use std::io;

impl_external!(fmt::Error);
impl_external!(io::Error);

impl<T> ToValue for Result<T, Panic>
where
    T: ToValue,
{
    fn to_value(self) -> Result<Value, VmError> {
        match self {
            Ok(value) => Ok(value.to_value()?),
            Err(reason) => Err(VmError::from(VmErrorKind::Panic { reason })),
        }
    }
}

impl<T> ToValue for Result<T, VmError>
where
    T: ToValue,
{
    fn to_value(self) -> Result<Value, VmError> {
        match self {
            Ok(value) => Ok(value.to_value()?),
            Err(error) => Err(error),
        }
    }
}

impl<T, E> ToValue for Result<T, E>
where
    T: ToValue,
    E: ToValue,
{
    fn to_value(self) -> Result<Value, VmError> {
        Ok(match self {
            Ok(ok) => {
                let ok = ok.to_value()?;
                Value::Result(Shared::new(Ok(ok)))
            }
            Err(err) => {
                let err = err.to_value()?;
                Value::Result(Shared::new(Err(err)))
            }
        })
    }
}

impl<T, E> FromValue for Result<T, E>
where
    T: FromValue,
    E: FromValue,
{
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(match value.into_result()?.take()? {
            Ok(ok) => Ok(T::from_value(ok)?),
            Err(err) => Err(E::from_value(err)?),
        })
    }
}

impl UnsafeFromValue for &Result<Value, Value> {
    type Output = *const Result<Value, Value>;
    type Guard = RawOwnedRef;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let result = value.into_result()?;
        let result = result.owned_ref()?;
        Ok(OwnedRef::into_raw(result))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}
