use core::fmt;

use crate::no_std::sync::Arc;
use crate::runtime::{RawStr, Rtti, StaticType, VariantRtti};

/// Type information about a value, that can be printed for human consumption
/// through its [Display][fmt::Display] implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TypeInfo {
    /// The static type of a value.
    StaticType(&'static StaticType),
    /// Reference to an external type.
    Any(AnyTypeInfo),
    /// A named type.
    Typed(Arc<Rtti>),
    /// A variant.
    Variant(Arc<VariantRtti>),
}

impl fmt::Display for TypeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaticType(ty) => {
                write!(f, "{}", ty.name)?;
            }
            Self::Any(info) => {
                write!(f, "{}", info.name)?;
            }
            Self::Typed(rtti) => {
                write!(f, "{}", rtti.item)?;
            }
            Self::Variant(rtti) => {
                write!(f, "{}", rtti.item)?;
            }
        }

        Ok(())
    }
}

/// Type information for the [`Any`][crate::Any] type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct AnyTypeInfo {
    /// The name of the type.
    pub name: RawStr,
}

impl AnyTypeInfo {
    /// Private constructor, use at your own risk.
    #[doc(hidden)]
    pub fn new(name: RawStr) -> Self {
        Self { name }
    }
}
