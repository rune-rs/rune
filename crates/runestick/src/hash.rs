use crate::{Any, IntoComponent, Item, Type};
use serde::{Deserialize, Serialize};
use std::any;
use std::fmt;
use std::hash;
use std::hash::{BuildHasher as _, BuildHasherDefault, Hash as _, Hasher as _};
use std::mem;
use twox_hash::XxHash64;

const SEP: usize = 0x7f;
const TYPE: usize = 1;
const INSTANCE_FUNCTION: usize = 2;
const GETTER: usize = 3;
const OBJECT_KEYS: usize = 4;

/// The hash of a primitive thing.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Hash(u64);

impl Hash {
    /// Construct a new raw hash.
    pub(crate) const fn new(hash: u64) -> Self {
        Self(hash)
    }

    /// Construct a hash from the given type id.
    pub fn from_any<T>() -> Self
    where
        T: Any,
    {
        Self::from_type_id(any::TypeId::of::<T>())
    }

    /// Construct a hash from a type id.
    pub fn from_type_id(type_id: any::TypeId) -> Self {
        // Safety: a type id is exactly a 64-bit unsigned integer.
        // And has an identical bit pattern to `Hash`.
        unsafe { mem::transmute(type_id) }
    }

    /// Construct a hash to an instance function, where the instance is a
    /// pre-determined type.
    pub fn instance_function<N>(type_of: Type, name: N) -> Self
    where
        N: IntoHash,
    {
        let name = name.into_hash();
        Self(Hash::of((INSTANCE_FUNCTION, type_of, SEP, name)).0)
    }

    /// Construct a hash corresponding to a getter.
    pub fn getter<N>(type_of: Type, name: N) -> Self
    where
        N: IntoHash,
    {
        let name = name.into_hash();
        Self(Hash::of((GETTER, type_of, SEP, name)).0)
    }

    /// Construct a simple hash from something that is hashable.
    pub fn of<T: hash::Hash>(thing: T) -> Self {
        let mut hasher = Self::new_hasher();
        thing.hash(&mut hasher);
        Self(hasher.finish())
    }

    /// Hash the given iterator of object keys.
    pub fn object_keys<I>(keys: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut hasher = Self::new_hasher();
        OBJECT_KEYS.hash(&mut hasher);

        for key in keys {
            SEP.hash(&mut hasher);
            key.as_ref().hash(&mut hasher);
        }

        Self(hasher.finish())
    }

    /// Get the hash of a type.
    pub fn type_hash<I>(path: I) -> Self
    where
        I: IntoHash,
    {
        path.into_hash()
    }

    /// Construct a new hasher.
    fn new_hasher() -> impl hash::Hasher {
        BuildHasherDefault::<XxHash64>::default().build_hasher()
    }

    /// Construct a hash for an use.
    fn path_hash<I>(kind: usize, path: I) -> Self
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        let mut hasher = Self::new_hasher();
        kind.hash(&mut hasher);

        for c in path {
            c.hash_component(&mut hasher);
        }

        Self(hasher.finish())
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "0x{:x}", self.0)
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "Hash(0x{:x})", self.0)
    }
}

/// Helper conversion into a function hash.
pub trait IntoHash: Copy {
    /// Generate a function hash.
    fn into_hash(self) -> Hash;

    /// Optionally convert into an item, if appropriate.
    fn into_item(self) -> Item;
}

impl IntoHash for Hash {
    fn into_hash(self) -> Hash {
        self
    }

    fn into_item(self) -> Item {
        Item::new()
    }
}

impl<I> IntoHash for I
where
    I: Copy + IntoIterator,
    I::Item: IntoComponent,
{
    fn into_hash(self) -> Hash {
        Hash::path_hash(TYPE, self)
    }

    fn into_item(self) -> Item {
        Item::of(self)
    }
}
