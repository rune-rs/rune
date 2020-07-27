use crate::value::ValueType;
use std::fmt;

/// The hash of a primitive thing.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hash(u64);

impl Hash {
    /// Construct a simple hash from something that is hashable.
    pub fn of<T: std::hash::Hash>(thing: T) -> Self {
        use std::hash::{BuildHasher as _, BuildHasherDefault, Hasher as _};
        use twox_hash::XxHash64;

        let mut hasher = BuildHasherDefault::<XxHash64>::default().build_hasher();
        thing.hash(&mut hasher);
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

/// The hash of a dynamic method.
///
/// It is simply determined by its name and number of arguments and is
/// constructed through the [of][FnDynamicHash::of] function.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FnDynamicHash(u64);

impl fmt::Display for FnDynamicHash {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "0x{:x}", self.0)
    }
}

impl fmt::Debug for FnDynamicHash {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "FnDynamicHash(0x{:x})", self.0)
    }
}

impl FnDynamicHash {
    const MARKER_ARGS: usize = 0;

    /// Construct a function hash.
    pub fn of(hash: Hash, args: usize) -> Self {
        use std::hash::{BuildHasher as _, BuildHasherDefault, Hash as _, Hasher as _};
        use twox_hash::XxHash64;

        let mut hasher = BuildHasherDefault::<XxHash64>::default().build_hasher();

        hash.hash(&mut hasher);
        Self::MARKER_ARGS.hash(&mut hasher);
        args.hash(&mut hasher);
        Self(hasher.finish())
    }
}

/// The hash of a function handler.
///
/// This is calculated as the hash of:
/// * The function name
/// * The function arguments
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FnHash(u64);

impl FnHash {
    const MARKER_FIXED: usize = 10;
    const MARKER_RAW: usize = 20;
    const MARKER_ARG: usize = 30;

    /// Construct a function hash.
    pub fn of(name: &str, args: &[ValueType]) -> Self {
        let hash = Hash::of(name);
        let hash = FnDynamicHash::of(hash, args.len());
        Self::of_dynamic(hash, args.iter().copied())
    }

    /// Register a raw function of the given name.
    ///
    /// If these are present, they take precedence over all other functions.
    pub fn raw(hash: Hash) -> Self {
        use std::hash::{BuildHasher as _, BuildHasherDefault, Hash as _, Hasher as _};
        use twox_hash::XxHash64;

        let mut hasher = BuildHasherDefault::<XxHash64>::default().build_hasher();
        Self::MARKER_RAW.hash(&mut hasher);
        hash.hash(&mut hasher);
        Self(hasher.finish())
    }

    /// Construct a function hash based on a dynamic one.
    pub fn of_dynamic<I>(hash: FnDynamicHash, args: I) -> Self
    where
        I: IntoIterator<Item = ValueType>,
    {
        use std::hash::{BuildHasher as _, BuildHasherDefault, Hash as _, Hasher as _};
        use twox_hash::XxHash64;

        let mut hasher = BuildHasherDefault::<XxHash64>::default().build_hasher();
        Self::MARKER_FIXED.hash(&mut hasher);
        hash.hash(&mut hasher);

        for arg in args {
            Self::MARKER_ARG.hash(&mut hasher);
            arg.hash(&mut hasher);
        }

        Self(hasher.finish())
    }

    /// Construct a function hash based on a dynamic one.
    pub fn of_dynamic_fallible<I, E>(hash: FnDynamicHash, args: I) -> Result<Self, E>
    where
        I: IntoIterator<Item = Result<ValueType, E>>,
    {
        use std::hash::{BuildHasher as _, BuildHasherDefault, Hash as _, Hasher as _};
        use twox_hash::XxHash64;

        let mut hasher = BuildHasherDefault::<XxHash64>::default().build_hasher();
        Self::MARKER_FIXED.hash(&mut hasher);
        hash.hash(&mut hasher);

        for arg in args {
            Self::MARKER_ARG.hash(&mut hasher);
            arg?.hash(&mut hasher);
        }

        Ok(Self(hasher.finish()))
    }
}

impl fmt::Display for FnHash {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "0x{:x}", self.0)
    }
}

impl fmt::Debug for FnHash {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "FnHash(0x{:x})", self.0)
    }
}
