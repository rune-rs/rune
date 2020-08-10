use crate::reflection::{FromValue, ReflectValueType, ToValue, UnsafeFromValue};
use crate::value::{Value, ValueType, ValueTypeInfo};
use crate::vm::{RawRefGuard, Ref, Vm, VmError};

impl<T> ReflectValueType for Vec<T> {
    type Owned = Vec<T>;

    fn value_type() -> ValueType {
        ValueType::Vec
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Vec
    }
}

impl<'a, T> ReflectValueType for &'a Vec<T> {
    type Owned = Vec<T>;

    fn value_type() -> ValueType {
        ValueType::Vec
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Vec
    }
}

impl<'a, T> ReflectValueType for &'a mut Vec<T> {
    type Owned = Vec<T>;

    fn value_type() -> ValueType {
        ValueType::Vec
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Vec
    }
}

impl<T> FromValue for Vec<T>
where
    T: FromValue,
{
    fn from_value(value: Value, vm: &mut Vm) -> Result<Self, VmError> {
        let slot = value.into_vec(vm)?;
        let vec = vm.vec_take(slot)?;

        let mut output = Vec::with_capacity(vec.len());

        for value in vec {
            output.push(T::from_value(value, vm)?);
        }

        Ok(output)
    }
}

impl<'a> UnsafeFromValue for &'a Vec<Value> {
    type Output = *const Vec<Value>;
    type Guard = RawRefGuard;

    unsafe fn unsafe_from_value(
        value: Value,
        vm: &mut Vm,
    ) -> Result<(Self::Output, Self::Guard), VmError> {
        let slot = value.into_vec(vm)?;
        Ok(Ref::unsafe_into_ref(vm.vec_ref(slot)?))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl<T> ToValue for Vec<T>
where
    T: ToValue,
{
    fn to_value(self, vm: &mut Vm) -> Result<Value, VmError> {
        let mut vec = Vec::with_capacity(self.len());

        for value in self {
            vec.push(value.to_value(vm)?);
        }

        Ok(vm.vec_allocate(vec))
    }
}
