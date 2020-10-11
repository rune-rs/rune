use crate::collections::HashMap;
use crate::{Bytes, Object, Shared, Tuple, TypeInfo, Value, Vec, VmError, VmErrorKind};
use std::vec;

/// A constant value.
#[derive(Debug, Clone)]
pub enum ConstValue {
    /// A constant unit.
    Unit,
    /// A byte.
    Byte(u8),
    /// A character.
    Char(char),
    /// A boolean constant value.
    Bool(bool),
    /// An integer constant.
    Integer(i64),
    /// An float constant.
    Float(f64),
    /// A string constant designated by its slot.
    String(String),
    /// A byte string.
    Bytes(Bytes),
    /// A vector of values.
    Vec(vec::Vec<ConstValue>),
    /// An anonymous tuple.
    Tuple(Box<[ConstValue]>),
    /// An anonymous object.
    Object(HashMap<String, ConstValue>),
}

impl ConstValue {
    /// Convert a value into a constant value.
    pub fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(match value {
            Value::Unit => ConstValue::Unit,
            Value::Byte(b) => ConstValue::Byte(b),
            Value::Char(c) => ConstValue::Char(c),
            Value::Bool(b) => ConstValue::Bool(b),
            Value::Integer(n) => ConstValue::Integer(n),
            Value::Float(f) => ConstValue::Float(f),
            Value::String(s) => {
                let s = s.take()?;
                ConstValue::String(s)
            }
            Value::Bytes(b) => {
                let b = b.take()?;
                ConstValue::Bytes(Bytes::from(b))
            }
            Value::Vec(vec) => {
                let vec = vec.take()?;
                let mut const_vec = vec::Vec::with_capacity(vec.len());

                for value in vec {
                    const_vec.push(Self::from_value(value)?);
                }

                ConstValue::Vec(const_vec)
            }
            Value::Tuple(tuple) => {
                let tuple = tuple.take()?;
                let mut const_tuple = vec::Vec::with_capacity(tuple.len());

                for value in vec::Vec::from(tuple.into_inner()) {
                    const_tuple.push(Self::from_value(value)?);
                }

                ConstValue::Tuple(const_tuple.into_boxed_slice())
            }
            Value::Object(object) => {
                let object = object.take()?;
                let mut const_object = HashMap::with_capacity(object.len());

                for (key, value) in object {
                    const_object.insert(key, Self::from_value(value)?);
                }

                ConstValue::Object(const_object)
            }
            value => {
                return Err(VmError::from(VmErrorKind::ConstNotSupported {
                    actual: value.type_info()?,
                }))
            }
        })
    }

    /// Convert into VM value.
    pub fn into_value(self) -> Value {
        match self {
            ConstValue::Unit => Value::Unit,
            ConstValue::Byte(b) => Value::Byte(b),
            ConstValue::Char(c) => Value::Char(c),
            ConstValue::Bool(b) => Value::Bool(b),
            ConstValue::Integer(n) => Value::Integer(n),
            ConstValue::Float(n) => Value::Float(n),
            ConstValue::String(s) => Value::String(Shared::new(s)),
            ConstValue::Bytes(b) => Value::Bytes(Shared::new(b)),
            ConstValue::Vec(vec) => {
                let mut v = Vec::with_capacity(vec.len());

                for value in vec {
                    v.push(value.into_value());
                }

                Value::Vec(Shared::new(v))
            }
            ConstValue::Tuple(tuple) => {
                let mut t = vec::Vec::with_capacity(tuple.len());

                for value in vec::Vec::from(tuple) {
                    t.push(value.into_value());
                }

                Value::Tuple(Shared::new(Tuple::from(t)))
            }
            ConstValue::Object(object) => {
                let mut o = Object::with_capacity(object.len());

                for (key, value) in object {
                    o.insert(key, value.into_value());
                }

                Value::Object(Shared::new(o))
            }
        }
    }

    /// Try to coerce into boolean.
    pub fn into_bool(self) -> Result<bool, Self> {
        match self {
            Self::Bool(value) => Ok(value),
            value => Err(value),
        }
    }

    /// Get the type information of the value.
    pub fn type_info(&self) -> TypeInfo {
        match self {
            Self::Unit => TypeInfo::StaticType(crate::UNIT_TYPE),
            Self::Byte(..) => TypeInfo::StaticType(crate::BYTE_TYPE),
            Self::Char(..) => TypeInfo::StaticType(crate::CHAR_TYPE),
            Self::Bool(..) => TypeInfo::StaticType(crate::BOOL_TYPE),
            Self::String(..) => TypeInfo::StaticType(crate::STRING_TYPE),
            Self::Bytes(..) => TypeInfo::StaticType(crate::BYTES_TYPE),
            Self::Integer(..) => TypeInfo::StaticType(crate::INTEGER_TYPE),
            Self::Float(..) => TypeInfo::StaticType(crate::FLOAT_TYPE),
            Self::Vec(..) => TypeInfo::StaticType(crate::VEC_TYPE),
            Self::Tuple(..) => TypeInfo::StaticType(crate::TUPLE_TYPE),
            Self::Object(..) => TypeInfo::StaticType(crate::OBJECT_TYPE),
        }
    }
}
