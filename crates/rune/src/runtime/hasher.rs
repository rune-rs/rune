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

    /// Hash some bytes.
    pub(crate) fn write(&mut self, bytes: &[u8]) {
        self.hasher.write(bytes);
    }

    /// Hash a string.
    pub(crate) fn write_str(&mut self, string: &str) {
        self.hasher.write(string.as_bytes());
    }

    /// Hash an 64-bit float.
    ///
    /// You should ensure that the float is normal per the [`f64::is_normal`]
    /// function before hashing it, since otherwise equality tests against the
    /// float won't work as intended. Otherwise, know what you're doing.
    pub(crate) fn write_f64(&mut self, value: f64) {
        let bits = value.to_bits();
        self.hasher.write_u64(bits);
    }

    /// Hash a 64-bit signed integer.
    pub(crate) fn write_i64(&mut self, value: i64) {
        self.hasher.write_i64(value);
    }

    /// Hash an 8-bit unsigned integer.
    pub(crate) fn write_u8(&mut self, value: u8) {
        self.hasher.write_u8(value);
    }

    /// Construct a hash.
    pub fn finish(self) -> u64 {
        self.hasher.finish()
    }
}
