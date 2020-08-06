//! String trait implementations.

use crate::reflection::{FromValue, ReflectValueType, ToValue, UnsafeFromValue};
use crate::unit::CompilationUnit;
use crate::value::{ValuePtr, ValueType, ValueTypeInfo};
use crate::vm::{Mut, RawMutGuard, RawRefGuard, Ref, Vm, VmError};

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
    fn to_value(self, vm: &mut Vm) -> Result<ValuePtr, VmError> {
        Ok(vm.string_allocate(self))
    }
}

impl FromValue for String {
    fn from_value(value: ValuePtr, vm: &mut Vm, _: &CompilationUnit) -> Result<Self, VmError> {
        let slot = value.into_string(vm)?;
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
    fn to_value(self, vm: &mut Vm) -> Result<ValuePtr, VmError> {
        Ok(vm.string_allocate(self.to_string()))
    }
}

impl FromValue for Box<str> {
    fn from_value(value: ValuePtr, vm: &mut Vm, _: &CompilationUnit) -> Result<Self, VmError> {
        let slot = value.into_string(vm)?;
        Ok(vm.string_take(slot)?.into_boxed_str())
    }
}

impl<'a> UnsafeFromValue for &'a str {
    type Guard = Option<RawRefGuard>;

    unsafe fn unsafe_from_value(
        value: ValuePtr,
        vm: &mut Vm,
        unit: &CompilationUnit,
    ) -> Result<(Self, Self::Guard), VmError> {
        Ok(match value {
            ValuePtr::String(slot) => {
                let (s, guard) = Ref::unsafe_into_ref(vm.string_ref(slot)?);
                (s.as_str(), Some(guard))
            }
            ValuePtr::StaticString(slot) => {
                let s = unit.lookup_string(slot)?;
                (&*(s as *const _), None)
            }
            actual => {
                return Err(VmError::ExpectedString {
                    actual: actual.type_info(vm)?,
                })
            }
        })
    }
}

impl<'a> UnsafeFromValue for &'a String {
    type Guard = RawRefGuard;

    unsafe fn unsafe_from_value(
        value: ValuePtr,
        vm: &mut Vm,
        _: &CompilationUnit,
    ) -> Result<(Self, Self::Guard), VmError> {
        let slot = value.into_string(vm)?;
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
    type Guard = RawMutGuard;

    unsafe fn unsafe_from_value(
        value: ValuePtr,
        vm: &mut Vm,
        _: &CompilationUnit,
    ) -> Result<(Self, Self::Guard), VmError> {
        let slot = value.into_string(vm)?;
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
