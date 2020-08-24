use crate::bytes::Bytes;
use crate::reflection::{FromValue, ReflectValueType, ToValue, UnsafeFromValue, UnsafeToValue};
use crate::shared::{RawStrongMutGuard, RawStrongRefGuard, Shared, StrongMut, StrongRef};
use crate::value::{Value, ValueType, ValueTypeInfo};
use crate::vm::VmError;

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
    fn to_value(self) -> Result<Value, VmError> {
        Ok(Value::Bytes(Shared::new(self)))
    }
}

impl<'a> UnsafeToValue for &'a Bytes {
    unsafe fn unsafe_to_value(self) -> Result<Value, VmError> {
        Ok(Value::from_ptr(self))
    }
}

impl<'a> UnsafeToValue for &'a mut Bytes {
    unsafe fn unsafe_to_value(self) -> Result<Value, VmError> {
        Ok(Value::from_mut_ptr(self))
    }
}

impl FromValue for Bytes {
    fn from_value(value: Value) -> Result<Self, VmError> {
        let bytes = value.into_bytes()?;
        Ok(bytes.get_ref()?.clone())
    }
}

impl<'a> UnsafeFromValue for &'a Bytes {
    type Output = *const Bytes;
    type Guard = RawStrongRefGuard;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let bytes = value.into_bytes()?;
        let bytes = bytes.strong_ref()?;
        Ok(StrongRef::into_raw(bytes))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl<'a> UnsafeFromValue for &'a mut Bytes {
    type Output = *mut Bytes;
    type Guard = RawStrongMutGuard;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let bytes = value.into_bytes()?;
        let bytes = bytes.strong_mut()?;
        Ok(StrongMut::into_raw(bytes))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &mut *output
    }
}

impl<'a> UnsafeFromValue for &'a [u8] {
    type Output = *const [u8];
    type Guard = RawStrongRefGuard;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let bytes = value.into_bytes()?;
        let bytes = bytes.strong_ref()?;
        let (value, guard) = StrongRef::into_raw(bytes);
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
