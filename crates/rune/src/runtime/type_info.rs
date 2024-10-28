use core::fmt;

use crate as rune;
use crate::alloc::prelude::*;
use crate::hash::Hash;
use crate::runtime::{RawStr, Rtti, StaticType, VariantRtti};
use crate::Any;

use ::rust_alloc::sync::Arc;

#[derive(Debug, TryClone, PartialEq, Eq)]
enum TypeInfoKind {
    /// The static type of a value.
    StaticType(&'static StaticType),
    /// Reference to an external type.
    Any(AnyTypeInfo),
    /// A named type.
    Typed(Arc<Rtti>),
    /// A variant.
    Variant(Arc<VariantRtti>),
}

/// Diagnostical type information for a given type.
///
/// Has reasonable [`Debug`] and [`Display`] implementations to identify a given
/// type.
///
/// [`Debug`]: core::fmt::Debug
/// [`Display`]: core::fmt::Display
#[derive(TryClone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TypeInfo {
    kind: TypeInfoKind,
}

impl TypeInfo {
    #[inline]
    const fn new(kind: TypeInfoKind) -> Self {
        Self { kind }
    }

    /// Construct type info from an statically known [`Any`] type.
    #[inline]
    pub const fn any<T>() -> Self
    where
        T: Any,
    {
        Self::any_type_info(T::INFO)
    }

    /// Construct type info from an statically known [`Any`] type.
    #[doc(hidden)]
    #[inline]
    pub(crate) const fn any_type_info(type_info: AnyTypeInfo) -> Self {
        Self::new(TypeInfoKind::Any(type_info))
    }

    #[doc(hidden)]
    pub(crate) const fn static_type(ty: &'static StaticType) -> Self {
        Self::new(TypeInfoKind::StaticType(ty))
    }

    #[inline]
    pub(crate) const fn typed(rtti: Arc<Rtti>) -> Self {
        Self::new(TypeInfoKind::Typed(rtti))
    }

    #[inline]
    pub(crate) const fn variant(rtti: Arc<VariantRtti>) -> Self {
        Self::new(TypeInfoKind::Variant(rtti))
    }

    #[cfg(feature = "emit")]
    pub(crate) fn type_hash(&self) -> Hash {
        match &self.kind {
            TypeInfoKind::StaticType(ty) => ty.hash,
            TypeInfoKind::Typed(ty) => ty.hash,
            TypeInfoKind::Variant(ty) => ty.hash,
            TypeInfoKind::Any(ty) => ty.hash,
        }
    }
}

impl fmt::Debug for TypeInfo {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl fmt::Display for TypeInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            TypeInfoKind::StaticType(ty) => {
                write!(f, "{}", ty.name)?;
            }
            TypeInfoKind::Typed(rtti) => {
                write!(f, "{}", rtti.item)?;
            }
            TypeInfoKind::Variant(rtti) => {
                write!(f, "{}", rtti.item)?;
            }
            TypeInfoKind::Any(info) => {
                write!(f, "{info}")?;
            }
        }

        Ok(())
    }
}

/// Type information for the [`Any`][crate::Any] type.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq)]
#[try_clone(copy)]
pub struct AnyTypeInfo {
    /// The name of the type.
    pub(crate) name: RawStr,
    /// The type hash of the item.
    pub(crate) hash: Hash,
}

impl AnyTypeInfo {
    /// Private constructor, use at your own risk.
    pub(crate) const fn new(name: RawStr, hash: Hash) -> Self {
        Self { name, hash }
    }
}

impl fmt::Display for AnyTypeInfo {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.name.fmt(f)
    }
}
