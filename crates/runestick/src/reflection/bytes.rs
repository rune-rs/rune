use crate::bytes::Bytes;
use crate::reflection::{FromValue, ReflectValueType, ToValue, UnsafeFromValue, UnsafeToValue};
use crate::shared::{OwnMut, OwnRef, RawOwnMut, RawOwnRef, Shared};
use crate::value::{Value, ValueError, ValueType, ValueTypeInfo};

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
    fn to_value(self) -> Result<Value, ValueError> {
        Ok(Value::Bytes(Shared::new(self)))
    }
}

impl<'a> UnsafeToValue for &'a Bytes {
    unsafe fn unsafe_to_value(self) -> Result<Value, ValueError> {
        Ok(Value::from_ptr(self))
    }
}

impl<'a> UnsafeToValue for &'a mut Bytes {
    unsafe fn unsafe_to_value(self) -> Result<Value, ValueError> {
        Ok(Value::from_mut_ptr(self))
    }
}

impl FromValue for Bytes {
    fn from_value(value: Value) -> Result<Self, ValueError> {
        let bytes = value.into_bytes()?;
        Ok(bytes.borrow_ref()?.clone())
    }
}

impl<'a> UnsafeFromValue for &'a Bytes {
    type Output = *const Bytes;
    type Guard = RawOwnRef;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), ValueError> {
        let bytes = value.into_bytes()?;
        let bytes = bytes.own_ref()?;
        Ok(OwnRef::into_raw(bytes))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl<'a> UnsafeFromValue for &'a mut Bytes {
    type Output = *mut Bytes;
    type Guard = RawOwnMut;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), ValueError> {
        let bytes = value.into_bytes()?;
        let bytes = bytes.own_mut()?;
        Ok(OwnMut::into_raw(bytes))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &mut *output
    }
}

impl<'a> UnsafeFromValue for &'a [u8] {
    type Output = *const [u8];
    type Guard = RawOwnRef;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), ValueError> {
        let bytes = value.into_bytes()?;
        let bytes = bytes.own_ref()?;
        let (value, guard) = OwnRef::into_raw(bytes);
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
