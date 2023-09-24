use crate::alloc::HashMap;

use core::hash::{BuildHasher, Hasher};

const SEED: u64 = 18446744073709551557u64;

#[doc(inline)]
pub use rune_core::{Hash, ToTypeHash};
#[doc(inline)]
pub(crate) use rune_core::{IntoHash, ParametersBuilder};

/// A hash map suitable for storing values with hash keys.
pub(crate) type Map<T> = HashMap<Hash, T, HashBuildHasher>;

#[derive(Default, Clone, Copy)]
pub(crate) struct HashBuildHasher;

impl BuildHasher for HashBuildHasher {
    type Hasher = HashHasher;

    #[inline]
    fn build_hasher(&self) -> Self::Hasher {
        HashHasher(SEED)
    }
}

pub(crate) struct HashHasher(u64);

impl Hasher for HashHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }

    #[inline]
    fn write(&mut self, _: &[u8]) {
        panic!("Hash hashers assume that 64-bit hashes are already random")
    }

    #[inline]
    fn write_u64(&mut self, hash: u64) {
        self.0 ^= hash;
    }
}
