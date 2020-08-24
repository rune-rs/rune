//! Trait implementations for `Result` types.

use crate::reflection::{FromValue, ReflectValueType, ToValue, UnsafeFromValue};
use crate::shared::{RawStrongRefGuard, Shared, StrongRef};
use crate::value::{Value, ValueType, ValueTypeInfo};
use crate::vm::VmError;

impl<T, E> ReflectValueType for Result<T, E> {
    type Owned = Result<T, E>;

    fn value_type() -> ValueType {
        ValueType::Result
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Result
    }
}

impl<'a, T, E> ReflectValueType for &'a Result<T, E> {
    type Owned = Result<T, E>;

    fn value_type() -> ValueType {
        ValueType::Result
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Result
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

/// Specialized implementation for directly raising VmError's.
impl<T> ToValue for Result<T, VmError>
where
    T: ToValue,
{
    fn to_value(self) -> Result<Value, VmError> {
        match self {
            Ok(ok) => Ok(Value::Result(Shared::new(Ok(ok.to_value()?)))),
            Err(err) => Err(err),
        }
    }
}

impl<T, E> FromValue for Result<T, E>
where
    T: FromValue,
    E: FromValue,
{
    fn from_value(value: Value) -> Result<Self, VmError> {
        match value {
            Value::Result(result) => Ok(match result.take()? {
                Ok(ok) => Ok(T::from_value(ok)?),
                Err(err) => Err(E::from_value(err)?),
            }),
            actual => Err(VmError::ExpectedOption {
                actual: actual.type_info()?,
            }),
        }
    }
}

impl<'a> UnsafeFromValue for &'a Result<Value, Value> {
    type Output = *const Result<Value, Value>;
    type Guard = RawStrongRefGuard;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let result = value.into_result()?;
        let result = result.strong_ref()?;
        Ok(StrongRef::into_raw(result))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}
