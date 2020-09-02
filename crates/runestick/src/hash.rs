use crate::{Component, ValueType};
use std::any;
use std::fmt;
use std::hash::{BuildHasher as _, BuildHasherDefault, Hash as _, Hasher as _};
use twox_hash::XxHash64;

const SEP: usize = 0x7f;
const TYPE: usize = 1;
const INSTANCE_FUNCTION: usize = 2;
const GETTER: usize = 3;
const OBJECT_KEYS: usize = 4;
const TYPE_ID: usize = 5;

/// The hash of a primitive thing.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hash(u64);

impl Hash {
    /// Construct a new raw hash.
    pub(crate) const fn new(hash: u64) -> Self {
        Self(hash)
    }

    /// Construct a hash to an instance function, where the instance is a
    /// pre-determined type.
    pub fn instance_function<N>(value_type: ValueType, name: N) -> Self
    where
        N: IntoTypeHash,
    {
        let name = name.into_type_hash();
        Self(Hash::of((INSTANCE_FUNCTION, value_type, SEP, name)).0)
    }

    /// Construct a hash corresponding to a getter.
    pub fn getter<N>(value_type: ValueType, name: N) -> Self
    where
        N: IntoTypeHash,
    {
        let name = name.into_type_hash();
        Self(Hash::of((GETTER, value_type, SEP, name)).0)
    }

    /// Construct a simple hash from something that is hashable.
    pub fn of<T: std::hash::Hash>(thing: T) -> Self {
        let mut hasher = BuildHasherDefault::<XxHash64>::default().build_hasher();
        thing.hash(&mut hasher);
        Self(hasher.finish())
    }

    /// Hash the given iterator of object keys.
    pub fn object_keys<I>(keys: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut hasher = BuildHasherDefault::<XxHash64>::default().build_hasher();
        OBJECT_KEYS.hash(&mut hasher);

        for key in keys {
            SEP.hash(&mut hasher);
            key.as_ref().hash(&mut hasher);
        }

        Self(hasher.finish())
    }

    /// Construct a hash for an use.
    fn path_hash<I>(kind: usize, path: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<Component>,
    {
        let mut hasher = BuildHasherDefault::<XxHash64>::default().build_hasher();
        kind.hash(&mut hasher);

        for part in path {
            part.into().hash(&mut hasher);
        }

        Self(hasher.finish())
    }

    /// Get the hash of a type.
    pub fn type_hash<I>(path: I) -> Self
    where
        I: IntoTypeHash,
    {
        path.into_type_hash()
    }

    /// Construct a type hash from an any type.
    pub fn of_any<T>() -> Self
    where
        T: any::Any,
    {
        Self::from_type_id(any::TypeId::of::<T>())
    }

    /// Construct a hash from a type id.
    pub fn from_type_id(type_id: any::TypeId) -> Self {
        Self::of((TYPE_ID, type_id))
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
pub trait IntoTypeHash {
    /// Generate a function hash.
    fn into_type_hash(self) -> Hash;
}

impl IntoTypeHash for Hash {
    fn into_type_hash(self) -> Hash {
        self
    }
}

impl<I> IntoTypeHash for I
where
    I: IntoIterator,
    I::Item: Into<Component>,
{
    fn into_type_hash(self) -> Hash {
        Hash::path_hash(TYPE, self)
    }
}
