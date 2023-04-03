use std::hash::{Hash as _, Hasher};

use crate::compile::{IntoComponent, ItemBuf};
use crate::hash::{Hash, TYPE};

/// Helper trait used to convert a type into a type hash.
///
/// This is used by [`Hash::type_hash`][crate::hash::Hash::type_hash] to get the
/// type hash of an object.
pub trait ToTypeHash {
    /// Generate a function hash.
    #[doc(hidden)]
    fn to_type_hash(&self) -> Hash;

    /// Optionally convert into an item, if appropriate.
    #[doc(hidden)]
    fn to_item(&self) -> Option<ItemBuf>;

    /// Hash the current value in-place.
    #[doc(hidden)]
    fn hash_type<H>(&self, hasher: &mut H)
    where
        H: Hasher;
}

impl<I> ToTypeHash for I
where
    I: Copy + IntoIterator,
    I::Item: IntoComponent,
{
    #[inline]
    fn to_type_hash(&self) -> Hash {
        let mut hasher = Hash::new_hasher();
        self.hash_type(&mut hasher);
        Hash::new(hasher.finish())
    }

    #[inline]
    fn to_item(&self) -> Option<ItemBuf> {
        Some(ItemBuf::with_item(*self))
    }

    #[inline]
    fn hash_type<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        TYPE.hash(hasher);

        for c in *self {
            c.hash_component(hasher);
        }
    }
}

impl ToTypeHash for Hash {
    #[inline]
    fn to_type_hash(&self) -> Hash {
        *self
    }

    #[inline]
    fn to_item(&self) -> Option<ItemBuf> {
        None
    }

    #[inline]
    fn hash_type<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.hash(hasher);
    }
}
