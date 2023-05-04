use crate::no_std::prelude::*;

use crate::ast::{Spanned, WithSpanExt};
use crate::collections::HashMap;
use crate::compile::{CompileError, IrErrorKind};
use crate::runtime as rt;
use crate::runtime::{Bytes, ConstValue, Shared, TypeInfo};

/// A constant value.
#[derive(Debug, Clone)]
pub enum IrValue {
    /// A constant unit.
    Unit,
    /// A byte.
    Byte(u8),
    /// A character.
    Char(char),
    /// A boolean constant value.
    Bool(bool),
    /// An integer constant.
    Integer(num::BigInt),
    /// An float constant.
    Float(f64),
    /// A string constant designated by its slot.
    String(Shared<String>),
    /// An optional value.
    Option(Shared<Option<IrValue>>),
    /// A byte string.
    Bytes(Shared<Bytes>),
    /// A vector of values.
    Vec(Shared<Vec<IrValue>>),
    /// An anonymous tuple.
    Tuple(Shared<Box<[IrValue]>>),
    /// An anonymous object.
    Object(Shared<HashMap<String, IrValue>>),
}

impl IrValue {
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
        T: TryFrom<num::BigInt>,
    {
        match self {
            Self::Integer(n) => T::try_from(n).ok(),
            _ => None,
        }
    }

    /// Convert a constant value into an interpreter value.
    pub(crate) fn from_const(value: &ConstValue) -> Self {
        match value {
            ConstValue::Unit => Self::Unit,
            ConstValue::Byte(b) => Self::Byte(*b),
            ConstValue::Char(c) => Self::Char(*c),
            ConstValue::Bool(b) => Self::Bool(*b),
            ConstValue::Integer(n) => Self::Integer((*n).into()),
            ConstValue::Float(n) => Self::Float(*n),
            ConstValue::String(s) => Self::String(Shared::new(s.clone())),
            ConstValue::StaticString(s) => Self::String(Shared::new((***s).to_owned())),
            ConstValue::Bytes(b) => Self::Bytes(Shared::new(b.clone())),
            ConstValue::Option(option) => Self::Option(Shared::new(
                option.as_ref().map(|some| Self::from_const(some)),
            )),
            ConstValue::Vec(vec) => {
                let mut ir_vec = Vec::with_capacity(vec.len());

                for value in vec {
                    ir_vec.push(Self::from_const(value));
                }

                Self::Vec(Shared::new(ir_vec))
            }
            ConstValue::Tuple(tuple) => {
                let mut ir_tuple = Vec::with_capacity(tuple.len());

                for value in tuple.iter() {
                    ir_tuple.push(Self::from_const(value));
                }

                Self::Tuple(Shared::new(ir_tuple.into_boxed_slice()))
            }
            ConstValue::Object(object) => {
                let mut ir_object = HashMap::with_capacity(object.len());

                for (key, value) in object {
                    ir_object.insert(key.clone(), Self::from_const(value));
                }

                Self::Object(Shared::new(ir_object))
            }
        }
    }

    /// Convert into constant value.
    pub(crate) fn into_const<S>(self, spanned: S) -> Result<ConstValue, CompileError>
    where
        S: Copy + Spanned,
    {
        use num::ToPrimitive as _;

        Ok(match self {
            IrValue::Unit => ConstValue::Unit,
            IrValue::Byte(b) => ConstValue::Byte(b),
            IrValue::Char(c) => ConstValue::Char(c),
            IrValue::Bool(b) => ConstValue::Bool(b),
            IrValue::Integer(n) => {
                let n = match n.clone().to_i64() {
                    Some(n) => n,
                    None => {
                        return Err(CompileError::new(
                            spanned,
                            IrErrorKind::NotInteger { value: n },
                        ))
                    }
                };

                ConstValue::Integer(n)
            }
            IrValue::Float(f) => ConstValue::Float(f),
            IrValue::String(s) => {
                let s = s.take().with_span(spanned)?;
                ConstValue::String(s)
            }
            IrValue::Bytes(b) => {
                let b = b.take().with_span(spanned)?;
                ConstValue::Bytes(b)
            }
            Self::Option(option) => ConstValue::Option(match option.take().with_span(spanned)? {
                Some(value) => Some(Box::new(value.into_const(spanned)?)),
                None => None,
            }),
            IrValue::Vec(vec) => {
                let vec = vec.take().with_span(spanned)?;
                let mut const_vec = Vec::with_capacity(vec.len());

                for value in vec {
                    const_vec.push(value.into_const(spanned)?);
                }

                ConstValue::Vec(const_vec)
            }
            IrValue::Tuple(tuple) => {
                let tuple = tuple.take().with_span(spanned)?;
                let mut const_tuple = Vec::with_capacity(tuple.len());

                for value in Vec::from(tuple) {
                    const_tuple.push(value.into_const(spanned)?);
                }

                ConstValue::Tuple(const_tuple.into_boxed_slice())
            }
            IrValue::Object(object) => {
                let object = object.take().with_span(spanned)?;
                let mut const_object = HashMap::with_capacity(object.len());

                for (key, value) in object {
                    const_object.insert(key, value.into_const(spanned)?);
                }

                ConstValue::Object(const_object)
            }
        })
    }

    /// Get the type information of the value.
    pub(crate) fn type_info(&self) -> TypeInfo {
        match self {
            Self::Unit => TypeInfo::StaticType(rt::UNIT_TYPE),
            Self::Byte(..) => TypeInfo::StaticType(rt::BYTE_TYPE),
            Self::Char(..) => TypeInfo::StaticType(rt::CHAR_TYPE),
            Self::Bool(..) => TypeInfo::StaticType(rt::BOOL_TYPE),
            Self::String(..) => TypeInfo::StaticType(rt::STRING_TYPE),
            Self::Bytes(..) => TypeInfo::StaticType(rt::BYTES_TYPE),
            Self::Integer(..) => TypeInfo::StaticType(rt::INTEGER_TYPE),
            Self::Float(..) => TypeInfo::StaticType(rt::FLOAT_TYPE),
            Self::Option(..) => TypeInfo::StaticType(rt::OPTION_TYPE),
            Self::Vec(..) => TypeInfo::StaticType(rt::VEC_TYPE),
            Self::Tuple(..) => TypeInfo::StaticType(rt::TUPLE_TYPE),
            Self::Object(..) => TypeInfo::StaticType(rt::OBJECT_TYPE),
        }
    }
}
