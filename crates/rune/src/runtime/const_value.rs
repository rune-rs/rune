use serde::{Deserialize, Serialize};

use crate::alloc::prelude::*;
use crate::alloc::{self, Box, HashMap, String, Vec};
use crate::runtime::{
    self, BorrowRefRepr, Bytes, FromValue, Inline, Mutable, Object, OwnedTuple, ToValue, TypeInfo,
    Value, VmErrorKind, VmResult,
};
use crate::TypeHash;

/// A constant value.
#[derive(Debug, Deserialize, Serialize)]
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
    Vec(Vec<ConstValue>),
    /// An anonymous tuple.
    Tuple(Box<[ConstValue]>),
    /// An anonymous object.
    Object(HashMap<String, ConstValue>),
    /// An option.
    Option(Option<Box<ConstValue>>),
}

impl ConstValue {
    /// Construct a constant value from a reference to a value..
    pub(crate) fn from_value_ref(value: &Value) -> VmResult<ConstValue> {
        VmResult::Ok(match vm_try!(value.borrow_ref_repr()) {
            BorrowRefRepr::Inline(value) => match *value {
                Inline::Unit => Self::Unit,
                Inline::Byte(value) => Self::Byte(value),
                Inline::Char(value) => Self::Char(value),
                Inline::Bool(value) => Self::Bool(value),
                Inline::Integer(value) => Self::Integer(value),
                Inline::Float(value) => Self::Float(value),
                ref actual => {
                    return VmResult::err(VmErrorKind::ConstNotSupported {
                        actual: actual.type_info(),
                    })
                }
            },
            BorrowRefRepr::Mutable(value) => match &*value {
                Mutable::Option(option) => Self::Option(match option {
                    Some(some) => Some(vm_try!(Box::try_new(vm_try!(Self::from_value_ref(some))))),
                    None => None,
                }),
                Mutable::Vec(ref vec) => {
                    let mut const_vec = vm_try!(Vec::try_with_capacity(vec.len()));

                    for value in vec {
                        vm_try!(const_vec.try_push(vm_try!(Self::from_value_ref(value))));
                    }

                    Self::Vec(const_vec)
                }
                Mutable::Tuple(ref tuple) => {
                    let mut const_tuple = vm_try!(Vec::try_with_capacity(tuple.len()));

                    for value in tuple.iter() {
                        vm_try!(const_tuple.try_push(vm_try!(Self::from_value_ref(value))));
                    }

                    Self::Tuple(vm_try!(const_tuple.try_into_boxed_slice()))
                }
                Mutable::Object(ref object) => {
                    let mut const_object = vm_try!(HashMap::try_with_capacity(object.len()));

                    for (key, value) in object {
                        let key = vm_try!(key.try_clone());
                        let value = vm_try!(Self::from_value_ref(value));
                        vm_try!(const_object.try_insert(key, value));
                    }

                    Self::Object(const_object)
                }
                value => {
                    return VmResult::err(VmErrorKind::ConstNotSupported {
                        actual: value.type_info(),
                    })
                }
            },
            BorrowRefRepr::Any(value) => match value.type_hash() {
                String::HASH => {
                    let s = vm_try!(value.borrow_ref::<String>());
                    Self::String(vm_try!(s.try_to_owned()))
                }
                Bytes::HASH => {
                    let s = vm_try!(value.borrow_ref::<Bytes>());
                    Self::Bytes(vm_try!(s.try_to_owned()))
                }
                _ => {
                    return VmResult::err(VmErrorKind::ConstNotSupported {
                        actual: value.type_info(),
                    });
                }
            },
        })
    }

    /// Convert into virtual machine value.
    ///
    /// We provide this associated method since a constant value can be
    /// converted into a value infallibly, which is not captured by the trait
    /// otherwise.
    pub(crate) fn to_value(&self) -> alloc::Result<Value> {
        Ok(match self {
            Self::Unit => Value::unit(),
            Self::Byte(b) => Value::from(*b),
            Self::Char(c) => Value::from(*c),
            Self::Bool(b) => Value::from(*b),
            Self::Integer(n) => Value::from(*n),
            Self::Float(n) => Value::from(*n),
            Self::String(string) => Value::try_from(string.try_clone()?)?,
            Self::Bytes(b) => Value::try_from(b.try_clone()?)?,
            Self::Option(option) => Value::try_from(match option {
                Some(some) => Some(Self::to_value(some)?),
                None => None,
            })?,
            Self::Vec(vec) => {
                let mut v = runtime::Vec::with_capacity(vec.len())?;

                for value in vec {
                    v.push(Self::to_value(value)?)?;
                }

                Value::try_from(v)?
            }
            Self::Tuple(tuple) => {
                let mut t = Vec::try_with_capacity(tuple.len())?;

                for value in tuple.iter() {
                    t.try_push(Self::to_value(value)?)?;
                }

                Value::try_from(OwnedTuple::try_from(t)?)?
            }
            Self::Object(object) => {
                let mut o = Object::with_capacity(object.len())?;

                for (key, value) in object {
                    let key = key.try_clone()?;
                    let value = Self::to_value(value)?;
                    o.insert(key, value)?;
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
            Self::Unit => TypeInfo::static_type(crate::runtime::static_type::TUPLE),
            Self::Byte(..) => TypeInfo::static_type(crate::runtime::static_type::BYTE),
            Self::Char(..) => TypeInfo::static_type(crate::runtime::static_type::CHAR),
            Self::Bool(..) => TypeInfo::static_type(crate::runtime::static_type::BOOL),
            Self::String(..) => TypeInfo::static_type(crate::runtime::static_type::STRING),
            Self::Bytes(..) => TypeInfo::static_type(crate::runtime::static_type::BYTES),
            Self::Integer(..) => TypeInfo::static_type(crate::runtime::static_type::INTEGER),
            Self::Float(..) => TypeInfo::static_type(crate::runtime::static_type::FLOAT),
            Self::Vec(..) => TypeInfo::static_type(crate::runtime::static_type::VEC),
            Self::Tuple(..) => TypeInfo::static_type(crate::runtime::static_type::TUPLE),
            Self::Object(..) => TypeInfo::static_type(crate::runtime::static_type::OBJECT),
            Self::Option(..) => TypeInfo::static_type(crate::runtime::static_type::OPTION),
        }
    }
}

impl TryClone for ConstValue {
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(match self {
            ConstValue::Unit => ConstValue::Unit,
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
        ConstValue::from_value_ref(&value)
    }
}

impl ToValue for ConstValue {
    fn to_value(self) -> VmResult<Value> {
        VmResult::Ok(vm_try!(ConstValue::to_value(&self)))
    }
}
