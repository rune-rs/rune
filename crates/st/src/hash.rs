use crate::value::ValueType;
use std::fmt;

/// The hash of a primitive thing.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hash(u64);

impl Hash {
    const SEP: usize = 0x7f;
    const GLOBAL_FN: usize = 1;
    const INSTANCE_FN: usize = 2;

    /// Construct a simple hash from something that is hashable.
    pub fn of<T: std::hash::Hash>(thing: T) -> Self {
        use std::hash::{BuildHasher as _, BuildHasherDefault, Hasher as _};
        use twox_hash::XxHash64;

        let mut hasher = BuildHasherDefault::<XxHash64>::default().build_hasher();
        thing.hash(&mut hasher);
        Self(hasher.finish())
    }

    /// Construct a hash for a global free function.
    pub fn global_fn(name: &str) -> Self {
        Self::of((Self::GLOBAL_FN, name))
    }

    /// Construct a hash to an instance function, where the instance is a
    /// pre-determined type.
    pub fn instance_fn(ty: ValueType, name: &str) -> Self {
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
