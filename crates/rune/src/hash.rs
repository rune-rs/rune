use crate::compile::{IntoComponent, Item};
use crate::runtime::Protocol;
use crate::Any;
use serde::{Deserialize, Serialize};
use std::any;
use std::fmt;
use std::hash::{self, BuildHasher, BuildHasherDefault, Hash as _, Hasher};
use std::mem;
use twox_hash::XxHash64;

const SEP: u64 = 0x4bc94d6bd06053ad;
const PARAMS: u64 = 0x19893cc8f39b1371;
const TYPE: u64 = 0x2fac10b63a6cc57c;
const INSTANCE_FUNCTION_HASH: u64 = 0x5ea77ffbcdf5f302;
const FIELD_FUNCTION_HASH: u64 = 0xab53b6a7a53c757e;
const OBJECT_KEYS: u64 = 0x4473d7017aef7645;

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
    pub(crate) fn of<T: hash::Hash>(thing: T) -> Self {
        let mut hasher = Self::new_hasher();
        thing.hash(&mut hasher);
        Self(hasher.finish())
    }

    /// Get the hash of a type.
    pub fn type_hash<I>(path: I) -> Self
    where
        I: IntoTypeHash,
    {
        path.into_type_hash()
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
        N: InstFnName,
    {
        let name = name.name_hash();
        Self(INSTANCE_FUNCTION_HASH ^ (type_hash.0 ^ name.0))
    }

    /// Construct a hash corresponding to a field function.
    #[inline]
    pub fn field_fn<N>(protocol: Protocol, type_hash: Hash, name: N) -> Self
    where
        N: InstFnName,
    {
        let name = name.name_hash();
        Self(FIELD_FUNCTION_HASH ^ ((type_hash.0 ^ protocol.hash.0) ^ name.0))
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
        I::Item: IntoTypeHash,
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

/// Helper conversion into a function hash.
pub trait IntoTypeHash: Copy {
    /// Generate a function hash.
    fn into_type_hash(self) -> Hash;

    /// Optionally convert into an item, if appropriate.
    fn into_item(self) -> Option<Item>;

    /// Hash the current value.
    fn hash<H>(self, hasher: &mut H)
    where
        H: Hasher;
}

impl IntoTypeHash for Hash {
    fn into_type_hash(self) -> Hash {
        self
    }

    fn into_item(self) -> Option<Item> {
        None
    }

    fn hash<H>(self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.0.hash(hasher);
    }
}

impl<I> IntoTypeHash for I
where
    I: Copy + IntoIterator,
    I::Item: IntoComponent,
{
    fn into_type_hash(self) -> Hash {
        let mut hasher = Hash::new_hasher();
        self.hash(&mut hasher);
        Hash(hasher.finish())
    }

    fn into_item(self) -> Option<Item> {
        Some(Item::with_item(self))
    }

    fn hash<H>(self, hasher: &mut H)
    where
        H: Hasher,
    {
        TYPE.hash(hasher);

        for c in self {
            c.hash_component(hasher);
        }
    }
}

/// An instance function name.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum InstFnKind {
    /// The instance function refers to the given protocol.
    Protocol(Protocol),
    /// The instance function refers to the given named instance fn.
    Instance(Box<str>),
    /// Instance function only has a hash.
    Hash(Hash),
}

impl fmt::Display for InstFnKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstFnKind::Protocol(protocol) => write!(f, "<{}>", protocol.name),
            InstFnKind::Instance(name) => write!(f, "{}", name),
            InstFnKind::Hash(hash) => write!(f, "<{}>", hash),
        }
    }
}

/// A descriptor for an instance function.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct InstFnInfo {
    /// The hash of the instance function.
    pub hash: Hash,
    /// The name of the instance function.
    pub kind: InstFnKind,
    /// Parameters hash.
    pub parameters: Hash,
}

/// Trait used to determine what can be used as an instance function name.
pub trait InstFnName: Copy {
    /// Get only the hash of the named instance function.
    fn name_hash(self) -> Hash;

    /// Get information on the naming of the instance function.
    fn info(self) -> InstFnInfo;
}

impl InstFnName for &str {
    #[inline]
    fn name_hash(self) -> Hash {
        Hash::of(self)
    }

    #[inline]
    fn info(self) -> InstFnInfo {
        InstFnInfo {
            hash: self.name_hash(),
            kind: InstFnKind::Instance(self.into()),
            parameters: Hash::EMPTY,
        }
    }
}

impl InstFnName for Hash {
    #[inline]
    fn name_hash(self) -> Hash {
        self
    }

    #[inline]
    fn info(self) -> InstFnInfo {
        InstFnInfo {
            hash: self,
            kind: InstFnKind::Hash(self),
            parameters: Hash::EMPTY,
        }
    }
}

/// Helper to register a parameterized function.
///
/// This is used to wrap the name of the function in order to associated
/// parameters with it.
#[derive(Clone, Copy)]
pub struct Params<T, P>(pub T, pub P);

impl<T, P> InstFnName for Params<T, P>
where
    T: InstFnName,
    P: Copy + IntoIterator,
    P::Item: IntoTypeHash,
{
    fn name_hash(self) -> Hash {
        self.0.name_hash()
    }

    fn info(self) -> InstFnInfo {
        let info = self.0.info();

        InstFnInfo {
            hash: info.hash,
            kind: info.kind,
            parameters: Hash::parameters(self.1),
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

    pub(crate) fn add(&mut self, p: impl IntoTypeHash) {
        SEP.hash(&mut self.hasher);
        p.hash(&mut self.hasher);
    }

    pub(crate) fn finish(&self) -> Hash {
        Hash(self.hasher.finish())
    }
}
