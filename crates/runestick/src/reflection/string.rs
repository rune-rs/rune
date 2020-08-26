//! String trait implementations.

use crate::reflection::{FromValue, ReflectValueType, ToValue, UnsafeFromValue};
use crate::shared::{RawStrongMutGuard, Shared, StrongMut};
use crate::shared::{RawStrongRefGuard, StrongRef};
use crate::value::{Value, ValueError, ValueType, ValueTypeInfo};

impl ReflectValueType for String {
    type Owned = String;

    fn value_type() -> ValueType {
        ValueType::String
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::String
    }
}

impl<'a> ReflectValueType for &'a str {
    type Owned = String;

    fn value_type() -> ValueType {
        ValueType::String
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::String
    }
}

impl ToValue for String {
    fn to_value(self) -> Result<Value, ValueError> {
        Ok(Value::String(Shared::new(self)))
    }
}

impl FromValue for String {
    fn from_value(value: Value) -> Result<Self, ValueError> {
        match value {
            Value::String(string) => Ok(string.get_ref()?.clone()),
            Value::StaticString(string) => Ok(string.as_ref().clone()),
            actual => Err(ValueError::ExpectedString {
                actual: actual.type_info()?,
            }),
        }
    }
}

/// Convert a string into a value type.
impl ReflectValueType for Box<str> {
    type Owned = String;

    fn value_type() -> ValueType {
        ValueType::String
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::String
    }
}

impl ToValue for Box<str> {
    fn to_value(self) -> Result<Value, ValueError> {
        Ok(Value::String(Shared::new(self.to_string())))
    }
}

impl FromValue for Box<str> {
    fn from_value(value: Value) -> Result<Self, ValueError> {
        let string = value.into_string()?;
        let string = string.get_ref()?.clone();
        Ok(string.into_boxed_str())
    }
}

impl<'a> UnsafeFromValue for &'a str {
    type Output = *const str;
    type Guard = Option<RawStrongRefGuard>;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), ValueError> {
        Ok(match value {
            Value::String(string) => {
                let string = string.strong_ref()?;
                let (s, guard) = StrongRef::into_raw(string);
                ((*s).as_str(), Some(guard))
            }
            Value::StaticString(string) => (string.as_ref().as_str(), None),
            actual => {
                return Err(ValueError::ExpectedString {
                    actual: actual.type_info()?,
                })
            }
        })
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl<'a> UnsafeFromValue for &'a String {
    type Output = *const String;
    type Guard = RawStrongRefGuard;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), ValueError> {
        let string = value.into_string()?;
        let string = string.strong_ref()?;
        Ok(StrongRef::into_raw(string))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl<'a> ReflectValueType for &'a String {
    type Owned = String;

    fn value_type() -> ValueType {
        ValueType::String
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::String
    }
}

impl<'a> UnsafeFromValue for &'a mut String {
    type Output = *mut String;
    type Guard = RawStrongMutGuard;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), ValueError> {
        let string = value.into_string()?;
        let string = string.strong_mut()?;
        Ok(StrongMut::into_raw(string))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &mut *output
    }
}

impl<'a> ReflectValueType for &'a mut String {
    type Owned = String;

    fn value_type() -> ValueType {
        ValueType::String
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::String
    }
}
