use core::hash::{Hash as _, Hasher as _};

#[cfg(feature = "alloc")]
use crate::alloc;
use crate::hash::Hash;
use crate::hash::TYPE;
use crate::item::{IntoComponent, ItemBuf};

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
    #[cfg(feature = "alloc")]
    fn to_item(&self) -> alloc::Result<Option<ItemBuf>>;
}

impl<I> ToTypeHash for I
where
    I: Copy + IntoIterator<Item: IntoComponent>,
{
    #[inline]
    fn to_type_hash(&self) -> Hash {
        let mut it = self.into_iter();

        let Some(first) = it.next() else {
            return Hash::EMPTY;
        };

        let mut hasher = Hash::new_hasher();

        TYPE.hash(&mut hasher);

        for c in [first].into_iter().chain(it) {
            c.hash_component(&mut hasher);
        }

        Hash::new(hasher.finish())
    }

    #[inline]
    #[cfg(feature = "alloc")]
    fn to_item(&self) -> alloc::Result<Option<ItemBuf>> {
        Ok(Some(ItemBuf::with_item(*self)?))
    }
}

impl ToTypeHash for Hash {
    #[inline]
    fn to_type_hash(&self) -> Hash {
        *self
    }

    #[inline]
    #[cfg(feature = "alloc")]
    fn to_item(&self) -> alloc::Result<Option<ItemBuf>> {
        Ok(None)
    }
}
