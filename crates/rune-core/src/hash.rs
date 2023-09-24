pub use self::into_hash::IntoHash;
mod into_hash;

pub use self::to_type_hash::ToTypeHash;
mod to_type_hash;

use core::fmt;
use core::hash::{self, BuildHasher, BuildHasherDefault, Hash as _, Hasher};

#[cfg(feature = "musli")]
use musli::{Decode, Encode};
use serde::{Deserialize, Serialize};
use twox_hash::XxHash64;

use crate::protocol::Protocol;

use crate::alloc;
use crate::alloc::clone::TryClone;

const SEP: u64 = 0x4bc94d6bd06053ad;
const PARAMS: u64 = 0x19893cc8f39b1371;
const TYPE: u64 = 0x2fac10b63a6cc57c;
const ASSOCIATED_FUNCTION_HASH: u64 = 0x5ea77ffbcdf5f302;
const OBJECT_KEYS: u64 = 0x4473d7017aef7645;
const IDENT: u64 = 0x1a095090689d4647;
const INDEX: u64 = 0xe1b2378d7a937035;

// Salt for type parameters.
const TYPE_PARAMETERS: u64 = 0x9d30e58b77e4599;
// Salt for function parameters.
const FUNCTION_PARAMETERS: u64 = 0x6052c152243a6eb3;

/// The primitive hash that among other things is used to reference items,
/// types, and native functions.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "musli", derive(Decode, Encode))]
#[repr(transparent)]
#[cfg_attr(feature = "musli", musli(transparent))]
pub struct Hash(u64);

impl Hash {
    /// The empty hash.
    pub const EMPTY: Self = Self(0);

    /// Construct a new raw hash.
    #[doc(hidden)]
    pub const fn new(hash: u64) -> Self {
        Self(hash)
    }

    /// Construct a new raw hash with the given parameters.
    #[doc(hidden)]
    pub const fn new_with_type_parameters(hash: u64, parameters: Hash) -> Self {
        Self(hash).with_type_parameters(parameters)
    }

    /// Coerce a hash into its inner numerical value.
    #[doc(hidden)]
    pub const fn into_inner(self) -> u64 {
        self.0
    }

    /// Test if hash is empty.
    #[doc(hidden)]
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Construct a hash from an index.
    #[inline]
    pub fn index(index: usize) -> Self {
        Self(INDEX ^ (index as u64))
    }

    /// Get the hash corresponding to a string identifier like `function` or
    /// `hello_world`.
    pub fn ident(name: &str) -> Hash {
        let mut hasher = Self::new_hasher();
        name.hash(&mut hasher);
        Self(IDENT ^ hasher.finish())
    }

    /// Get the hash of a type.
    pub fn type_hash<I>(path: I) -> Self
    where
        I: ToTypeHash,
    {
        path.to_type_hash()
    }

    /// Construct a hash to an instance function, where the instance is a
    /// pre-determined type.
    #[inline]
    pub fn associated_function<N>(type_hash: Hash, name: N) -> Self
    where
        N: IntoHash,
    {
        let name = name.into_hash();
        Self(ASSOCIATED_FUNCTION_HASH ^ (type_hash.0 ^ name.0))
    }

    /// Construct a hash corresponding to a field function.
    #[inline]
    pub fn field_function<N>(protocol: Protocol, type_hash: Hash, name: N) -> Self
    where
        N: IntoHash,
    {
        Self::associated_function(Hash(type_hash.0 ^ protocol.0), name)
    }

    /// Construct an index function.
    #[inline]
    pub fn index_function(protocol: Protocol, type_hash: Hash, index: Hash) -> Self {
        Self::associated_function(Hash(type_hash.0 ^ protocol.0), index)
    }

    /// Get the hash corresponding to a static byte array.
    pub fn static_bytes(bytes: &[u8]) -> Hash {
        let mut hasher = Self::new_hasher();
        bytes.hash(&mut hasher);
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

    /// Mix in generics hash.
    ///
    /// The generics hash must be a combination of the output from
    /// `with_type_parameters` and `with_function_parameters`.
    pub const fn with_generics(self, generics: Self) -> Self {
        Self(self.0 ^ generics.0)
    }

    /// Mix the current hash with type parameters.
    pub const fn with_type_parameters(self, ty: Self) -> Self {
        if !ty.is_empty() {
            Self(self.0 ^ (ty.0 ^ TYPE_PARAMETERS))
        } else {
            self
        }
    }

    /// Mix the current hash with function parameters.
    pub const fn with_function_parameters(self, f: Self) -> Self {
        if !f.is_empty() {
            Self(self.0 ^ (f.0 ^ FUNCTION_PARAMETERS))
        } else {
            self
        }
    }

    /// Hash type parameters.
    pub fn parameters<I>(parameters: I) -> Self
    where
        I: IntoIterator,
        I::Item: hash::Hash,
    {
        let mut hasher = ParametersBuilder::new();

        for p in parameters {
            hasher.add(p);
        }

        hasher.finish()
    }

    /// Construct a new hasher.
    fn new_hasher() -> XxHash64 {
        BuildHasherDefault::<XxHash64>::default().build_hasher()
    }
}

impl TryClone for Hash {
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(*self)
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:x}", self.0)
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:x}", self.0)
    }
}

/// Helper to build a parameters hash.
#[doc(hidden)]
pub struct ParametersBuilder {
    hasher: XxHash64,
}

impl ParametersBuilder {
    #[doc(hidden)]
    pub fn new() -> Self {
        let mut hasher = Hash::new_hasher();
        PARAMS.hash(&mut hasher);
        Self { hasher }
    }

    #[doc(hidden)]
    pub fn add<P>(&mut self, p: P)
    where
        P: hash::Hash,
    {
        SEP.hash(&mut self.hasher);
        p.hash(&mut self.hasher);
    }

    #[doc(hidden)]
    pub fn finish(&self) -> Hash {
        Hash::new(self.hasher.finish())
    }
}
