//! A container of bytes, corresponding to the [Value::Bytes] type.
//!
//! [Value::Bytes]: crate::Value::Bytes.

use std::fmt;
use std::ops;

/// A vector of bytes.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bytes {
    pub(crate) bytes: Vec<u8>,
}

impl Bytes {
    /// Construct a new bytes container.
    pub fn new() -> Self {
        Bytes { bytes: Vec::new() }
    }

    /// Construct a new bytes container with the specified capacity.
    pub fn with_capacity(cap: usize) -> Self {
        Bytes {
            bytes: Vec::with_capacity(cap),
        }
    }

    /// Convert into vector.
    pub fn into_vec(self) -> Vec<u8> {
        self.bytes
    }

    /// Construct from a byte vector.
    pub fn from_vec(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    /// Do something with the bytes.
    pub fn extend(&mut self, other: &Self) {
        self.bytes.extend(other.bytes.iter().copied());
    }

    /// Do something with the bytes.
    pub fn extend_str(&mut self, s: &str) {
        self.bytes.extend(s.as_bytes());
    }

    /// Get the length of the bytes collection.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Get the capacity of the bytes collection.
    pub fn capacity(&self) -> usize {
        self.bytes.capacity()
    }

    /// Get the bytes collection.
    pub fn clear(&mut self) {
        self.bytes.clear();
    }

    /// Reserve additional space.
    ///
    /// The exact amount is unspecified.
    pub fn reserve(&mut self, additional: usize) {
        self.bytes.reserve(additional);
    }

    /// Resever additional space to the exact amount specified.
    pub fn reserve_exact(&mut self, additional: usize) {
        self.bytes.reserve_exact(additional);
    }

    /// Shrink to fit the amount of bytes in the container.
    pub fn shrink_to_fit(&mut self) {
        self.bytes.shrink_to_fit();
    }

    /// Pop the last byte.
    pub fn pop(&mut self) -> Option<u8> {
        self.bytes.pop()
    }

    /// Access the last byte.
    pub fn last(&mut self) -> Option<u8> {
        self.bytes.last().copied()
    }
}

impl fmt::Debug for Bytes {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_list().entries(&self.bytes).finish()
    }
}

impl ops::Deref for Bytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

impl ops::DerefMut for Bytes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.bytes
    }
}
