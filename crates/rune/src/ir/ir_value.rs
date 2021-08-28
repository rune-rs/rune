use crate::collections::HashMap;
use crate::{IrError, IrErrorKind, Spanned};
use runestick::{Bytes, ConstValue, Shared, TypeInfo};
use std::convert::TryFrom;

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
    Bytes(Shared<Vec<u8>>),
    /// A vector of values.
    Vec(Shared<Vec<IrValue>>),
    /// An anonymous tuple.
    Tuple(Shared<Box<[IrValue]>>),
    /// An anonymous object.
    Object(Shared<HashMap<String, IrValue>>),
}

impl IrValue {
    /// Convert a constant value into an interpreter value.
    pub fn from_const(value: ConstValue) -> Self {
        match value {
            ConstValue::Unit => Self::Unit,
            ConstValue::Byte(b) => Self::Byte(b),
            ConstValue::Char(c) => Self::Char(c),
            ConstValue::Bool(b) => Self::Bool(b),
            ConstValue::Integer(n) => Self::Integer(n.into()),
            ConstValue::Float(n) => Self::Float(n),
            ConstValue::String(s) => Self::String(Shared::new(s)),
            ConstValue::StaticString(s) => Self::String(Shared::new((**s).to_owned())),
            ConstValue::Bytes(b) => Self::Bytes(Shared::new(b.into_vec())),
            ConstValue::Option(option) => {
                Self::Option(Shared::new(option.map(|some| Self::from_const(*some))))
            }
            ConstValue::Vec(vec) => {
                let mut ir_vec = Vec::with_capacity(vec.len());

                for value in vec {
                    ir_vec.push(Self::from_const(value));
                }

                Self::Vec(Shared::new(ir_vec))
            }
            ConstValue::Tuple(tuple) => {
                let mut ir_tuple = Vec::with_capacity(tuple.len());

                for value in Vec::from(tuple) {
                    ir_tuple.push(Self::from_const(value));
                }

                Self::Tuple(Shared::new(ir_tuple.into_boxed_slice()))
            }
            ConstValue::Object(object) => {
                let mut ir_object = HashMap::with_capacity(object.len());

                for (key, value) in object {
                    ir_object.insert(key, Self::from_const(value));
                }

                Self::Object(Shared::new(ir_object))
            }
        }
    }

    /// Convert into constant value.
    pub fn into_const<S>(self, spanned: S) -> Result<ConstValue, IrError>
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
                        return Err(IrError::new(spanned, IrErrorKind::NotInteger { value: n }))
                    }
                };

                ConstValue::Integer(n)
            }
            IrValue::Float(f) => ConstValue::Float(f),
            IrValue::String(s) => {
                let s = s.take().map_err(IrError::access(spanned))?;
                ConstValue::String(s)
            }
            IrValue::Bytes(b) => {
                let b = b.take().map_err(IrError::access(spanned))?;
                ConstValue::Bytes(Bytes::from(b))
            }
            Self::Option(option) => {
                ConstValue::Option(match option.take().map_err(IrError::access(spanned))? {
                    Some(value) => Some(Box::new(value.into_const(spanned)?)),
                    None => None,
                })
            }
            IrValue::Vec(vec) => {
                let vec = vec.take().map_err(IrError::access(spanned))?;
                let mut const_vec = Vec::with_capacity(vec.len());

                for value in vec {
                    const_vec.push(value.into_const(spanned)?);
                }

                ConstValue::Vec(const_vec)
            }
            IrValue::Tuple(tuple) => {
                let tuple = tuple.take().map_err(IrError::access(spanned))?;
                let mut const_tuple = Vec::with_capacity(tuple.len());

                for value in Vec::from(tuple) {
                    const_tuple.push(value.into_const(spanned)?);
                }

                ConstValue::Tuple(const_tuple.into_boxed_slice())
            }
            IrValue::Object(object) => {
                let object = object.take().map_err(IrError::access(spanned))?;
                let mut const_object = HashMap::with_capacity(object.len());

                for (key, value) in object {
                    const_object.insert(key, value.into_const(spanned)?);
                }

                ConstValue::Object(const_object)
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

    /// Get the type information of the value.
    pub fn type_info(&self) -> TypeInfo {
        match self {
            Self::Unit => TypeInfo::StaticType(runestick::UNIT_TYPE),
            Self::Byte(..) => TypeInfo::StaticType(runestick::BYTE_TYPE),
            Self::Char(..) => TypeInfo::StaticType(runestick::CHAR_TYPE),
            Self::Bool(..) => TypeInfo::StaticType(runestick::BOOL_TYPE),
            Self::String(..) => TypeInfo::StaticType(runestick::STRING_TYPE),
            Self::Bytes(..) => TypeInfo::StaticType(runestick::BYTES_TYPE),
            Self::Integer(..) => TypeInfo::StaticType(runestick::INTEGER_TYPE),
            Self::Float(..) => TypeInfo::StaticType(runestick::FLOAT_TYPE),
            Self::Option(..) => TypeInfo::StaticType(runestick::OPTION_TYPE),
            Self::Vec(..) => TypeInfo::StaticType(runestick::VEC_TYPE),
            Self::Tuple(..) => TypeInfo::StaticType(runestick::TUPLE_TYPE),
            Self::Object(..) => TypeInfo::StaticType(runestick::OBJECT_TYPE),
        }
    }
}
