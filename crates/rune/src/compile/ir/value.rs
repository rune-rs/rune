use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, Box, HashMap, String, Vec};
use crate::ast::Spanned;
use crate::compile::{self, WithSpan};
use crate::runtime as rt;
use crate::runtime::{Bytes, ConstValue, Shared, TypeInfo};

/// A constant value.
#[derive(Debug, TryClone)]
pub enum Value {
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
    String(Shared<String>),
    /// An optional value.
    Option(Shared<Option<Value>>),
    /// A byte string.
    Bytes(Shared<Bytes>),
    /// A vector of values.
    Vec(Shared<Vec<Value>>),
    /// An anonymous tuple.
    Tuple(Shared<Box<[Value]>>),
    /// An anonymous object.
    Object(Shared<HashMap<String, Value>>),
}

impl Value {
    /// Try to coerce into boolean.
    pub fn into_bool(self) -> Result<bool, Self> {
        match self {
            Self::Bool(value) => Ok(value),
            value => Err(value),
        }
    }

    /// Try to coerce into an integer of the specified type.
    pub fn into_integer<T>(self) -> Option<T>
    where
        T: TryFrom<i64>,
    {
        match self {
            Self::Integer(n) => T::try_from(n).ok(),
            _ => None,
        }
    }

    /// Convert a constant value into an interpreter value.
    pub(crate) fn from_const(value: &ConstValue) -> alloc::Result<Self> {
        Ok(match value {
            ConstValue::EmptyTuple => Self::EmptyTuple,
            ConstValue::Byte(b) => Self::Byte(*b),
            ConstValue::Char(c) => Self::Char(*c),
            ConstValue::Bool(b) => Self::Bool(*b),
            ConstValue::Integer(n) => Self::Integer(*n),
            ConstValue::Float(n) => Self::Float(*n),
            ConstValue::String(s) => Self::String(Shared::new(s.try_clone()?)?),
            ConstValue::Bytes(b) => Self::Bytes(Shared::new(b.try_clone()?)?),
            ConstValue::Option(option) => Self::Option(Shared::new(match option {
                Some(some) => Some(Self::from_const(some)?),
                None => None,
            })?),
            ConstValue::Vec(vec) => {
                let mut ir_vec = Vec::try_with_capacity(vec.len())?;

                for value in vec {
                    ir_vec.try_push(Self::from_const(value)?)?;
                }

                Self::Vec(Shared::new(ir_vec)?)
            }
            ConstValue::Tuple(tuple) => {
                let mut ir_tuple = Vec::try_with_capacity(tuple.len())?;

                for value in tuple.iter() {
                    ir_tuple.try_push(Self::from_const(value)?)?;
                }

                Self::Tuple(Shared::new(ir_tuple.try_into_boxed_slice()?)?)
            }
            ConstValue::Object(object) => {
                let mut ir_object = HashMap::try_with_capacity(object.len())?;

                for (key, value) in object {
                    ir_object.try_insert(key.try_clone()?, Self::from_const(value)?)?;
                }

                Self::Object(Shared::new(ir_object)?)
            }
        })
    }

    /// Convert into constant value.
    pub(crate) fn into_const<S>(self, spanned: S) -> compile::Result<ConstValue>
    where
        S: Copy + Spanned,
    {
        Ok(match self {
            Value::EmptyTuple => ConstValue::EmptyTuple,
            Value::Byte(b) => ConstValue::Byte(b),
            Value::Char(c) => ConstValue::Char(c),
            Value::Bool(b) => ConstValue::Bool(b),
            Value::Integer(n) => ConstValue::Integer(n),
            Value::Float(f) => ConstValue::Float(f),
            Value::String(s) => {
                let s = s.take().with_span(spanned)?;
                ConstValue::String(s)
            }
            Value::Bytes(b) => {
                let b = b.take().with_span(spanned)?;
                ConstValue::Bytes(b)
            }
            Self::Option(option) => ConstValue::Option(match option.take().with_span(spanned)? {
                Some(value) => Some(Box::try_new(value.into_const(spanned)?)?),
                None => None,
            }),
            Value::Vec(vec) => {
                let vec = vec.take().with_span(spanned)?;
                let mut const_vec = Vec::try_with_capacity(vec.len())?;

                for value in vec {
                    const_vec.try_push(value.into_const(spanned)?)?;
                }

                ConstValue::Vec(const_vec)
            }
            Value::Tuple(tuple) => {
                let tuple = tuple.take().with_span(spanned)?;
                let mut const_tuple = Vec::try_with_capacity(tuple.len())?;

                for value in Vec::from(tuple) {
                    const_tuple.try_push(value.into_const(spanned)?)?;
                }

                ConstValue::Tuple(const_tuple.try_into_boxed_slice()?)
            }
            Value::Object(object) => {
                let object = object.take().with_span(spanned)?;
                let mut const_object = HashMap::try_with_capacity(object.len())?;

                for (key, value) in object {
                    const_object.try_insert(key, value.into_const(spanned)?)?;
                }

                ConstValue::Object(const_object)
            }
        })
    }

    /// Get the type information of the value.
    pub(crate) fn type_info(&self) -> TypeInfo {
        match self {
            Self::EmptyTuple => TypeInfo::StaticType(rt::static_type::TUPLE_TYPE),
            Self::Byte(..) => TypeInfo::StaticType(rt::static_type::BYTE_TYPE),
            Self::Char(..) => TypeInfo::StaticType(rt::static_type::CHAR_TYPE),
            Self::Bool(..) => TypeInfo::StaticType(rt::static_type::BOOL_TYPE),
            Self::String(..) => TypeInfo::StaticType(rt::static_type::STRING_TYPE),
            Self::Bytes(..) => TypeInfo::StaticType(rt::static_type::BYTES_TYPE),
            Self::Integer(..) => TypeInfo::StaticType(rt::static_type::INTEGER_TYPE),
            Self::Float(..) => TypeInfo::StaticType(rt::static_type::FLOAT_TYPE),
            Self::Option(..) => TypeInfo::StaticType(rt::static_type::OPTION_TYPE),
            Self::Vec(..) => TypeInfo::StaticType(rt::static_type::VEC_TYPE),
            Self::Tuple(..) => TypeInfo::StaticType(rt::static_type::TUPLE_TYPE),
            Self::Object(..) => TypeInfo::StaticType(rt::static_type::OBJECT_TYPE),
        }
    }
}
