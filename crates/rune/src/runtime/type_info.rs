use core::fmt;

use crate as rune;
use crate::alloc::prelude::*;
use crate::hash::Hash;
use crate::Any;

use ::rust_alloc::sync::Arc;

use super::{Rtti, StaticType, StaticTypeInfo, StaticTypeInfoKind, VariantRtti};

#[derive(Debug, TryClone, PartialEq, Eq)]
enum TypeInfoKind {
    /// The static type of a value.
    StaticType(StaticType),
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
        Self::any_type_info(T::ANY_TYPE_INFO)
    }

    /// Construct type info from an statically known [`Any`] type.
    #[doc(hidden)]
    #[inline]
    pub(crate) const fn any_type_info(type_info: AnyTypeInfo) -> Self {
        Self::new(TypeInfoKind::Any(type_info))
    }

    #[doc(hidden)]
    pub(crate) const fn static_type(ty: StaticType) -> Self {
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
                write!(f, "{}", ty.item)?;
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

impl From<AnyTypeInfo> for TypeInfo {
    #[inline]
    fn from(type_info: AnyTypeInfo) -> Self {
        Self::any_type_info(type_info)
    }
}

impl From<StaticTypeInfo> for TypeInfo {
    #[inline]
    fn from(type_info: StaticTypeInfo) -> Self {
        match type_info.into_kind() {
            StaticTypeInfoKind::StaticType(static_type) => Self::static_type(static_type),
            StaticTypeInfoKind::AnyTypeInfo(any_type_info) => Self::any_type_info(any_type_info),
        }
    }
}

/// Type information for the [`Any`][crate::Any] type.
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq)]
#[try_clone(copy)]
pub struct AnyTypeInfo {
    /// Formatter to display a full name.
    pub(crate) full_name: FullNameFn,
    /// The type hash of the item.
    pub(crate) hash: Hash,
}

impl AnyTypeInfo {
    /// Private constructor, use at your own risk.
    pub(crate) const fn new(full_name: FullNameFn, hash: Hash) -> Self {
        Self { full_name, hash }
    }
}

impl fmt::Display for AnyTypeInfo {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.full_name)(f)
    }
}

pub type FullNameFn = fn(&mut fmt::Formatter<'_>) -> fmt::Result;
