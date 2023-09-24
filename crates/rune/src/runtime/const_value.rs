use serde::{Deserialize, Serialize};

use crate::alloc::prelude::*;
use crate::alloc::{self, Box, HashMap, String, Vec};
use crate::runtime::{
    self, Bytes, FromValue, Object, OwnedTuple, Shared, ToValue, TypeInfo, Value, VmErrorKind,
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
    pub fn into_value(self) -> alloc::Result<Value> {
        Ok(match self {
            Self::Byte(b) => Value::Byte(b),
            Self::Char(c) => Value::Char(c),
            Self::Bool(b) => Value::Bool(b),
            Self::Integer(n) => Value::Integer(n),
            Self::Float(n) => Value::Float(n),
            Self::String(string) => Value::String(Shared::new(string)?),
            Self::Bytes(b) => Value::Bytes(Shared::new(b)?),
            Self::Option(option) => Value::Option(Shared::new(match option {
                Some(some) => Some(Box::into_inner(some).into_value()?),
                None => None,
            })?),
            Self::Vec(vec) => {
                let mut v = runtime::Vec::with_capacity(vec.len())?;

                for value in vec {
                    v.push(value.into_value()?)?;
                }

                Value::Vec(Shared::new(v)?)
            }
            Self::EmptyTuple => Value::EmptyTuple,
            Self::Tuple(tuple) => {
                let mut t = Vec::try_with_capacity(tuple.len())?;

                for value in Vec::from(tuple) {
                    t.try_push(value.into_value()?)?;
                }

                Value::Tuple(Shared::new(OwnedTuple::try_from(t)?)?)
            }
            Self::Object(object) => {
                let mut o = Object::with_capacity(object.len())?;

                for (key, value) in object {
                    o.insert(key, value.into_value()?)?;
                }

                Value::Object(Shared::new(o)?)
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
            Self::EmptyTuple => TypeInfo::StaticType(crate::runtime::static_type::TUPLE_TYPE),
            Self::Byte(..) => TypeInfo::StaticType(crate::runtime::static_type::BYTE_TYPE),
            Self::Char(..) => TypeInfo::StaticType(crate::runtime::static_type::CHAR_TYPE),
            Self::Bool(..) => TypeInfo::StaticType(crate::runtime::static_type::BOOL_TYPE),
            Self::String(..) => TypeInfo::StaticType(crate::runtime::static_type::STRING_TYPE),
            Self::Bytes(..) => TypeInfo::StaticType(crate::runtime::static_type::BYTES_TYPE),
            Self::Integer(..) => TypeInfo::StaticType(crate::runtime::static_type::INTEGER_TYPE),
            Self::Float(..) => TypeInfo::StaticType(crate::runtime::static_type::FLOAT_TYPE),
            Self::Vec(..) => TypeInfo::StaticType(crate::runtime::static_type::VEC_TYPE),
            Self::Tuple(..) => TypeInfo::StaticType(crate::runtime::static_type::TUPLE_TYPE),
            Self::Object(..) => TypeInfo::StaticType(crate::runtime::static_type::OBJECT_TYPE),
            Self::Option(..) => TypeInfo::StaticType(crate::runtime::static_type::OPTION_TYPE),
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
        VmResult::Ok(match value {
            Value::EmptyTuple => Self::EmptyTuple,
            Value::Byte(b) => Self::Byte(b),
            Value::Char(c) => Self::Char(c),
            Value::Bool(b) => Self::Bool(b),
            Value::Integer(n) => Self::Integer(n),
            Value::Float(f) => Self::Float(f),
            Value::String(s) => Self::String(vm_try!(s.take())),
            Value::Option(option) => Self::Option(match vm_try!(option.take()) {
                Some(some) => Some(vm_try!(Box::try_new(vm_try!(Self::from_value(some))))),
                None => None,
            }),
            Value::Bytes(b) => {
                let b = vm_try!(b.take());
                Self::Bytes(b)
            }
            Value::Vec(vec) => {
                let vec = vm_try!(vec.take());
                let mut const_vec = vm_try!(Vec::try_with_capacity(vec.len()));

                for value in vec {
                    vm_try!(const_vec.try_push(vm_try!(Self::from_value(value))));
                }

                Self::Vec(const_vec)
            }
            Value::Tuple(tuple) => {
                let tuple = vm_try!(tuple.take());
                let mut const_tuple = vm_try!(Vec::try_with_capacity(tuple.len()));

                for value in Vec::from(tuple.into_inner()) {
                    vm_try!(const_tuple.try_push(vm_try!(Self::from_value(value))));
                }

                Self::Tuple(vm_try!(const_tuple.try_into_boxed_slice()))
            }
            Value::Object(object) => {
                let object = vm_try!(object.take());
                let mut const_object = vm_try!(HashMap::try_with_capacity(object.len()));

                for (key, value) in object {
                    vm_try!(const_object.try_insert(key, vm_try!(Self::from_value(value))));
                }

                Self::Object(const_object)
            }
            value => {
                return VmResult::err(VmErrorKind::ConstNotSupported {
                    actual: vm_try!(value.type_info()),
                })
            }
        })
    }
}

impl ToValue for ConstValue {
    fn to_value(self) -> VmResult<Value> {
        VmResult::Ok(vm_try!(ConstValue::into_value(self)))
    }
}
