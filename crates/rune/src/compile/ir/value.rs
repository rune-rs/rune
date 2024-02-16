use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, Box, HashMap, String, Vec};
use crate::ast::Spanned;
use crate::compile::{self, WithSpan};
use crate::runtime as rt;
use crate::runtime::{Bytes, ConstValue, Shared, TypeInfo};

/// A value governed by intermediate representation.
#[derive(Debug, TryClone)]
pub struct Value {
    inner: ValueKind,
}

impl Value {
    pub(crate) fn new(inner: ValueKind) -> Self {
        Self { inner }
    }

    pub(crate) fn kind(&self) -> &ValueKind {
        &self.inner
    }

    pub(crate) fn kind_mut(&mut self) -> &mut ValueKind {
        &mut self.inner
    }

    pub(crate) fn into_kind(self) -> ValueKind {
        self.inner
    }

    /// Try to coerce into boolean.
    pub(crate) fn into_bool(self) -> Result<bool, Self> {
        match self.inner {
            ValueKind::Bool(value) => Ok(value),
            inner => Err(Self { inner }),
        }
    }

    /// Convert a constant value into an interpreter value.
    pub(crate) fn from_const(value: &ConstValue) -> alloc::Result<Self> {
        let inner = match value {
            ConstValue::EmptyTuple => ValueKind::EmptyTuple,
            ConstValue::Byte(b) => ValueKind::Byte(*b),
            ConstValue::Char(c) => ValueKind::Char(*c),
            ConstValue::Bool(b) => ValueKind::Bool(*b),
            ConstValue::Integer(n) => ValueKind::Integer(*n),
            ConstValue::Float(n) => ValueKind::Float(*n),
            ConstValue::String(s) => ValueKind::String(Shared::new(s.try_clone()?)?),
            ConstValue::Bytes(b) => ValueKind::Bytes(Shared::new(b.try_clone()?)?),
            ConstValue::Option(option) => ValueKind::Option(Shared::new(match option {
                Some(some) => Some(Self::from_const(some)?),
                None => None,
            })?),
            ConstValue::Vec(vec) => {
                let mut ir_vec = Vec::try_with_capacity(vec.len())?;

                for value in vec {
                    ir_vec.try_push(Self::from_const(value)?)?;
                }

                ValueKind::Vec(Shared::new(ir_vec)?)
            }
            ConstValue::Tuple(tuple) => {
                let mut ir_tuple = Vec::try_with_capacity(tuple.len())?;

                for value in tuple.iter() {
                    ir_tuple.try_push(Self::from_const(value)?)?;
                }

                ValueKind::Tuple(Shared::new(ir_tuple.try_into_boxed_slice()?)?)
            }
            ConstValue::Object(object) => {
                let mut ir_object = HashMap::try_with_capacity(object.len())?;

                for (key, value) in object {
                    ir_object.try_insert(key.try_clone()?, Self::from_const(value)?)?;
                }

                ValueKind::Object(Shared::new(ir_object)?)
            }
        };

        Ok(Self { inner })
    }

    /// Convert into constant value.
    pub(crate) fn into_const<S>(self, spanned: S) -> compile::Result<ConstValue>
    where
        S: Copy + Spanned,
    {
        Ok(match self.inner {
            ValueKind::EmptyTuple => ConstValue::EmptyTuple,
            ValueKind::Byte(b) => ConstValue::Byte(b),
            ValueKind::Char(c) => ConstValue::Char(c),
            ValueKind::Bool(b) => ConstValue::Bool(b),
            ValueKind::Integer(n) => ConstValue::Integer(n),
            ValueKind::Float(f) => ConstValue::Float(f),
            ValueKind::String(s) => {
                let s = s.take().with_span(spanned)?;
                ConstValue::String(s)
            }
            ValueKind::Bytes(b) => {
                let b = b.take().with_span(spanned)?;
                ConstValue::Bytes(b)
            }
            ValueKind::Option(option) => {
                ConstValue::Option(match option.take().with_span(spanned)? {
                    Some(value) => Some(Box::try_new(value.into_const(spanned)?)?),
                    None => None,
                })
            }
            ValueKind::Vec(vec) => {
                let vec = vec.take().with_span(spanned)?;
                let mut const_vec = Vec::try_with_capacity(vec.len())?;

                for value in vec {
                    const_vec.try_push(value.into_const(spanned)?)?;
                }

                ConstValue::Vec(const_vec)
            }
            ValueKind::Tuple(tuple) => {
                let tuple = tuple.take().with_span(spanned)?;
                let mut const_tuple = Vec::try_with_capacity(tuple.len())?;

                for value in Vec::from(tuple) {
                    const_tuple.try_push(value.into_const(spanned)?)?;
                }

                ConstValue::Tuple(const_tuple.try_into_boxed_slice()?)
            }
            ValueKind::Object(object) => {
                let object = object.take().with_span(spanned)?;
                let mut const_object = HashMap::try_with_capacity(object.len())?;

                for (key, value) in object {
                    const_object.try_insert(key, value.into_const(spanned)?)?;
                }

                ConstValue::Object(const_object)
            }
        })
    }

    pub(crate) fn type_info(&self) -> TypeInfo {
        self.inner.type_info()
    }
}

#[derive(Debug, TryClone)]
pub(crate) enum ValueKind {
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

impl ValueKind {
    /// Get the type information of the value.
    pub(crate) fn type_info(&self) -> TypeInfo {
        match self {
            ValueKind::EmptyTuple => TypeInfo::StaticType(rt::static_type::TUPLE_TYPE),
            ValueKind::Byte(..) => TypeInfo::StaticType(rt::static_type::BYTE_TYPE),
            ValueKind::Char(..) => TypeInfo::StaticType(rt::static_type::CHAR_TYPE),
            ValueKind::Bool(..) => TypeInfo::StaticType(rt::static_type::BOOL_TYPE),
            ValueKind::String(..) => TypeInfo::StaticType(rt::static_type::STRING_TYPE),
            ValueKind::Bytes(..) => TypeInfo::StaticType(rt::static_type::BYTES_TYPE),
            ValueKind::Integer(..) => TypeInfo::StaticType(rt::static_type::INTEGER_TYPE),
            ValueKind::Float(..) => TypeInfo::StaticType(rt::static_type::FLOAT_TYPE),
            ValueKind::Option(..) => TypeInfo::StaticType(rt::static_type::OPTION_TYPE),
            ValueKind::Vec(..) => TypeInfo::StaticType(rt::static_type::VEC_TYPE),
            ValueKind::Tuple(..) => TypeInfo::StaticType(rt::static_type::TUPLE_TYPE),
            ValueKind::Object(..) => TypeInfo::StaticType(rt::static_type::OBJECT_TYPE),
        }
    }
}
