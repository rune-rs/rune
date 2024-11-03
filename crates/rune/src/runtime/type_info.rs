use core::fmt;
use core::hash;

use crate as rune;
use crate::alloc::prelude::*;
use crate::compile::Named;
use crate::hash::Hash;
use crate::{Any, TypeHash};

use ::rust_alloc::sync::Arc;

use super::Rtti;

#[derive(Debug, TryClone, PartialEq, Eq)]
enum TypeInfoKind {
    /// Reference to an external type.
    Any(AnyTypeInfo),
    /// A named type.
    Runtime(Arc<Rtti>),
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

    /// Construct type info from an statically known [`Named`] type.
    #[inline]
    pub const fn named<T>() -> Self
    where
        T: Named + TypeHash,
    {
        Self::any_type_info(AnyTypeInfo::new(T::full_name, T::HASH))
    }

    /// Construct type info from an statically known [`Any`] type.
    #[doc(hidden)]
    #[inline]
    pub(crate) const fn any_type_info(type_info: AnyTypeInfo) -> Self {
        Self::new(TypeInfoKind::Any(type_info))
    }

    #[inline]
    pub(crate) const fn rtti(rtti: Arc<Rtti>) -> Self {
        Self::new(TypeInfoKind::Runtime(rtti))
    }

    #[cfg(feature = "emit")]
    pub(crate) fn type_hash(&self) -> Hash {
        match &self.kind {
            TypeInfoKind::Any(ty) => ty.hash,
            TypeInfoKind::Runtime(ty) => ty.type_hash(),
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
            TypeInfoKind::Runtime(rtti) => {
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

/// Type information for the [`Any`][crate::Any] type.
#[derive(Debug, TryClone, Clone, Copy)]
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

impl PartialEq for AnyTypeInfo {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Eq for AnyTypeInfo {}

impl hash::Hash for AnyTypeInfo {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}
