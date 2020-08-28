//! String trait implementations.

use crate::{
    BorrowMut, BorrowRef, FromValue, OwnedMut, OwnedRef, RawMut, RawRef, ReflectValueType, Shared,
    SharedPtr, ToValue, UnsafeFromValue, UnsafeToValue, Value, ValueError, ValueType,
    ValueTypeInfo,
};

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

impl<'a> ReflectValueType for &'a mut str {
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

impl UnsafeToValue for &'_ String {
    unsafe fn unsafe_to_value(self) -> Result<Value, ValueError> {
        Ok(Value::Ptr(Shared::new(SharedPtr::from_ptr(self))))
    }
}

impl UnsafeToValue for &'_ mut String {
    unsafe fn unsafe_to_value(self) -> Result<Value, ValueError> {
        Ok(Value::Ptr(Shared::new(SharedPtr::from_mut_ptr(self))))
    }
}

impl FromValue for String {
    fn from_value(value: Value) -> Result<Self, ValueError> {
        match value {
            Value::String(string) => Ok(string.borrow_ref()?.clone()),
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
        let string = string.borrow_ref()?.clone();
        Ok(string.into_boxed_str())
    }
}

impl UnsafeFromValue for &'_ str {
    type Output = *const str;
    type Guard = Option<RawRef>;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), ValueError> {
        Ok(match value {
            Value::String(string) => {
                let string = string.owned_ref()?;
                let (s, guard) = OwnedRef::into_raw(string);
                ((*s).as_str(), Some(guard.into()))
            }
            Value::Ptr(ptr) => {
                let ptr = ptr.downcast_borrow_ref::<String>()?;
                let (string, guard) = BorrowRef::into_raw(ptr);
                ((*string).as_str(), Some(guard.into()))
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

impl UnsafeFromValue for &'_ String {
    type Output = *const String;
    type Guard = Option<RawRef>;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), ValueError> {
        Ok(match value {
            Value::String(string) => {
                let string = string.owned_ref()?;
                let (s, guard) = OwnedRef::into_raw(string);
                (s, Some(guard.into()))
            }
            Value::Ptr(ptr) => {
                let ptr = ptr.downcast_borrow_ref::<String>()?;
                let (string, guard) = BorrowRef::into_raw(ptr);
                (string, Some(guard.into()))
            }
            Value::StaticString(string) => (string.as_ref(), None),
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

impl ReflectValueType for &'_ String {
    type Owned = String;

    fn value_type() -> ValueType {
        ValueType::String
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::String
    }
}

impl UnsafeFromValue for &'_ mut String {
    type Output = *mut String;
    type Guard = RawMut;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), ValueError> {
        Ok(match value {
            Value::String(string) => {
                let string = string.owned_mut()?;
                let (s, guard) = OwnedMut::into_raw(string);
                (s, guard.into())
            }
            Value::Ptr(ptr) => {
                let ptr = ptr.downcast_borrow_mut::<String>()?;
                let (string, guard) = BorrowMut::into_raw(ptr);
                (string, guard.into())
            }
            actual => {
                return Err(ValueError::ExpectedString {
                    actual: actual.type_info()?,
                })
            }
        })
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &mut *output
    }
}

impl ReflectValueType for &'_ mut String {
    type Owned = String;

    fn value_type() -> ValueType {
        ValueType::String
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::String
    }
}
