//! Trait implementations for primitive types.

use crate::reflection::{FromValue, ReflectValueType, ToValue};
use crate::value::{Value, ValueType, ValueTypeInfo};
use crate::vm::{Integer, Vm, VmError};

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
    fn to_value(self, _vm: &mut Vm) -> Result<Value, VmError> {
        Ok(Value::Bool(self))
    }
}

impl FromValue for bool {
    fn from_value(value: Value, vm: &mut Vm) -> Result<Self, VmError> {
        match value {
            Value::Bool(value) => Ok(value),
            actual => Err(VmError::ExpectedBoolean {
                actual: actual.type_info(vm)?,
            }),
        }
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
    fn to_value(self, _vm: &mut Vm) -> Result<Value, VmError> {
        Ok(Value::Char(self))
    }
}

impl FromValue for char {
    fn from_value(value: Value, vm: &mut Vm) -> Result<Self, VmError> {
        match value {
            Value::Char(value) => Ok(value),
            actual => Err(VmError::ExpectedChar {
                actual: actual.type_info(vm)?,
            }),
        }
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
            fn to_value(self, _vm: &mut Vm) -> Result<Value, VmError> {
                use std::convert::TryInto as _;

                match self.try_into() {
                    Ok(number) => Ok(Value::Integer(number)),
                    Err(..) => Err(VmError::IntegerToValueCoercionError {
                        from: Integer::$variant(self),
                        to: std::any::type_name::<i64>(),
                    }),
                }
            }
        }

        impl FromValue for $ty {
            fn from_value(value: Value, vm: &mut Vm) -> Result<Self, VmError> {
                use std::convert::TryInto as _;

                match value {
                    Value::Integer(number) => match number.try_into() {
                        Ok(number) => Ok(number),
                        Err(..) => Err(VmError::ValueToIntegerCoercionError {
                            from: Integer::I64(number),
                            to: std::any::type_name::<Self>(),
                        }),
                    },
                    actual => Err(VmError::ExpectedInteger {
                        actual: actual.type_info(vm)?,
                    }),
                }
            }
        }
    };
}

number_value_trait!(u8, U8);
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
    fn to_value(self, _vm: &mut Vm) -> Result<Value, VmError> {
        Ok(Value::Float(self))
    }
}

impl FromValue for f64 {
    fn from_value(value: Value, vm: &mut Vm) -> Result<Self, VmError> {
        match value {
            Value::Float(number) => Ok(number),
            actual => Err(VmError::ExpectedFloat {
                actual: actual.type_info(vm)?,
            }),
        }
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
    fn to_value(self, _vm: &mut Vm) -> Result<Value, VmError> {
        Ok(Value::Float(self as f64))
    }
}

impl FromValue for f32 {
    fn from_value(value: Value, vm: &mut Vm) -> Result<Self, VmError> {
        match value {
            Value::Float(number) => Ok(number as f32),
            actual => Err(VmError::ExpectedFloat {
                actual: actual.type_info(vm)?,
            }),
        }
    }
}
