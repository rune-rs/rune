//! A container of bytes, corresponding to the [Value::Bytes] type.
//!
//! [Value::Bytes]: crate::Value::Bytes.

use core::cmp;
use core::fmt;
use core::ops;

use crate::no_std::prelude::*;

use serde::{Deserialize, Serialize};

use crate::compile::Named;
use crate::module::InstallWith;
use crate::runtime::{RawRef, RawStr, Ref, UnsafeToRef, Value, VmResult};

/// A vector of bytes.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Bytes {
    #[serde(with = "serde_bytes")]
    pub(crate) bytes: Vec<u8>,
}

impl Bytes {
    /// Construct a new byte array.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Bytes;
    ///
    /// let bytes = Bytes::new();
    /// assert_eq!(bytes, b"");
    /// ```
    #[inline]
    pub const fn new() -> Self {
        Bytes { bytes: Vec::new() }
    }

    /// Construct a byte array with the given preallocated capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Bytes;
    ///
    /// let mut bytes = Bytes::with_capacity(32);
    /// assert_eq!(bytes, b"");
    /// bytes.extend(b"abcd");
    /// assert_eq!(bytes, b"abcd");
    /// ```
    #[inline]
    pub fn with_capacity(cap: usize) -> Self {
        Bytes {
            bytes: Vec::with_capacity(cap),
        }
    }

    /// Convert the byte array into a vector of bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Bytes;
    ///
    /// let bytes = Bytes::from_vec(vec![b'a', b'b', b'c', b'd']);
    /// assert_eq!(bytes.into_vec(), [b'a', b'b', b'c', b'd']);
    /// ```
    #[inline]
    pub fn into_vec(self) -> Vec<u8> {
        self.bytes
    }

    /// Convert a slice into bytes.
    ///
    /// Calling this function allocates bytes internally.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Bytes;
    ///
    /// let bytes = Bytes::from_slice(vec![b'a', b'b', b'c', b'd']);
    /// assert_eq!(bytes, b"abcd");
    /// ```
    #[inline]
    pub fn from_slice<B>(bytes: B) -> Self
    where
        B: AsRef<[u8]>,
    {
        Self {
            bytes: bytes.as_ref().to_vec(),
        }
    }

    /// Convert a byte array into bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Bytes;
    ///
    /// let bytes = Bytes::from_vec(vec![b'a', b'b', b'c', b'd']);
    /// assert_eq!(bytes, b"abcd");
    /// ```
    #[inline]
    pub fn from_vec(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    /// Extend these bytes with another collection of bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Bytes;
    ///
    /// let mut bytes = Bytes::from_vec(vec![b'a', b'b', b'c', b'd']);
    /// bytes.extend(b"efgh");
    /// assert_eq!(bytes, b"abcdefgh");
    /// ```
    pub fn extend<O>(&mut self, other: O)
    where
        O: AsRef<[u8]>,
    {
        self.bytes.extend_from_slice(other.as_ref());
    }

    /// Test if the collection is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Bytes;
    ///
    /// let mut bytes = Bytes::new();
    /// assert!(bytes.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Get the length of the bytes collection.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Bytes;
    ///
    /// let mut bytes = Bytes::new();
    /// assert_eq!(bytes.len(), 0);
    /// bytes.extend(b"abcd");
    /// assert_eq!(bytes.len(), 4);
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Bytes;
    ///
    /// let mut bytes = Bytes::from_slice(b"abcd");
    /// assert_eq!(bytes.pop(), Some(b'd'));
    /// assert_eq!(bytes, b"abc");
    /// ```
    pub fn pop(&mut self) -> Option<u8> {
        self.bytes.pop()
    }

    /// Get the first byte.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Bytes;
    ///
    /// let bytes = Bytes::from_slice(b"abcd");
    /// assert_eq!(bytes.first(), Some(b'a'));
    /// ```
    pub fn first(&self) -> Option<u8> {
        self.bytes.first().copied()
    }

    /// Get the last byte.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Bytes;
    ///
    /// let bytes = Bytes::from_slice(b"abcd");
    /// assert_eq!(bytes.last(), Some(b'd'));
    /// ```
    pub fn last(&self) -> Option<u8> {
        self.bytes.last().copied()
    }
}

impl From<Vec<u8>> for Bytes {
    fn from(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}

impl fmt::Debug for Bytes {
    #[inline]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_list().entries(&self.bytes).finish()
    }
}

impl ops::Deref for Bytes {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

impl ops::DerefMut for Bytes {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.bytes
    }
}

impl AsRef<[u8]> for Bytes {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

from_value!(Bytes, into_bytes);

impl UnsafeToRef for [u8] {
    type Guard = RawRef;

    unsafe fn unsafe_to_ref<'a>(value: Value) -> VmResult<(&'a Self, Self::Guard)> {
        let bytes = vm_try!(value.into_bytes());
        let bytes = vm_try!(bytes.into_ref());
        let (value, guard) = Ref::into_raw(bytes);
        // Safety: we're holding onto the guard for the slice here, so it is
        // live.
        VmResult::Ok(((*value).bytes.as_slice(), guard))
    }
}

impl Named for Bytes {
    const BASE_NAME: RawStr = RawStr::from_str("Bytes");
}

impl InstallWith for Bytes {}

impl<const N: usize> cmp::PartialEq<[u8; N]> for Bytes {
    #[inline]
    fn eq(&self, other: &[u8; N]) -> bool {
        self.bytes == other[..]
    }
}

impl<const N: usize> cmp::PartialEq<&[u8; N]> for Bytes {
    #[inline]
    fn eq(&self, other: &&[u8; N]) -> bool {
        self.bytes == other[..]
    }
}

impl<const N: usize> cmp::PartialEq<Bytes> for [u8; N] {
    #[inline]
    fn eq(&self, other: &Bytes) -> bool {
        self[..] == other.bytes
    }
}

impl<const N: usize> cmp::PartialEq<Bytes> for &[u8; N] {
    #[inline]
    fn eq(&self, other: &Bytes) -> bool {
        self[..] == other.bytes
    }
}

impl cmp::PartialEq<[u8]> for Bytes {
    #[inline]
    fn eq(&self, other: &[u8]) -> bool {
        self.bytes == other
    }
}

impl cmp::PartialEq<Bytes> for [u8] {
    #[inline]
    fn eq(&self, other: &Bytes) -> bool {
        self == other.bytes
    }
}

#[cfg(test)]
mod tests {
    use crate::no_std::prelude::*;
    use crate::runtime::{Bytes, Shared, Value};

    #[test]
    #[allow(clippy::let_and_return)]
    fn test_clone_issue() -> Result<(), Box<dyn std::error::Error>> {
        let shared = Value::Bytes(Shared::new(Bytes::new()));

        let _ = {
            let shared = shared.into_bytes().into_result()?;
            let out = shared.borrow_ref()?.clone();
            out
        };

        Ok(())
    }
}
