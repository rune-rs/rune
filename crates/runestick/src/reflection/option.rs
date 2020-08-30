//! Trait implementations for Option<T>.

use crate::{
    FromValue, OwnedRef, RawOwnedRef, ReflectValueType, Shared, ToValue, UnsafeFromValue, Value,
    ValueError, ValueType, ValueTypeInfo,
};

impl<T> ReflectValueType for Option<T> {
    type Owned = Option<T>;

    fn value_type() -> ValueType {
        ValueType::Option
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Option
    }
}

impl<T> ReflectValueType for &Option<T> {
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
    fn to_value(self) -> Result<Value, ValueError> {
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
    fn from_value(value: Value) -> Result<Self, ValueError> {
        Ok(match value.into_option()?.take()? {
            Some(some) => Some(T::from_value(some)?),
            None => None,
        })
    }
}

impl UnsafeFromValue for &Option<Value> {
    type Output = *const Option<Value>;
    type Guard = RawOwnedRef;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), ValueError> {
        let option = value.into_option()?;
        Ok(OwnedRef::into_raw(option.owned_ref()?))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}
