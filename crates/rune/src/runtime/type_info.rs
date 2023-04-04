use std::fmt;
use std::sync::Arc;

use crate::hash::Hash;
use crate::runtime::{RawStr, Rtti, StaticType, VariantRtti};

/// Type information about a value, that can be printed for human consumption
/// through its [Display][fmt::Display] implementation.
#[derive(Debug, Clone)]
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

/// Type information for the [`Any`] type.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct AnyTypeInfo {
    /// The name of the type.
    pub name: RawStr,
    /// The hash of the type.
    #[cfg(feature = "doc")]
    #[allow(unused)]
    // TODO: will be used to lookup meta for a given type when generating documentation.
    pub(crate) hash: Hash,
}

impl AnyTypeInfo {
    /// Private constructor, use at your own risk.
    #[doc(hidden)]
    pub fn new(name: RawStr, #[cfg_attr(not(feature = "doc"), allow(unused))] hash: Hash) -> Self {
        Self {
            name,
            #[cfg(feature = "doc")]
            hash,
        }
    }

    /// Private constructor, use at your own risk which can optionally construct a hash from a provided function.
    #[doc(hidden)]
    pub fn new_from(
        name: RawStr,
        #[cfg_attr(not(feature = "doc"), allow(unused))] hash: fn() -> Hash,
    ) -> Self {
        Self {
            name,
            #[cfg(feature = "doc")]
            hash: hash(),
        }
    }
}
