use crate::{Hash, RawStr, StaticType};
use std::fmt;

/// Type information about a value, that can be printed for human consumption
/// through its [Display][fmt::Display] implementation.
#[derive(Debug, Clone, Copy)]
pub enum TypeInfo {
    /// The static type of a value.
    StaticType(&'static StaticType),
    /// The type hash of a value.
    Hash(Hash),
    /// Reference to an external type.
    Any(RawStr),
}

impl fmt::Display for TypeInfo {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::StaticType(ty) => {
                write!(fmt, "{}", ty.name)?;
            }
            Self::Hash(ty) => {
                write!(fmt, "Type({})", ty)?;
            }
            Self::Any(type_name) => {
                write!(fmt, "{}", type_name)?;
            }
        }

        Ok(())
    }
}
