use core::fmt;

use crate as rune;
use crate::alloc::prelude::*;
use crate::hash::Hash;
use crate::runtime::{RawStr, Rtti, StaticType, VariantRtti};
use ::rust_alloc::sync::Arc;

/// Type information about a value, that can be printed for human consumption
/// through its [Display][fmt::Display] implementation.
#[derive(Debug, TryClone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TypeInfo {
    /// The static type of a value.
    StaticType(&'static StaticType),
    /// A named type.
    Typed(Arc<Rtti>),
    /// A variant.
    Variant(Arc<VariantRtti>),
    /// Reference to an external type.
    Any(AnyTypeInfo),
}

impl TypeInfo {
    #[cfg(feature = "emit")]
    pub(crate) fn type_hash(&self) -> Hash {
        match self {
            TypeInfo::StaticType(ty) => ty.hash,
            TypeInfo::Typed(ty) => ty.hash,
            TypeInfo::Variant(ty) => ty.hash,
            TypeInfo::Any(ty) => ty.hash,
        }
    }
}

impl fmt::Display for TypeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaticType(ty) => {
                write!(f, "{}", ty.name)?;
            }
            Self::Typed(rtti) => {
                write!(f, "{}", rtti.item)?;
            }
            Self::Variant(rtti) => {
                write!(f, "{}", rtti.item)?;
            }
            Self::Any(info) => {
                write!(f, "{}", info.name)?;
            }
        }

        Ok(())
    }
}

/// Type information for the [`Any`][crate::Any] type.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq)]
#[try_clone(copy)]
#[non_exhaustive]
pub struct AnyTypeInfo {
    /// The name of the type.
    pub name: RawStr,
    /// The type hash of the item.
    pub hash: Hash,
}

impl AnyTypeInfo {
    /// Private constructor, use at your own risk.
    #[doc(hidden)]
    pub fn __private_new(name: RawStr, hash: Hash) -> Self {
        Self { name, hash }
    }
}
