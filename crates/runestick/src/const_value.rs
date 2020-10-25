use crate::collections::HashMap;
use crate::{
    Bytes, FromValue, Object, Shared, StaticString, ToValue, Tuple, TypeInfo, Value, Vec, VmError,
    VmErrorKind,
};
use std::sync::Arc;
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
    /// A static string.
    StaticString(Arc<StaticString>),
    /// A byte string.
    Bytes(Bytes),
    /// A vector of values.
    Vec(vec::Vec<ConstValue>),
    /// An anonymous tuple.
    Tuple(Box<[ConstValue]>),
    /// An anonymous object.
    Object(HashMap<String, ConstValue>),
    /// An option.
    Option(Option<Box<ConstValue>>),
}

impl ConstValue {
    /// Convert into virtual machine value.
    ///
    /// We provide this associated method since a constant value can be
    /// converted into a value infallibly, which is not captured by the trait
    /// otherwise.
    pub fn into_value(self) -> Value {
        match self {
            Self::Unit => Value::Unit,
            Self::Byte(b) => Value::Byte(b),
            Self::Char(c) => Value::Char(c),
            Self::Bool(b) => Value::Bool(b),
            Self::Integer(n) => Value::Integer(n),
            Self::Float(n) => Value::Float(n),
            Self::String(s) => Value::String(Shared::new(s)),
            Self::StaticString(s) => Value::StaticString(s),
            Self::Bytes(b) => Value::Bytes(Shared::new(b)),
            Self::Option(option) => Value::Option(Shared::new(match option {
                Some(some) => Some(some.into_value()),
                None => None,
            })),
            Self::Vec(vec) => {
                let mut v = Vec::with_capacity(vec.len());

                for value in vec {
                    v.push(value.into_value());
                }

                Value::Vec(Shared::new(v))
            }
            Self::Tuple(tuple) => {
                let mut t = vec::Vec::with_capacity(tuple.len());

                for value in vec::Vec::from(tuple) {
                    t.push(value.into_value());
                }

                Value::Tuple(Shared::new(Tuple::from(t)))
            }
            Self::Object(object) => {
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
            Self::StaticString(..) => TypeInfo::StaticType(crate::STRING_TYPE),
            Self::Bytes(..) => TypeInfo::StaticType(crate::BYTES_TYPE),
            Self::Integer(..) => TypeInfo::StaticType(crate::INTEGER_TYPE),
            Self::Float(..) => TypeInfo::StaticType(crate::FLOAT_TYPE),
            Self::Vec(..) => TypeInfo::StaticType(crate::VEC_TYPE),
            Self::Tuple(..) => TypeInfo::StaticType(crate::TUPLE_TYPE),
            Self::Object(..) => TypeInfo::StaticType(crate::OBJECT_TYPE),
            Self::Option(..) => TypeInfo::StaticType(crate::OPTION_TYPE),
        }
    }
}

impl FromValue for ConstValue {
    fn from_value(value: Value) -> Result<Self, VmError> {
        Ok(match value {
            Value::Unit => Self::Unit,
            Value::Byte(b) => Self::Byte(b),
            Value::Char(c) => Self::Char(c),
            Value::Bool(b) => Self::Bool(b),
            Value::Integer(n) => Self::Integer(n),
            Value::Float(f) => Self::Float(f),
            Value::String(s) => {
                let s = s.take()?;
                Self::String(s)
            }
            Value::StaticString(s) => Self::StaticString(s),
            Value::Option(option) => Self::Option(match option.take()? {
                Some(some) => Some(Box::new(Self::from_value(some)?)),
                None => None,
            }),
            Value::Bytes(b) => {
                let b = b.take()?;
                Self::Bytes(Bytes::from(b))
            }
            Value::Vec(vec) => {
                let vec = vec.take()?;
                let mut const_vec = vec::Vec::with_capacity(vec.len());

                for value in vec {
                    const_vec.push(Self::from_value(value)?);
                }

                Self::Vec(const_vec)
            }
            Value::Tuple(tuple) => {
                let tuple = tuple.take()?;
                let mut const_tuple = vec::Vec::with_capacity(tuple.len());

                for value in vec::Vec::from(tuple.into_inner()) {
                    const_tuple.push(Self::from_value(value)?);
                }

                Self::Tuple(const_tuple.into_boxed_slice())
            }
            Value::Object(object) => {
                let object = object.take()?;
                let mut const_object = HashMap::with_capacity(object.len());

                for (key, value) in object {
                    const_object.insert(key, Self::from_value(value)?);
                }

                Self::Object(const_object)
            }
            value => {
                return Err(VmError::from(VmErrorKind::ConstNotSupported {
                    actual: value.type_info()?,
                }))
            }
        })
    }
}

impl ToValue for ConstValue {
    fn to_value(self) -> Result<Value, VmError> {
        Ok(ConstValue::into_value(self))
    }
}
