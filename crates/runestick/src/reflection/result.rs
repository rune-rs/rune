//! Trait implementations for `Result` types.

use crate::reflection::{FromValue, ReflectValueType, ToValue, UnsafeFromValue};
use crate::value::{Value, ValueType, ValueTypeInfo};
use crate::vm::{RawRefGuard, Ref, Vm, VmError};

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
    fn to_value(self, vm: &mut Vm) -> Result<Value, VmError> {
        Ok(match self {
            Ok(ok) => {
                let ok = ok.to_value(vm)?;
                vm.result_allocate(Ok(ok))
            }
            Err(err) => {
                let err = err.to_value(vm)?;
                vm.result_allocate(Err(err))
            }
        })
    }
}

/// Specialized implementation for directly raising VmError's.
impl<T> ToValue for Result<T, VmError>
where
    T: ToValue,
{
    fn to_value(self, vm: &mut Vm) -> Result<Value, VmError> {
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
    fn from_value(value: Value, vm: &mut Vm) -> Result<Self, VmError> {
        match value {
            Value::Result(slot) => {
                let result = vm.result_take(slot)?;

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

impl<'a> UnsafeFromValue for &'a Result<Value, Value> {
    type Output = *const Result<Value, Value>;
    type Guard = RawRefGuard;

    unsafe fn unsafe_from_value(
        value: Value,
        vm: &mut Vm,
    ) -> Result<(Self::Output, Self::Guard), VmError> {
        let slot = value.into_result(vm)?;
        let result = vm.result_ref(slot)?;
        Ok(Ref::unsafe_into_ref(result))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}
