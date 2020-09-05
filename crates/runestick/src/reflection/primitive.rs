//! Trait implementations for primitive types.

use crate::{FromValue, Integer, ToValue, Value, VmError, VmErrorKind};

value_types!(crate::UNIT_TYPE, () => ());
value_types!(crate::BYTE_TYPE, u8 => u8);
value_types!(crate::BOOL_TYPE, bool => bool);
value_types!(crate::CHAR_TYPE, char => char);
value_types!(crate::INTEGER_TYPE, i64 => i64);
value_types!(crate::FLOAT_TYPE, f64 => f64);
value_types!(crate::FLOAT_TYPE, f32 => f32);

impl FromValue for () {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_unit()?)
    }
}

impl FromValue for u8 {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_byte()?)
    }
}

impl FromValue for bool {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_bool()?)
    }
}

impl FromValue for char {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_char()?)
    }
}

impl FromValue for i64 {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_integer()?)
    }
}

macro_rules! number_value_trait {
    ($ty:ty, $variant:ident) => {
        value_types!(crate::INTEGER_TYPE, $ty => $ty);

        impl ToValue for $ty {
            fn to_value(self) -> Result<Value, VmError> {
                use std::convert::TryInto as _;

                match self.try_into() {
                    Ok(number) => Ok(Value::Integer(number)),
                    Err(..) => Err(VmError::from(VmErrorKind::IntegerToValueCoercionError {
                        from: Integer::$variant(self),
                        to: std::any::type_name::<i64>(),
                    })),
                }
            }
        }

        impl FromValue for $ty {
            fn from_value(value: Value) -> Result<Self, VmError> {
                use std::convert::TryInto as _;
                let integer = value.into_integer()?;

                match integer.try_into() {
                    Ok(number) => Ok(number),
                    Err(..) => Err(VmError::from(VmErrorKind::ValueToIntegerCoercionError {
                        from: Integer::I64(integer),
                        to: std::any::type_name::<Self>(),
                    })),
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
number_value_trait!(i128, I128);
number_value_trait!(isize, Isize);

impl FromValue for f64 {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_float()?)
    }
}

impl ToValue for f32 {
    fn to_value(self) -> Result<Value, VmError> {
        Ok(Value::Float(self as f64))
    }
}

impl FromValue for f32 {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(value.into_float()? as f32)
    }
}
