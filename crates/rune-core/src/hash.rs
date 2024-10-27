pub use self::into_hash::IntoHash;
mod into_hash;

pub use self::to_type_hash::ToTypeHash;
mod to_type_hash;

use core::fmt;
use core::hash::{BuildHasher, BuildHasherDefault, Hash as _, Hasher};

#[cfg(feature = "musli")]
use musli::{Decode, Encode};
use serde::{Deserialize, Serialize};
use twox_hash::XxHash64;

#[derive(Debug)]
#[non_exhaustive]
pub struct TooManyParameters;

impl fmt::Display for TooManyParameters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Only 32 type parameters are supported")
    }
}

impl core::error::Error for TooManyParameters {}

use crate::protocol::Protocol;

use crate::alloc;
use crate::alloc::clone::TryClone;

const SEP: u64 = 0x4bc94d6bd06053ad;
const TYPE: u64 = 0x2fac10b63a6cc57c;
const ASSOCIATED_FUNCTION_HASH: u64 = 0x5ea77ffbcdf5f302;
const OBJECT_KEYS: u64 = 0x4473d7017aef7645;
const IDENT: u64 = 0x1a095090689d4647;
const INDEX: u64 = 0xe1b2378d7a937035;
// Salt for type parameters.
const TYPE_PARAMETERS: u32 = 16;
// Salt for function parameters.
const FUNCTION_PARAMETERS: u32 = 48;
// A bunch of random hashes to mix in when calculating type parameters.
const PARAMETERS: [u64; 32] = [
    0x2d859a05306ebe33,
    0x75ac929aabdda742,
    0x18f4e51cd6f60e86,
    0x3b47f977015b002,
    0xd7e3b9e36d59b900,
    0xad75a1d63593d47c,
    0x8fc37a65ac89ed71,
    0x39eb9ab133d1cf22,
    0xa287885efb6bf688,
    0x9eeef0c7395ea8ca,
    0x26a193328114c317,
    0x9edc35591d044a28,
    0xbfa4e9a8eca88b80,
    0x94a626c6aa89a686,
    0x95970296235c5b91,
    0xa8ab16ceff9068b8,
    0x153e675e2a27db85,
    0xa873a7e51dfe4205,
    0xde401d82396a7876,
    0x9dbbae67606eddc3,
    0x23d51f8018d09e74,
    0xb5bfa5d588fedcc6,
    0x9702480ba16eeb96,
    0x58549fb39441505c,
    0xd88078065e88667d,
    0x38a1d4efb147fe18,
    0xf712c95e9ffd1ba5,
    0x73c2249b2845a5e0,
    0x8079aff8b0833e20,
    0x7e708fb5e906bbb5,
    0x22d363b1d55a5eec,
    0x9e2d56cbbd4459f1,
];

/// The primitive hash that among other things is used to reference items,
/// types, and native functions.
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "musli", derive(Decode, Encode))]
#[repr(transparent)]
#[cfg_attr(feature = "musli", musli(transparent))]
pub struct Hash(#[doc(hidden)] pub u64);

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

    /// Return the current hash if it is non-empty.
    #[inline]
    pub fn as_non_empty(&self) -> Option<Self> {
        if self.is_empty() {
            None
        } else {
            Some(*self)
        }
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
    pub fn type_hash(path: impl ToTypeHash) -> Self {
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
        Self(self.0 ^ ty.0.wrapping_shl(TYPE_PARAMETERS))
    }

    /// Mix the current hash with function parameters.
    pub const fn with_function_parameters(self, f: Self) -> Self {
        Self(self.0 ^ f.0.wrapping_shl(FUNCTION_PARAMETERS))
    }

    /// Hash type parameters.
    pub const fn parameters<const N: usize>(params: [Hash; N]) -> Self {
        let mut builder = ParametersBuilder::new();

        while builder.index < N {
            let param = params[builder.index];

            let Ok(b) = builder.add(param) else {
                panic!("Only up to 32 type parameters are supported");
            };

            builder = b;
        }

        builder.finish()
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
///
/// A collection of parameters are like the type parameters like `String` and
/// `i64` in a signature like:
///
/// `::my_crate::Map<String, i64>`
///
/// # Examples
///
/// ```
/// use rune::TypeHash;
/// use rune::hash::ParametersBuilder;
///
/// let mut params = ParametersBuilder::new();
///
/// let params = params.add(String::HASH)?;
/// let params = params.add(i64::HASH)?;
///
/// let hash = params.finish();
/// # Ok::<_, rune::hash::TooManyParameters>(())
/// ```
#[derive(Default)]
pub struct ParametersBuilder {
    base: u64,
    index: usize,
    shift: u32,
}

impl ParametersBuilder {
    /// Construct a new collection of parameters.
    pub const fn new() -> Self {
        Self {
            base: 0,
            index: 0,
            shift: 0,
        }
    }

    /// Add a hash to the collection of parameters.
    ///
    /// # Errors
    ///
    /// Errors if too many parameters are added.
    pub const fn add(mut self, Hash(hash): Hash) -> Result<Self, TooManyParameters> {
        if self.index >= PARAMETERS.len() {
            self.shift += 8;
            self.index = 0;

            if self.shift >= u64::BITS {
                return Err(TooManyParameters);
            }
        }

        self.base ^= hash ^ PARAMETERS[self.index].wrapping_shl(self.shift);
        self.index += 1;
        Ok(self)
    }

    /// Finish building the parameters hash.
    pub const fn finish(self) -> Hash {
        Hash::new(self.base)
    }
}
