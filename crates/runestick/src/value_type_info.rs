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
    /// Reference to a foreign type.
    External(&'static str),
    /// The type of a value.
    Type(Hash),
    /// A pointer to the stack.
    Ptr,
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
}

impl fmt::Display for ValueTypeInfo {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ValueTypeInfo::Unit => {
                write!(fmt, "unit")?;
            }
            ValueTypeInfo::Bool => {
                write!(fmt, "bool")?;
            }
            ValueTypeInfo::Char => {
                write!(fmt, "char")?;
            }
            ValueTypeInfo::Byte => {
                write!(fmt, "byte")?;
            }
            ValueTypeInfo::Integer => {
                write!(fmt, "int")?;
            }
            ValueTypeInfo::Float => {
                write!(fmt, "float")?;
            }
            ValueTypeInfo::String => {
                write!(fmt, "String")?;
            }
            ValueTypeInfo::Bytes => {
                write!(fmt, "Bytes")?;
            }
            ValueTypeInfo::Vec => {
                write!(fmt, "Vec")?;
            }
            ValueTypeInfo::Tuple => {
                write!(fmt, "Tuple")?;
            }
            ValueTypeInfo::Object => {
                write!(fmt, "Object")?;
            }
            ValueTypeInfo::External(type_name) => {
                write!(fmt, "{}", type_name)?;
            }
            ValueTypeInfo::Type(hash) => {
                write!(fmt, "type({})", hash)?;
            }
            ValueTypeInfo::Ptr => {
                write!(fmt, "ptr")?;
            }
            ValueTypeInfo::Future => {
                write!(fmt, "future")?;
            }
            ValueTypeInfo::Option => {
                write!(fmt, "option")?;
            }
            ValueTypeInfo::Result => {
                write!(fmt, "result")?;
            }
            ValueTypeInfo::TypedObject(ty) => {
                write!(fmt, "typed-object({})", ty)?;
            }
            ValueTypeInfo::VariantObject(ty, variant_type) => {
                write!(fmt, "variant-object({}, {})", ty, variant_type)?;
            }
            ValueTypeInfo::TypedTuple(ty) => {
                write!(fmt, "typed-tuple({})", ty)?;
            }
            ValueTypeInfo::VariantTuple(ty, variant_type) => {
                write!(fmt, "variant-tuple({}, {})", ty, variant_type)?;
            }
        }

        Ok(())
    }
}
