//! Trait implementations for Option<T>.

use crate::reflection::{FromValue, ReflectValueType, ToValue};
use crate::value::{ValuePtr, ValueType, ValueTypeInfo};
use crate::vm::{Vm, VmError};

impl<T> ReflectValueType for Option<T>
where
    T: ReflectValueType,
{
    fn value_type() -> ValueType {
        T::value_type()
    }

    fn value_type_info() -> ValueTypeInfo {
        T::value_type_info()
    }
}

impl<T> ToValue for Option<T>
where
    T: ToValue,
{
    fn to_value(self, vm: &mut Vm) -> Result<ValuePtr, VmError> {
        match self {
            Some(s) => s.to_value(vm),
            None => Ok(ValuePtr::None),
        }
    }
}

impl<T> FromValue for Option<T>
where
    T: FromValue,
{
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, VmError> {
        match value {
            ValuePtr::None => Ok(None),
            _ => Ok(Some(T::from_value(value, vm)?)),
        }
    }
}
