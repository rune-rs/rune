//! Trait implementations for `Result` types.

use crate::{
    FromValue, OwnedRef, Panic, RawOwnedRef, ReflectValueType, Shared, ToValue, UnsafeFromValue,
    Value, ValueError, ValueType, ValueTypeInfo,
};
use std::fmt;
use std::io;

decl_external!(fmt::Error);
decl_external!(io::Error);

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

impl<T> ToValue for Result<T, Panic>
where
    T: ToValue,
{
    fn to_value(self) -> Result<Value, ValueError> {
        match self {
            Ok(ok) => {
                let ok = ok.to_value()?;
                Ok(Value::Result(Shared::new(Ok(ok))))
            }
            Err(reason) => Err(ValueError::Panic { reason }),
        }
    }
}

impl<T, E> ToValue for Result<T, E>
where
    T: ToValue,
    E: ToValue,
{
    fn to_value(self) -> Result<Value, ValueError> {
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
    fn from_value(value: Value) -> Result<Self, ValueError> {
        Ok(match value.into_result()?.take()? {
            Ok(ok) => Ok(T::from_value(ok)?),
            Err(err) => Err(E::from_value(err)?),
        })
    }
}

impl<'a> UnsafeFromValue for &'a Result<Value, Value> {
    type Output = *const Result<Value, Value>;
    type Guard = RawOwnedRef;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), ValueError> {
        let result = value.into_result()?;
        let result = result.owned_ref()?;
        Ok(OwnedRef::into_raw(result))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}
