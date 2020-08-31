//! Trait implementations for primitive types.

use crate::{FromValue, Integer, ToValue, Value, ValueError};

value_types!(crate::BOOL_TYPE, bool => bool);
value_types!(crate::CHAR_TYPE, char => char);
value_types!(crate::BYTE_TYPE, u8 => u8);
value_types!(crate::FLOAT_TYPE, f64 => f64);
value_types!(crate::FLOAT_TYPE, f32 => f32);

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
        value_types!(crate::INTEGER_TYPE, $ty => $ty);

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
