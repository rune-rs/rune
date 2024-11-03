use core::hash::{BuildHasher, Hasher as _};

use rune_alloc::hash_map;

use crate as rune;
use crate::Any;

/// The default hasher used in Rune.
#[derive(Any)]
#[rune(item = ::std::hash)]
pub struct Hasher {
    hasher: hash_map::Hasher,
}

impl Hasher {
    /// Construct a new empty hasher.
    pub(crate) fn new_with<S>(build_hasher: &S) -> Self
    where
        S: BuildHasher<Hasher = hash_map::Hasher>,
    {
        Self {
            hasher: build_hasher.build_hasher(),
        }
    }

    /// Hash a string.
    pub(crate) fn write_str(&mut self, string: &str) {
        self.hasher.write(string.as_bytes());
    }

    /// Construct a hash.
    pub fn finish(&self) -> u64 {
        self.hasher.finish()
    }
}

impl core::hash::Hasher for Hasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.hasher.finish()
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        self.hasher.write(bytes);
    }
}
