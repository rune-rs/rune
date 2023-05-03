mod into_hash;
mod to_type_hash;

use core::any;
use core::fmt;
use core::hash::{self, BuildHasher, BuildHasherDefault, Hash as _, Hasher};
use core::mem;

use serde::{Deserialize, Serialize};
use twox_hash::XxHash64;

pub use self::into_hash::IntoHash;
pub use self::to_type_hash::ToTypeHash;
use crate::runtime::Protocol;
use crate::Any;

const SEP: u64 = 0x4bc94d6bd06053ad;
const PARAMS: u64 = 0x19893cc8f39b1371;
const TYPE: u64 = 0x2fac10b63a6cc57c;
const INSTANCE_FUNCTION_HASH: u64 = 0x5ea77ffbcdf5f302;
const FIELD_FUNCTION_HASH: u64 = 0xab53b6a7a53c757e;
const OBJECT_KEYS: u64 = 0x4473d7017aef7645;
const INDEX_FUNCTION_HASH: u64 = 0x2579e52d1534901b;
const INDEX: u64 = 0xe1b2378d7a937035;

/// The primitive hash that among other things is used to reference items,
/// types, and native functions.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Hash(u64);

impl Hash {
    /// The empty hash.
    pub const EMPTY: Self = Self(0);

    /// Construct a new raw hash.
    pub(crate) const fn new(hash: u64) -> Self {
        Self(hash)
    }

    /// Construct a simple hash from something that is hashable.
    fn of<T: hash::Hash>(thing: T) -> Self {
        let mut hasher = Self::new_hasher();
        thing.hash(&mut hasher);
        Self(hasher.finish())
    }

    /// Construct a hash from an index.
    #[inline]
    pub fn index(index: usize) -> Self {
        Self(INDEX ^ (index as u64))
    }

    /// Get the hash of a type.
    pub fn type_hash<I>(path: I) -> Self
    where
        I: ToTypeHash,
    {
        path.to_type_hash()
    }

    /// Construct a hash from the given type id.
    pub fn from_any<T>() -> Self
    where
        T: Any,
    {
        Self::from_type_id(any::TypeId::of::<T>())
    }

    /// Construct a hash from a type id.
    pub const fn from_type_id(type_id: any::TypeId) -> Self {
        // Safety: a type id is exactly a 64-bit unsigned integer.
        // And has an identical bit pattern to `Hash`.
        unsafe { mem::transmute(type_id) }
    }

    /// Construct a hash to an instance function, where the instance is a
    /// pre-determined type.
    #[inline]
    pub fn instance_function<N>(type_hash: Hash, name: N) -> Self
    where
        N: IntoHash,
    {
        let name = name.into_hash();
        Self(INSTANCE_FUNCTION_HASH ^ (type_hash.0 ^ name.0))
    }

    /// Construct a hash corresponding to a field function.
    #[inline]
    pub fn field_fn<N>(protocol: Protocol, type_hash: Hash, name: N) -> Self
    where
        N: IntoHash,
    {
        Self(FIELD_FUNCTION_HASH ^ ((type_hash.0 ^ protocol.hash.0) ^ name.into_hash().0))
    }

    /// Construct an index function.
    #[inline]
    pub fn index_fn(protocol: Protocol, type_hash: Hash, index: Hash) -> Self {
        Self(INDEX_FUNCTION_HASH ^ ((type_hash.0 ^ protocol.hash.0) ^ index.0))
    }

    /// Get the hash corresponding to a static byte array.
    pub fn static_bytes(bytes: &[u8]) -> Hash {
        Self::of(bytes)
    }

    /// Get the hash corresponding to a instance function name.
    pub fn instance_fn_name(name: &str) -> Hash {
        Self::of(name)
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

    /// Mix the current hash in the correct manner with another parameters hash.
    pub(crate) fn with_parameters(self, parameters: Self) -> Self {
        Self(self.0 ^ parameters.0)
    }

    /// Hash type parameters.
    pub(crate) fn parameters<I>(parameters: I) -> Self
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

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:x}", self.0)
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return f.debug_tuple("Hash").field(&Hex(self.0)).finish();

        #[repr(transparent)]
        struct Hex(u64);

        impl fmt::Debug for Hex {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "0x{:x}", self.0)
            }
        }
    }
}

/// Helper to build a parameters hash.
pub(crate) struct ParametersBuilder {
    hasher: XxHash64,
}

impl ParametersBuilder {
    pub(crate) fn new() -> Self {
        let mut hasher = Hash::new_hasher();
        PARAMS.hash(&mut hasher);
        Self { hasher }
    }

    pub(crate) fn add<P>(&mut self, p: P)
    where
        P: hash::Hash,
    {
        SEP.hash(&mut self.hasher);
        p.hash(&mut self.hasher);
    }

    pub(crate) fn finish(&self) -> Hash {
        Hash::new(self.hasher.finish())
    }
}
