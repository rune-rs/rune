use crate::runtime::{RawStr, Rtti, StaticType, VariantRtti};
use std::fmt;
use std::sync::Arc;

/// Type information about a value, that can be printed for human consumption
/// through its [Display][fmt::Display] implementation.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum TypeInfo {
    /// The static type of a value.
    StaticType(&'static StaticType),
    /// Reference to an external type.
    Any(RawStr),
    /// A named type.
    Typed(Arc<Rtti>),
    /// A variant.
    Variant(Arc<VariantRtti>),
}

impl fmt::Display for TypeInfo {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaticType(ty) => {
                write!(fmt, "{}", ty.name)?;
            }
            Self::Any(type_name) => {
                write!(fmt, "{}", *type_name)?;
            }
            Self::Typed(rtti) => {
                write!(fmt, "{}", rtti.item)?;
            }
            Self::Variant(rtti) => {
                write!(fmt, "{}", rtti.item)?;
            }
        }

        Ok(())
    }
}
