use crate::hash::Hash;
use std::fmt;

/// Type information about a value, that can be printed for human consumption
/// through its [Display][fmt::Display] implementation.
#[derive(Debug, Clone, Copy)]
pub enum ValueTypeInfo {
    /// An empty value indicating nothing.
    Unit,
    /// A boolean.
    Bool,
    /// A character.
    Char,
    /// A byte.
    Byte,
    /// A number.
    Integer,
    /// A float.
    Float,
    /// A string.
    String,
    /// Byte array.
    Bytes,
    /// A vecotr.
    Vec,
    /// A tuple.
    Tuple,
    /// An object.
    Object,
    /// The type of a value.
    Type(Hash),
    /// A future.
    Future,
    /// An optional value.
    Option,
    /// A result value.
    Result,
    /// A typed object.
    TypedObject(Hash),
    /// A typed object variant.
    VariantObject(Hash, Hash),
    /// A typed tuple.
    TypedTuple(Hash),
    /// A typed tuple variant.
    VariantTuple(Hash, Hash),
    /// A function pointer.
    FnPtr,
    /// Reference to a foreign type.
    Any(&'static str),
}

impl fmt::Display for ValueTypeInfo {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Unit => {
                write!(fmt, "unit")?;
            }
            Self::Bool => {
                write!(fmt, "bool")?;
            }
            Self::Char => {
                write!(fmt, "char")?;
            }
            Self::Byte => {
                write!(fmt, "byte")?;
            }
            Self::Integer => {
                write!(fmt, "int")?;
            }
            Self::Float => {
                write!(fmt, "float")?;
            }
            Self::String => {
                write!(fmt, "String")?;
            }
            Self::Bytes => {
                write!(fmt, "Bytes")?;
            }
            Self::Vec => {
                write!(fmt, "Vec")?;
            }
            Self::Tuple => {
                write!(fmt, "Tuple")?;
            }
            Self::Object => {
                write!(fmt, "Object")?;
            }
            Self::Type(hash) => {
                write!(fmt, "type({})", hash)?;
            }
            Self::Future => {
                write!(fmt, "future")?;
            }
            Self::Option => {
                write!(fmt, "option")?;
            }
            Self::Result => {
                write!(fmt, "result")?;
            }
            Self::TypedObject(ty) => {
                write!(fmt, "typed-object({})", ty)?;
            }
            Self::VariantObject(ty, variant_type) => {
                write!(fmt, "variant-object({}, {})", ty, variant_type)?;
            }
            Self::TypedTuple(ty) => {
                write!(fmt, "typed-tuple({})", ty)?;
            }
            Self::VariantTuple(ty, variant_type) => {
                write!(fmt, "variant-tuple({}, {})", ty, variant_type)?;
            }
            Self::FnPtr => {
                write!(fmt, "fn-ptr")?;
            }
            Self::Any(type_name) => {
                write!(fmt, "{}", type_name)?;
            }
        }

        Ok(())
    }
}
