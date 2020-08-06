use crate::reflection::{FromValue, ReflectValueType, ToValue, UnsafeFromValue};
use crate::unit::CompilationUnit;
use crate::value::{Array, ValuePtr, ValueType, ValueTypeInfo};
use crate::vm::{RawRefGuard, Ref, Vm, VmError};

impl<T> ReflectValueType for Array<T> {
    fn value_type() -> ValueType {
        ValueType::Array
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Array
    }
}

impl<'a, T> ReflectValueType for &'a Array<T> {
    fn value_type() -> ValueType {
        ValueType::Array
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Array
    }
}

impl<'a, T> ReflectValueType for &'a mut Array<T> {
    fn value_type() -> ValueType {
        ValueType::Array
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Array
    }
}

impl<T> FromValue for Array<T>
where
    T: FromValue,
{
    fn from_value(value: ValuePtr, vm: &mut Vm, unit: &CompilationUnit) -> Result<Self, VmError> {
        let slot = value.into_array(vm)?;
        let array = vm.array_take(slot)?;

        let mut output = Array::with_capacity(array.len());

        for value in array {
            output.push(T::from_value(value, vm, unit)?);
        }

        Ok(output)
    }
}

impl<'a> UnsafeFromValue for &'a Array<ValuePtr> {
    type Guard = RawRefGuard;

    unsafe fn unsafe_from_value(
        value: ValuePtr,
        vm: &mut Vm,
        _: &CompilationUnit,
    ) -> Result<(Self, Self::Guard), VmError> {
        let slot = value.into_array(vm)?;
        Ok(Ref::unsafe_into_ref(vm.array_ref(slot)?))
    }
}

impl<T> ToValue for Array<T>
where
    T: ToValue,
{
    fn to_value(self, vm: &mut Vm) -> Result<ValuePtr, VmError> {
        let mut array = Array::with_capacity(self.len());

        for value in self {
            array.push(value.to_value(vm)?);
        }

        Ok(vm.array_allocate(array))
    }
}
