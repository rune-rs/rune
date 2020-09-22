use crate::TypeInfo;

/// A constant value.
#[derive(Debug, Clone)]
pub enum ConstValue {
    /// A constant unit.
    Unit,
    /// A boolean constant value.
    Bool(bool),
    /// A string constant designated by its slot.
    String(Box<str>),
    /// An integer constant.
    Integer(i64),
    /// An float constant.
    Float(f64),
    /// A vector of values.
    Vec(Box<[ConstValue]>),
    /// An anonymous tuple.
    Tuple(Box<[ConstValue]>),
}

impl ConstValue {
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
            Self::Bool(..) => TypeInfo::StaticType(crate::BOOL_TYPE),
            Self::String(..) => TypeInfo::StaticType(crate::STRING_TYPE),
            Self::Integer(..) => TypeInfo::StaticType(crate::INTEGER_TYPE),
            Self::Float(..) => TypeInfo::StaticType(crate::FLOAT_TYPE),
            Self::Vec(..) => TypeInfo::StaticType(crate::VEC_TYPE),
            Self::Tuple(..) => TypeInfo::StaticType(crate::TUPLE_TYPE),
        }
    }
}
