//! Trait implementations for Option<T>.

use crate::reflection::{FromValue, ReflectValueType, ToValue, UnsafeFromValue};
use crate::value::{ValuePtr, ValueType, ValueTypeInfo};
use crate::vm::{RawRefGuard, Ref, Vm, VmError};

impl<T> ReflectValueType for Option<T> {
    fn value_type() -> ValueType {
        ValueType::Option
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Option
    }
}

impl<'a, T> ReflectValueType for &'a Option<T> {
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
    fn to_value(self, vm: &mut Vm) -> Result<ValuePtr, VmError> {
        Ok(match self {
            Some(s) => {
                let value = s.to_value(vm)?;
                let slot = vm.slot_allocate::<Option<ValuePtr>>(Some(value));
                ValuePtr::Option(slot)
            }
            None => {
                let slot = vm.slot_allocate::<Option<ValuePtr>>(None);
                ValuePtr::Option(slot)
            }
        })
    }
}

impl<T> FromValue for Option<T>
where
    T: FromValue,
{
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, VmError> {
        match value {
            ValuePtr::Option(slot) => {
                let option = vm.external_take::<Option<ValuePtr>>(slot)?;

                Ok(match option {
                    Some(some) => Some(T::from_value(some, vm)?),
                    None => None,
                })
            }
            actual => Err(VmError::ExpectedOption {
                actual: actual.type_info(vm)?,
            }),
        }
    }
}

impl<'a> UnsafeFromValue for &'a Option<ValuePtr> {
    type Output = *const Option<ValuePtr>;
    type Guard = RawRefGuard;

    unsafe fn unsafe_from_value(
        value: ValuePtr,
        vm: &mut Vm,
    ) -> Result<(Self::Output, Self::Guard), VmError> {
        let slot = value.into_option(vm)?;
        let result = vm.external_ref::<Option<ValuePtr>>(slot)?;
        Ok(Ref::unsafe_into_ref(result))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}
