//! Trait implementations for primitive types.

use crate::reflection::{FromValue, ReflectValueType, ToValue};
use crate::value::{ValuePtr, ValueType, ValueTypeInfo};
use crate::vm::{Integer, Vm, VmError};

impl ReflectValueType for crate::value::Unit {
    fn value_type() -> ValueType {
        ValueType::Unit
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Unit
    }
}

impl ToValue for crate::value::Unit {
    fn to_value(self, _vm: &mut Vm) -> Result<ValuePtr, VmError> {
        Ok(ValuePtr::None)
    }
}

impl FromValue for crate::value::Unit {
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, VmError> {
        match value {
            ValuePtr::None => Ok(crate::value::Unit),
            actual => Err(VmError::ExpectedNone {
                actual: actual.type_info(vm)?,
            }),
        }
    }
}

impl ToValue for () {
    fn to_value(self, _vm: &mut Vm) -> Result<ValuePtr, VmError> {
        Ok(ValuePtr::None)
    }
}

impl ReflectValueType for bool {
    fn value_type() -> ValueType {
        ValueType::Bool
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Bool
    }
}

impl ToValue for bool {
    fn to_value(self, _vm: &mut Vm) -> Result<ValuePtr, VmError> {
        Ok(ValuePtr::Bool(self))
    }
}

impl FromValue for bool {
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, VmError> {
        match value {
            ValuePtr::Bool(value) => Ok(value),
            actual => Err(VmError::ExpectedBoolean {
                actual: actual.type_info(vm)?,
            }),
        }
    }
}

impl ReflectValueType for char {
    fn value_type() -> ValueType {
        ValueType::Char
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Char
    }
}

impl ToValue for char {
    fn to_value(self, _vm: &mut Vm) -> Result<ValuePtr, VmError> {
        Ok(ValuePtr::Char(self))
    }
}

impl FromValue for char {
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, VmError> {
        match value {
            ValuePtr::Char(value) => Ok(value),
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
            fn value_type() -> ValueType {
                ValueType::Integer
            }

            fn value_type_info() -> ValueTypeInfo {
                ValueTypeInfo::Integer
            }
        }

        impl ToValue for $ty {
            fn to_value(self, _vm: &mut Vm) -> Result<ValuePtr, VmError> {
                use std::convert::TryInto as _;

                match self.try_into() {
                    Ok(number) => Ok(ValuePtr::Integer(number)),
                    Err(..) => Err(VmError::IntegerToValueCoercionError {
                        from: Integer::$variant(self),
                        to: std::any::type_name::<i64>(),
                    }),
                }
            }
        }

        impl FromValue for $ty {
            fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, VmError> {
                use std::convert::TryInto as _;

                match value {
                    ValuePtr::Integer(number) => match number.try_into() {
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
    fn value_type() -> ValueType {
        ValueType::Float
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Float
    }
}

impl ToValue for f64 {
    fn to_value(self, _vm: &mut Vm) -> Result<ValuePtr, VmError> {
        Ok(ValuePtr::Float(self))
    }
}

impl FromValue for f64 {
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, VmError> {
        match value {
            ValuePtr::Float(number) => Ok(number),
            actual => Err(VmError::ExpectedFloat {
                actual: actual.type_info(vm)?,
            }),
        }
    }
}

/// Convert a float into a value type.
impl ReflectValueType for f32 {
    fn value_type() -> ValueType {
        ValueType::Float
    }

    fn value_type_info() -> ValueTypeInfo {
        ValueTypeInfo::Float
    }
}

impl ToValue for f32 {
    fn to_value(self, _vm: &mut Vm) -> Result<ValuePtr, VmError> {
        Ok(ValuePtr::Float(self as f64))
    }
}

impl FromValue for f32 {
    fn from_value(value: ValuePtr, vm: &mut Vm) -> Result<Self, VmError> {
        match value {
            ValuePtr::Float(number) => Ok(number as f32),
            actual => Err(VmError::ExpectedFloat {
                actual: actual.type_info(vm)?,
            }),
        }
    }
}
