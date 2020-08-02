use crate::reflection::{FromValue, ReflectValueType, ToValue, UnsafeFromValue};
use crate::value::{Object, ValuePtr, ValueType, ValueTypeInfo};
use crate::vm::{RawRefGuard, Ref, StackError, Vm};

impl<T> ReflectValueType for Object<T> {
    fn value_type() -> ValueType {
        ValueType::Array
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Array
    }
}

impl<'a, T> ReflectValueType for &'a Object<T> {
    fn value_type() -> ValueType {
        ValueType::Array
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Array
    }
}

impl<'a, T> ReflectValueType for &'a mut Object<T> {
    fn value_type() -> ValueType {
        ValueType::Array
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Array
    }
}

impl<T> FromValue for Object<T>
where
    T: FromValue,
{
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, StackError> {
        let slot = value.into_object(vm)?;
        let value = vm.object_take(slot)?;
        let mut object = Object::with_capacity(value.len());

        for (key, value) in value {
            object.insert(key, T::from_value(value, vm)?);
        }

        Ok(object)
    }
}

impl<'a> UnsafeFromValue for &'a Object<ValuePtr> {
    type Guard = RawRefGuard;

    unsafe fn unsafe_from_value(
        value: ValuePtr,
        vm: &mut Vm,
    ) -> Result<(Self, Self::Guard), StackError> {
        let slot = value.into_object(vm)?;
        Ok(Ref::unsafe_into_ref(vm.object_ref(slot)?))
    }
}

impl<T> ToValue for Object<T>
where
    T: ToValue,
{
    fn to_value(self, vm: &mut Vm) -> Result<ValuePtr, StackError> {
        let mut object = Object::with_capacity(self.len());

        for (key, value) in self {
            object.insert(key, value.to_value(vm)?);
        }

        Ok(vm.object_allocate(object))
    }
}
