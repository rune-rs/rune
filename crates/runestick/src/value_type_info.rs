use crate::hash::Hash;
use crate::StaticType;
use std::fmt;

/// Type information about a value, that can be printed for human consumption
/// through its [Display][fmt::Display] implementation.
#[derive(Debug, Clone, Copy)]
pub enum ValueTypeInfo {
    /// The static type of a value.
    StaticType(&'static StaticType),
    /// The type of a value.
    Type(Hash),
    /// Reference to a foreign type.
    Any(&'static str),
}

impl fmt::Display for ValueTypeInfo {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::StaticType(ty) => {
                write!(fmt, "type({})", ty.name)?;
            }
            Self::Type(ty) => {
                write!(fmt, "type({})", ty)?;
            }
            Self::Any(type_name) => {
                write!(fmt, "{}", type_name)?;
            }
        }

        Ok(())
    }
}
