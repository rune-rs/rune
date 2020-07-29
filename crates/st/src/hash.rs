use crate::value::ValueType;
use std::fmt;
use std::hash::{BuildHasher as _, BuildHasherDefault, Hash as _, Hasher as _};
use twox_hash::XxHash64;

/// The hash of a primitive thing.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hash(u64);

impl Hash {
    /// Hash corresponding to global function calls.
    pub const GLOBAL_MODULE: Hash = Hash(0);

    const SEP: usize = 0x7f;
    const IMPORT: usize = 1;
    const GLOBAL_FN: usize = 2;
    const INSTANCE_FN: usize = 3;

    /// Construct a simple hash from something that is hashable.
    pub fn of<T: std::hash::Hash>(thing: T) -> Self {
        let mut hasher = BuildHasherDefault::<XxHash64>::default().build_hasher();
        thing.hash(&mut hasher);
        Self(hasher.finish())
    }

    /// Construct a hash for an import.
    pub fn module<I>(path: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut hasher = BuildHasherDefault::<XxHash64>::default().build_hasher();
        Self::IMPORT.hash(&mut hasher);

        for part in path {
            part.as_ref().hash(&mut hasher);
            Self::SEP.hash(&mut hasher);
        }

        Self(hasher.finish())
    }

    /// Construct a hash for a global free function.
    pub fn global_fn(name: &str) -> Self {
        Self::of((Self::GLOBAL_FN, name))
    }

    /// Construct a hash to an instance function, where the instance is a
    /// pre-determined type.
    pub fn instance_fn(ty: ValueType, name: Hash) -> Self {
        Self::of((Self::INSTANCE_FN, ty, Self::SEP, name))
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
