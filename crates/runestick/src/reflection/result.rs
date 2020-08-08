//! Trait implementations for `Result` types.

use crate::reflection::{FromValue, ReflectValueType, ToValue};
use crate::value::{ValuePtr, ValueType, ValueTypeInfo};
use crate::vm::{Vm, VmError};

impl<T, E> ReflectValueType for Result<T, E> {
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
    fn to_value(self, vm: &mut Vm) -> Result<ValuePtr, VmError> {
        Ok(match self {
            Ok(ok) => {
                let ok = ok.to_value(vm)?;
                let slot = vm.slot_allocate::<Result<ValuePtr, ValuePtr>>(Ok(ok));
                ValuePtr::Result(slot)
            }
            Err(err) => {
                let err = err.to_value(vm)?;
                let slot = vm.slot_allocate::<Result<ValuePtr, ValuePtr>>(Err(err));
                ValuePtr::Result(slot)
            }
        })
    }
}

/// Specialized implementation for directly raising VmError's.
impl<T> ToValue for Result<T, VmError>
where
    T: ToValue,
{
    fn to_value(self, vm: &mut Vm) -> Result<ValuePtr, VmError> {
        match self {
            Ok(ok) => ok.to_value(vm),
            Err(err) => Err(err),
        }
    }
}

impl<T, E> FromValue for Result<T, E>
where
    T: FromValue,
    E: FromValue,
{
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, VmError> {
        match value {
            ValuePtr::Result(slot) => {
                let result = vm.external_take::<Result<ValuePtr, ValuePtr>>(slot)?;

                Ok(match result {
                    Ok(ok) => Ok(T::from_value(ok, vm)?),
                    Err(err) => Err(E::from_value(err, vm)?),
                })
            }
            actual => Err(VmError::ExpectedOption {
                actual: actual.type_info(vm)?,
            }),
        }
    }
}
