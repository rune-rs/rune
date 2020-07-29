use std::fmt;

/// Managed entries on the stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Managed {
    /// A string.
    String,
    /// An array.
    Array,
    /// An object.
    Object,
    /// Reference to an external type.
    External,
}

impl fmt::Display for Managed {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::String => write!(fmt, "string"),
            Self::Array => write!(fmt, "array"),
            Self::Object => write!(fmt, "object"),
            Self::External => write!(fmt, "external"),
        }
    }
}
