use core::borrow::Borrow;
use core::cmp::Ordering;
use core::hash;

#[cfg(feature = "musli")]
use musli_core::{Decode, Encode};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::alloc::prelude::*;
use crate::alloc::HashMap;
use crate::item::Item;
use crate::runtime::{FieldMap, TypeInfo, Value};
use crate::sync::Arc;
use crate::{Hash, ItemBuf};

/// Field accessor for a variant struct.
#[doc(hidden)]
pub struct Accessor<'a> {
    pub(crate) fields: &'a HashMap<Box<str>, usize>,
    pub(crate) data: &'a [Value],
}

impl Accessor<'_> {
    /// Get a field through the accessor.
    #[doc(hidden)]
    pub fn get<Q>(&self, key: &Q) -> Option<&Value>
    where
        Box<str>: Borrow<Q>,
        Q: hash::Hash + Eq + ?Sized,
    {
        self.data.get(*self.fields.get(key)?)
    }
}

/// The kind of value stored.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(rename_all = "kebab-case")
)]
#[cfg_attr(feature = "musli", derive(Decode, Encode), musli(crate = musli_core))]
pub(crate) enum RttiKind {
    /// The value stored is empty.
    Empty,
    /// The value stored is a tuple.
    Tuple,
    /// The value stored is a strict.
    Struct,
}

/// Runtime information on variant.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "musli", derive(Decode, Encode), musli(crate = musli_core))]
#[non_exhaustive]
pub struct Rtti {
    /// The kind of value.
    pub(crate) kind: RttiKind,
    /// The type hash of the type.
    pub(crate) hash: Hash,
    /// If this type is a variant, designates the hash of the variant.
    pub(crate) variant_hash: Hash,
    /// The item of the type.
    pub(crate) item: ItemBuf,
    /// Mapping from field names to their corresponding indexes.
    pub(crate) fields: FieldMap<Box<str>, usize>,
}

impl Rtti {
    /// Test if this RTTI matches the given raw hashes.
    #[inline]
    pub(crate) fn is(&self, hash: Hash, variant_hash: Hash) -> bool {
        self.hash == hash && self.variant_hash == variant_hash
    }

    /// Access the item of the RTTI.
    #[inline]
    pub fn item(&self) -> &Item {
        &self.item
    }

    /// Access the type hash of the RTTI.
    #[inline]
    pub fn type_hash(&self) -> Hash {
        self.hash
    }

    /// Access the type information for the RTTI.
    #[inline]
    pub fn type_info(this: Arc<Self>) -> TypeInfo {
        TypeInfo::rtti(this)
    }
}

impl PartialEq for Rtti {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash && self.variant_hash == other.variant_hash
    }
}

impl Eq for Rtti {}

impl hash::Hash for Rtti {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
        self.variant_hash.hash(state);
    }
}

impl PartialOrd for Rtti {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Rtti {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.hash
            .cmp(&other.hash)
            .then_with(|| self.variant_hash.cmp(&other.variant_hash))
    }
}
