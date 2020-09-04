//! Trait implementations for Option<T>.

use crate::{
    FromValue, OwnedMut, OwnedRef, RawOwnedMut, RawOwnedRef, ReflectValueType, Shared, ToValue,
    UnsafeFromValue, Value, ValueType, ValueTypeInfo, VmError,
};

impl<T> ReflectValueType for Option<T> {
    type Owned = Option<T>;

    fn value_type() -> ValueType {
        ValueType::StaticType(crate::OPTION_TYPE)
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::StaticType(crate::OPTION_TYPE)
    }
}

impl<T> ReflectValueType for &Option<T> {
    type Owned = Option<T>;

    fn value_type() -> ValueType {
        ValueType::StaticType(crate::OPTION_TYPE)
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::StaticType(crate::OPTION_TYPE)
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
        Ok(match value.into_option()?.take()? {
            Some(some) => Some(T::from_value(some)?),
            None => None,
        })
    }
}

impl UnsafeFromValue for &Option<Value> {
    type Output = *const Option<Value>;
    type Guard = RawOwnedRef;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let option = value.into_option()?;
        Ok(OwnedRef::into_raw(option.owned_ref()?))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl UnsafeFromValue for &mut Option<Value> {
    type Output = *mut Option<Value>;
    type Guard = RawOwnedMut;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let option = value.into_option()?;
        Ok(OwnedMut::into_raw(option.owned_mut()?))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &mut *output
    }
}
