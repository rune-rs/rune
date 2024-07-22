use serde::{Deserialize, Serialize};

use crate::alloc::prelude::*;
use crate::alloc::{self, Box, HashMap, String, Vec};
use crate::runtime::{
    self, Bytes, FromValue, Object, OwnedTuple, ToValue, TypeInfo, Value, ValueKind, VmErrorKind,
    VmResult,
};

/// A constant value.
#[derive(Debug, Deserialize, Serialize)]
pub enum ConstValue {
    /// A constant unit.
    EmptyTuple,
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
    Vec(Vec<ConstValue>),
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
    pub fn as_value(&self) -> alloc::Result<Value> {
        Ok(match self {
            Self::EmptyTuple => Value::unit()?,
            Self::Byte(b) => Value::try_from(*b)?,
            Self::Char(c) => Value::try_from(*c)?,
            Self::Bool(b) => Value::try_from(*b)?,
            Self::Integer(n) => Value::try_from(*n)?,
            Self::Float(n) => Value::try_from(*n)?,
            Self::String(string) => Value::try_from(string.try_clone()?)?,
            Self::Bytes(b) => Value::try_from(b.try_clone()?)?,
            Self::Option(option) => Value::try_from(match option {
                Some(some) => Some(some.as_value()?),
                None => None,
            })?,
            Self::Vec(vec) => {
                let mut v = runtime::Vec::with_capacity(vec.len())?;

                for value in vec {
                    v.push(value.as_value()?)?;
                }

                Value::try_from(v)?
            }
            Self::Tuple(tuple) => {
                let mut t = Vec::try_with_capacity(tuple.len())?;

                for value in tuple.iter() {
                    t.try_push(value.as_value()?)?;
                }

                Value::try_from(OwnedTuple::try_from(t)?)?
            }
            Self::Object(object) => {
                let mut o = Object::with_capacity(object.len())?;

                for (key, value) in object {
                    let key = key.try_clone()?;
                    o.insert(key, value.as_value()?)?;
                }

                Value::try_from(o)?
            }
        })
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
            Self::EmptyTuple => TypeInfo::StaticType(crate::runtime::static_type::TUPLE),
            Self::Byte(..) => TypeInfo::StaticType(crate::runtime::static_type::BYTE),
            Self::Char(..) => TypeInfo::StaticType(crate::runtime::static_type::CHAR),
            Self::Bool(..) => TypeInfo::StaticType(crate::runtime::static_type::BOOL),
            Self::String(..) => TypeInfo::StaticType(crate::runtime::static_type::STRING),
            Self::Bytes(..) => TypeInfo::StaticType(crate::runtime::static_type::BYTES),
            Self::Integer(..) => TypeInfo::StaticType(crate::runtime::static_type::INTEGER),
            Self::Float(..) => TypeInfo::StaticType(crate::runtime::static_type::FLOAT),
            Self::Vec(..) => TypeInfo::StaticType(crate::runtime::static_type::VEC),
            Self::Tuple(..) => TypeInfo::StaticType(crate::runtime::static_type::TUPLE),
            Self::Object(..) => TypeInfo::StaticType(crate::runtime::static_type::OBJECT),
            Self::Option(..) => TypeInfo::StaticType(crate::runtime::static_type::OPTION),
        }
    }
}

impl TryClone for ConstValue {
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(match self {
            ConstValue::EmptyTuple => ConstValue::EmptyTuple,
            ConstValue::Byte(byte) => ConstValue::Byte(*byte),
            ConstValue::Char(char) => ConstValue::Char(*char),
            ConstValue::Bool(bool) => ConstValue::Bool(*bool),
            ConstValue::Integer(integer) => ConstValue::Integer(*integer),
            ConstValue::Float(float) => ConstValue::Float(*float),
            ConstValue::String(value) => ConstValue::String(value.try_clone()?),
            ConstValue::Bytes(value) => ConstValue::Bytes(value.try_clone()?),
            ConstValue::Vec(value) => ConstValue::Vec(value.try_clone()?),
            ConstValue::Tuple(value) => ConstValue::Tuple(value.try_clone()?),
            ConstValue::Object(value) => ConstValue::Object(value.try_clone()?),
            ConstValue::Option(value) => ConstValue::Option(value.try_clone()?),
        })
    }
}

impl FromValue for ConstValue {
    fn from_value(value: Value) -> VmResult<Self> {
        VmResult::Ok(match vm_try!(value.take_kind()) {
            ValueKind::EmptyTuple => Self::EmptyTuple,
            ValueKind::Byte(b) => Self::Byte(b),
            ValueKind::Char(c) => Self::Char(c),
            ValueKind::Bool(b) => Self::Bool(b),
            ValueKind::Integer(n) => Self::Integer(n),
            ValueKind::Float(f) => Self::Float(f),
            ValueKind::String(s) => Self::String(s),
            ValueKind::Option(option) => Self::Option(match option {
                Some(some) => Some(vm_try!(Box::try_new(vm_try!(Self::from_value(some))))),
                None => None,
            }),
            ValueKind::Bytes(b) => Self::Bytes(b),
            ValueKind::Vec(vec) => {
                let mut const_vec = vm_try!(Vec::try_with_capacity(vec.len()));

                for value in vec {
                    vm_try!(const_vec.try_push(vm_try!(Self::from_value(value))));
                }

                Self::Vec(const_vec)
            }
            ValueKind::Tuple(tuple) => {
                let mut const_tuple = vm_try!(Vec::try_with_capacity(tuple.len()));

                for value in Vec::from(tuple.into_inner()) {
                    vm_try!(const_tuple.try_push(vm_try!(Self::from_value(value))));
                }

                Self::Tuple(vm_try!(const_tuple.try_into_boxed_slice()))
            }
            ValueKind::Object(object) => {
                let mut const_object = vm_try!(HashMap::try_with_capacity(object.len()));

                for (key, value) in object {
                    vm_try!(const_object.try_insert(key, vm_try!(Self::from_value(value))));
                }

                Self::Object(const_object)
            }
            actual => {
                return VmResult::err(VmErrorKind::ConstNotSupported {
                    actual: actual.type_info(),
                })
            }
        })
    }
}

impl ToValue for ConstValue {
    fn to_value(self) -> VmResult<Value> {
        VmResult::Ok(vm_try!(ConstValue::as_value(&self)))
    }
}
