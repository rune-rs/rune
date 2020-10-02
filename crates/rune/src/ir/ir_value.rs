use crate::collections::HashMap;
use crate::{IrError, Spanned};
use runestick::{ConstValue, Shared, TypeInfo};

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
    pub fn from_const(value: ConstValue) -> Self {
        match value {
            ConstValue::Unit => Self::Unit,
            ConstValue::Byte(b) => Self::Byte(b),
            ConstValue::Char(c) => Self::Char(c),
            ConstValue::Bool(b) => Self::Bool(b),
            ConstValue::Integer(n) => Self::Integer(n),
            ConstValue::Float(n) => Self::Float(n),
            ConstValue::String(s) => Self::String(Shared::new(s)),
            ConstValue::Bytes(b) => Self::Bytes(Shared::new(b)),
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
        Ok(match self {
            IrValue::Unit => ConstValue::Unit,
            IrValue::Byte(b) => ConstValue::Byte(b),
            IrValue::Char(c) => ConstValue::Char(c),
            IrValue::Bool(b) => ConstValue::Bool(b),
            IrValue::Integer(n) => ConstValue::Integer(n),
            IrValue::Float(f) => ConstValue::Float(f),
            IrValue::String(s) => {
                let s = s.take().map_err(IrError::access(spanned))?;
                ConstValue::String(s)
            }
            IrValue::Bytes(b) => {
                let b = b.take().map_err(IrError::access(spanned))?;
                ConstValue::Bytes(b)
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
            Self::Vec(..) => TypeInfo::StaticType(runestick::VEC_TYPE),
            Self::Tuple(..) => TypeInfo::StaticType(runestick::TUPLE_TYPE),
            Self::Object(..) => TypeInfo::StaticType(runestick::OBJECT_TYPE),
        }
    }
}
