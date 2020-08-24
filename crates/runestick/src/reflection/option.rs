//! Trait implementations for Option<T>.

use crate::reflection::{FromValue, ReflectValueType, ToValue, UnsafeFromValue};
use crate::shared::{RawStrongRefGuard, Shared, StrongRef};
use crate::value::{Value, ValueType, ValueTypeInfo};
use crate::vm::VmError;

impl<T> ReflectValueType for Option<T> {
    type Owned = Option<T>;

    fn value_type() -> ValueType {
        ValueType::Option
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Option
    }
}

impl<'a, T> ReflectValueType for &'a Option<T> {
    type Owned = Option<T>;

    fn value_type() -> ValueType {
        ValueType::Option
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Option
    }
}

impl<T> ToValue for Option<T>
where
    T: ToValue,
{
    fn to_value(self) -> Result<Value, VmError> {
        Ok(Value::Option(Shared::new(match self {
            Some(some) => {
                let value = some.to_value()?;
                Some(value)
            }
            None => None,
        })))
    }
}

impl<T> FromValue for Option<T>
where
    T: FromValue,
{
    fn from_value(value: Value) -> Result<Self, VmError> {
        match value {
            Value::Option(option) => Ok(match option.take()? {
                Some(some) => Some(T::from_value(some)?),
                None => None,
            }),
            actual => Err(VmError::ExpectedOption {
                actual: actual.type_info()?,
            }),
        }
    }
}

impl<'a> UnsafeFromValue for &'a Option<Value> {
    type Output = *const Option<Value>;
    type Guard = RawStrongRefGuard;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let option = value.into_option()?;
        Ok(StrongRef::into_raw(option.strong_ref()?))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}
