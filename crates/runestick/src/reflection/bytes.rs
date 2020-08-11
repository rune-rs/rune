use crate::bytes::Bytes;
use crate::reflection::{FromValue, ReflectValueType, ToValue, UnsafeFromValue, UnsafeToValue};
use crate::value::{Value, ValueType, ValueTypeInfo};
use crate::vm::{Mut, RawMutGuard, RawRefGuard, Ref, Vm, VmError};

impl ReflectValueType for Bytes {
    type Owned = Bytes;

    fn value_type() -> ValueType {
        ValueType::Bytes
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Bytes
    }
}

impl<'a> ReflectValueType for &'a Bytes {
    type Owned = Bytes;

    fn value_type() -> ValueType {
        ValueType::Bytes
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Bytes
    }
}

impl<'a> ReflectValueType for &'a mut Bytes {
    type Owned = Bytes;

    fn value_type() -> ValueType {
        ValueType::Bytes
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Bytes
    }
}

impl ToValue for Bytes {
    fn to_value(self, vm: &mut Vm) -> Result<Value, VmError> {
        Ok(vm.bytes_allocate(self))
    }
}

impl<'a> UnsafeToValue for &'a Bytes {
    unsafe fn unsafe_to_value(self, vm: &mut Vm) -> Result<Value, VmError> {
        Ok(vm.external_allocate_ptr(self))
    }
}

impl<'a> UnsafeToValue for &'a mut Bytes {
    unsafe fn unsafe_to_value(self, vm: &mut Vm) -> Result<Value, VmError> {
        Ok(vm.external_allocate_mut_ptr(self))
    }
}

impl FromValue for Bytes {
    fn from_value(value: Value, vm: &mut Vm) -> Result<Self, VmError> {
        let slot = value.into_bytes(vm)?;
        vm.bytes_take(slot)
    }
}

impl<'a> UnsafeFromValue for &'a Bytes {
    type Output = *const Bytes;
    type Guard = RawRefGuard;

    unsafe fn unsafe_from_value(
        value: Value,
        vm: &mut Vm,
    ) -> Result<(Self::Output, Self::Guard), VmError> {
        let slot = value.into_bytes(vm)?;
        Ok(Ref::unsafe_into_ref(vm.bytes_ref(slot)?))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl<'a> UnsafeFromValue for &'a mut Bytes {
    type Output = *mut Bytes;
    type Guard = RawMutGuard;

    unsafe fn unsafe_from_value(
        value: Value,
        vm: &mut Vm,
    ) -> Result<(Self::Output, Self::Guard), VmError> {
        let slot = value.into_bytes(vm)?;
        Ok(Mut::unsafe_into_mut(vm.bytes_mut(slot)?))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &mut *output
    }
}

impl<'a> UnsafeFromValue for &'a [u8] {
    type Output = *const [u8];
    type Guard = RawRefGuard;

    unsafe fn unsafe_from_value(
        value: Value,
        vm: &mut Vm,
    ) -> Result<(Self::Output, Self::Guard), VmError> {
        let slot = value.into_bytes(vm)?;
        let (value, guard) = Ref::unsafe_into_ref(vm.bytes_ref(slot)?);
        Ok(((*value).bytes.as_slice(), guard))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl<'a> ReflectValueType for &'a [u8] {
    type Owned = Bytes;

    fn value_type() -> ValueType {
        ValueType::Bytes
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Bytes
    }
}
