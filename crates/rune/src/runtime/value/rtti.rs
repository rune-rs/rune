use core::borrow::Borrow;
use core::cmp::Ordering;
use core::hash;

use serde::{Deserialize, Serialize};

use crate::alloc::prelude::*;
use crate::alloc::HashMap;
use crate::runtime::Value;
use crate::{Hash, ItemBuf};

/// Field accessor for a variant struct.
#[doc(hidden)]
pub struct Accessor<'a> {
    pub(crate) fields: &'a HashMap<Box<str>, usize>,
    pub(crate) data: &'a [Value],
}

impl<'a> Accessor<'a> {
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

/// Runtime information on variant.
#[derive(Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Rtti {
    /// The type hash of the type.
    pub hash: Hash,
    /// If this type is a variant, designates the hash of the variant.
    pub variant_hash: Hash,
    /// The item of the type.
    pub item: ItemBuf,
    /// Mapping from field names to their corresponding indexes.
    pub fields: HashMap<Box<str>, usize>,
}

impl Rtti {
    /// Access a named field mutably from the given data.
    pub fn get_field<'a, Q>(&self, data: &'a [Value], key: &Q) -> Option<&'a Value>
    where
        Box<str>: Borrow<Q>,
        Q: hash::Hash + Eq + ?Sized,
    {
        data.get(*self.fields.get(key)?)
    }

    /// Access a named field immutably from the given data.
    pub fn get_field_mut<'a, Q>(&self, data: &'a mut [Value], key: &Q) -> Option<&'a mut Value>
    where
        Box<str>: Borrow<Q>,
        Q: hash::Hash + Eq + ?Sized,
    {
        data.get_mut(*self.fields.get(key)?)
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
