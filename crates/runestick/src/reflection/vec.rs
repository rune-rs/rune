use crate::reflection::{FromValue, ReflectValueType, ToValue, UnsafeFromValue};
use crate::shared::{RawStrongMutGuard, RawStrongRefGuard, Shared, StrongMut, StrongRef};
use crate::value::{Value, ValueType, ValueTypeInfo};
use crate::vm::VmError;

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
    fn from_value(value: Value) -> Result<Self, VmError> {
        let vec = value.into_vec()?;
        let vec = vec.take()?;

        let mut output = Vec::with_capacity(vec.len());

        for value in vec {
            output.push(T::from_value(value)?);
        }

        Ok(output)
    }
}

impl<'a> UnsafeFromValue for &'a Vec<Value> {
    type Output = *const Vec<Value>;
    type Guard = RawStrongRefGuard;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let vec = value.into_vec()?;
        Ok(StrongRef::into_raw(vec.strong_ref()?))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl<'a> UnsafeFromValue for &'a mut Vec<Value> {
    type Output = *mut Vec<Value>;
    type Guard = RawStrongMutGuard;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let vec = value.into_vec()?;
        Ok(StrongMut::into_raw(vec.strong_mut()?))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &mut *output
    }
}

impl<T> ToValue for Vec<T>
where
    T: ToValue,
{
    fn to_value(self) -> Result<Value, VmError> {
        let mut vec = Vec::with_capacity(self.len());

        for value in self {
            vec.push(value.to_value()?);
        }

        Ok(Value::Vec(Shared::new(vec)))
    }
}
