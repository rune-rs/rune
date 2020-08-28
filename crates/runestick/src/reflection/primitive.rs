//! Trait implementations for primitive types.

use crate::{
    FromValue, Integer, ReflectValueType, ToValue, Value, ValueError, ValueType, ValueTypeInfo,
};

impl ReflectValueType for bool {
    type Owned = bool;

    fn value_type() -> ValueType {
        ValueType::Bool
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Bool
    }
}

impl ToValue for bool {
    fn to_value(self) -> Result<Value, ValueError> {
        Ok(Value::Bool(self))
    }
}

impl FromValue for bool {
    fn from_value(value: Value) -> Result<Self, ValueError> {
        Ok(value.into_bool()?)
    }
}

impl ReflectValueType for char {
    type Owned = char;

    fn value_type() -> ValueType {
        ValueType::Char
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Char
    }
}

impl ToValue for char {
    fn to_value(self) -> Result<Value, ValueError> {
        Ok(Value::Char(self))
    }
}

impl FromValue for char {
    fn from_value(value: Value) -> Result<Self, ValueError> {
        Ok(value.into_char()?)
    }
}

impl ReflectValueType for u8 {
    type Owned = u8;

    fn value_type() -> ValueType {
        ValueType::Byte
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Byte
    }
}

impl ToValue for u8 {
    fn to_value(self) -> Result<Value, ValueError> {
        Ok(Value::Byte(self))
    }
}

impl FromValue for u8 {
    fn from_value(value: Value) -> Result<Self, ValueError> {
        Ok(value.into_byte()?)
    }
}

macro_rules! number_value_trait {
    ($ty:ty, $variant:ident) => {
        /// Convert a number into a value type.
        impl ReflectValueType for $ty {
            type Owned = $ty;

            fn value_type() -> ValueType {
                ValueType::Integer
            }

            fn value_type_info() -> ValueTypeInfo {
                ValueTypeInfo::Integer
            }
        }

        impl ToValue for $ty {
            fn to_value(self) -> Result<Value, ValueError> {
                use std::convert::TryInto as _;

                match self.try_into() {
                    Ok(number) => Ok(Value::Integer(number)),
                    Err(..) => Err(ValueError::IntegerToValueCoercionError {
                        from: Integer::$variant(self),
                        to: std::any::type_name::<i64>(),
                    }),
                }
            }
        }

        impl FromValue for $ty {
            fn from_value(value: Value) -> Result<Self, ValueError> {
                use std::convert::TryInto as _;
                let integer = value.into_integer()?;

                match integer.try_into() {
                    Ok(number) => Ok(number),
                    Err(..) => Err(ValueError::ValueToIntegerCoercionError {
                        from: Integer::I64(integer),
                        to: std::any::type_name::<Self>(),
                    }),
                }
            }
        }
    };
}

number_value_trait!(u32, U32);
number_value_trait!(u64, U64);
number_value_trait!(u128, U128);
number_value_trait!(usize, Usize);
number_value_trait!(i8, I8);
number_value_trait!(i32, I32);
number_value_trait!(i64, I64);
number_value_trait!(i128, I128);
number_value_trait!(isize, Isize);

/// Convert a float into a value type.
impl ReflectValueType for f64 {
    type Owned = f64;

    fn value_type() -> ValueType {
        ValueType::Float
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Float
    }
}

impl ToValue for f64 {
    fn to_value(self) -> Result<Value, ValueError> {
        Ok(Value::Float(self))
    }
}

impl FromValue for f64 {
    fn from_value(value: Value) -> Result<Self, ValueError> {
        Ok(value.into_float()?)
    }
}

/// Convert a float into a value type.
impl ReflectValueType for f32 {
    type Owned = f32;

    fn value_type() -> ValueType {
        ValueType::Float
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Float
    }
}

impl ToValue for f32 {
    fn to_value(self) -> Result<Value, ValueError> {
        Ok(Value::Float(self as f64))
    }
}

impl FromValue for f32 {
    fn from_value(value: Value) -> Result<Self, ValueError> {
        Ok(value.into_float()? as f32)
    }
}
