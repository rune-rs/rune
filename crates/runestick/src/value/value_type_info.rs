use crate::hash::Hash;
use std::fmt;

/// Type information about a value, that can be printed for human consumption
/// through its [Display][fmt::Display] implementation.
#[derive(Debug, Clone, Copy)]
pub enum ValueTypeInfo {
    /// An empty value indicating nothing.
    Unit,
    /// A string.
    String,
    /// An array.
    Array,
    /// A tuple.
    Tuple,
    /// An object.
    Object,
    /// A number.
    Integer,
    /// A float.
    Float,
    /// A boolean.
    Bool,
    /// A character.
    Char,
    /// Reference to a foreign type.
    External(&'static str),
    /// The type of a value.
    Type,
    /// A pointer to the stack.
    Ptr,
    /// A function.
    Fn(Hash),
    /// A future.
    Future,
    /// An optional value.
    Option,
    /// A result value.
    Result,
}

impl fmt::Display for ValueTypeInfo {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ValueTypeInfo::Unit => {
                write!(fmt, "unit")?;
            }
            ValueTypeInfo::String => {
                write!(fmt, "String")?;
            }
            ValueTypeInfo::Array => {
                write!(fmt, "Array")?;
            }
            ValueTypeInfo::Tuple => {
                write!(fmt, "Tuple")?;
            }
            ValueTypeInfo::Object => {
                write!(fmt, "Object")?;
            }
            ValueTypeInfo::Integer => {
                write!(fmt, "int")?;
            }
            ValueTypeInfo::Float => {
                write!(fmt, "float")?;
            }
            ValueTypeInfo::Bool => {
                write!(fmt, "bool")?;
            }
            ValueTypeInfo::Char => {
                write!(fmt, "char")?;
            }
            ValueTypeInfo::External(type_name) => {
                write!(fmt, "{}", type_name)?;
            }
            ValueTypeInfo::Type => {
                write!(fmt, "type")?;
            }
            ValueTypeInfo::Ptr => {
                write!(fmt, "ptr")?;
            }
            ValueTypeInfo::Fn(hash) => {
                write!(fmt, "fn({})", hash)?;
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
        }

        Ok(())
    }
}
