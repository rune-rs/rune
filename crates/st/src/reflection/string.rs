//! String trait implementations.

use crate::reflection::{FromValue, ReflectValueType, ToValue, UnsafeFromValue};
use crate::value::{ValuePtr, ValueType, ValueTypeInfo};
use crate::vm::{Mut, Ref, StackError, Vm};

impl ReflectValueType for String {
    fn value_type() -> ValueType {
        ValueType::String
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::String
    }
}

impl<'a> ReflectValueType for &'a str {
    fn value_type() -> ValueType {
        ValueType::String
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::String
    }
}

impl ToValue for String {
    fn to_value(self, vm: &mut Vm) -> Result<ValuePtr, StackError> {
        Ok(vm.string_allocate(self))
    }
}

impl FromValue for String {
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, StackError> {
        let slot = value.into_string()?;
        vm.string_take(slot)
    }
}

/// Convert a string into a value type.
impl ReflectValueType for Box<str> {
    fn value_type() -> ValueType {
        ValueType::String
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::String
    }
}

impl ToValue for Box<str> {
    fn to_value(self, vm: &mut Vm) -> Result<ValuePtr, StackError> {
        Ok(vm.string_allocate(self.to_string()))
    }
}

impl FromValue for Box<str> {
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, StackError> {
        let slot = value.into_string()?;
        Ok(vm.string_take(slot)?.into_boxed_str())
    }
}

impl<'a> UnsafeFromValue for &'a str {
    unsafe fn unsafe_from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, StackError> {
        let slot = value.into_string()?;
        Ok(Ref::unsafe_into_ref(vm.string_ref(slot)?).as_str())
    }
}

impl<'a> UnsafeFromValue for &'a String {
    unsafe fn unsafe_from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, StackError> {
        let slot = value.into_string()?;
        Ok(Ref::unsafe_into_ref(vm.string_ref(slot)?))
    }
}

impl<'a> ReflectValueType for &'a String {
    fn value_type() -> ValueType {
        ValueType::String
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::String
    }
}

impl<'a> UnsafeFromValue for &'a mut String {
    unsafe fn unsafe_from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, StackError> {
        let slot = value.into_string()?;
        Ok(Mut::unsafe_into_mut(vm.string_mut(slot)?))
    }
}

impl<'a> ReflectValueType for &'a mut String {
    fn value_type() -> ValueType {
        ValueType::String
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::String
    }
}
