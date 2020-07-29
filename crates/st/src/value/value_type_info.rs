use std::fmt;

/// Type information about a value, that can be printed for human consumption
/// through its [Display][fmt::Display] implementation.
#[derive(Debug, Clone, Copy)]
pub enum ValueTypeInfo {
    /// An empty unit.
    Unit,
    /// A string.
    String,
    /// An array.
    Array,
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
}

impl fmt::Display for ValueTypeInfo {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Unit => write!(fmt, "()"),
            Self::String => write!(fmt, "String"),
            Self::Array => write!(fmt, "Array"),
            Self::Integer => write!(fmt, "Integer"),
            Self::Float => write!(fmt, "Float"),
            Self::Bool => write!(fmt, "Bool"),
            Self::Char => write!(fmt, "Char"),
            Self::External(name) => write!(fmt, "External({})", name),
        }
    }
}
