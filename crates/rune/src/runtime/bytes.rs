//! A container of bytes, corresponding to the [Value::Bytes] type.
//!
//! [Value::Bytes]: crate::Value::Bytes.

use core::cmp;
use core::fmt;
use core::ops;

use serde::de;
use serde::ser;

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, Box, Vec};
use crate::runtime::{RawRef, Ref, UnsafeToRef, Value, VmResult};
use crate::Any;

/// A vector of bytes.
#[derive(Any, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[rune(builtin, static_type = BYTES_TYPE)]
pub struct Bytes {
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
    /// let mut bytes = Bytes::with_capacity(32)?;
    /// assert_eq!(bytes, b"");
    /// bytes.extend(b"abcd")?;
    /// assert_eq!(bytes, b"abcd");
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn with_capacity(cap: usize) -> alloc::Result<Self> {
        Ok(Self {
            bytes: Vec::try_with_capacity(cap)?,
        })
    }

    /// Convert the byte array into a vector of bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Bytes;
    /// use rune::alloc::prelude::*;
    /// use rune::alloc::try_vec;
    ///
    /// let bytes = Bytes::from_vec(try_vec![b'a', b'b', b'c', b'd']);
    /// assert_eq!(bytes.into_vec(), [b'a', b'b', b'c', b'd']);
    ///
    /// Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn into_vec(self) -> Vec<u8> {
        self.bytes
    }

    /// Access bytes as a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Bytes;
    /// use rune::alloc::try_vec;
    ///
    /// let bytes = Bytes::from_vec(try_vec![b'a', b'b', b'c', b'd']);
    /// assert_eq!(bytes.as_slice(), &[b'a', b'b', b'c', b'd']);
    ///
    /// Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes
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
    /// let bytes = Bytes::from_slice(vec![b'a', b'b', b'c', b'd'])?;
    /// assert_eq!(bytes, b"abcd");
    ///
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn from_slice<B>(bytes: B) -> alloc::Result<Self>
    where
        B: AsRef<[u8]>,
    {
        Ok(Self {
            bytes: Vec::try_from(bytes.as_ref())?,
        })
    }

    /// Convert a byte array into bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Bytes;
    /// use rune::alloc::try_vec;
    ///
    /// let bytes = Bytes::from_vec(try_vec![b'a', b'b', b'c', b'd']);
    /// assert_eq!(bytes, b"abcd");
    /// # Ok::<_, rune::support::Error>(())
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
    /// use rune::alloc::try_vec;
    ///
    /// let mut bytes = Bytes::from_vec(try_vec![b'a', b'b', b'c', b'd']);
    /// bytes.extend(b"efgh");
    /// assert_eq!(bytes, b"abcdefgh");
    ///
    /// Ok::<_, rune::support::Error>(())
    /// ```
    pub fn extend<O>(&mut self, other: O) -> alloc::Result<()>
    where
        O: AsRef<[u8]>,
    {
        self.bytes.try_extend_from_slice(other.as_ref())
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
    pub fn reserve(&mut self, additional: usize) -> alloc::Result<()> {
        self.bytes.try_reserve(additional)
    }

    /// Resever additional space to the exact amount specified.
    pub fn reserve_exact(&mut self, additional: usize) -> alloc::Result<()> {
        self.bytes.try_reserve_exact(additional)
    }

    /// Shrink to fit the amount of bytes in the container.
    pub fn shrink_to_fit(&mut self) -> alloc::Result<()> {
        self.bytes.try_shrink_to_fit()
    }

    /// Pop the last byte.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Bytes;
    ///
    /// let mut bytes = Bytes::from_slice(b"abcd")?;
    /// assert_eq!(bytes.pop(), Some(b'd'));
    /// assert_eq!(bytes, b"abc");
    /// Ok::<_, rune::support::Error>(())
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
    /// let bytes = Bytes::from_slice(b"abcd")?;
    /// assert_eq!(bytes.first(), Some(b'a'));
    ///
    /// Ok::<_, rune::support::Error>(())
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
    /// let bytes = Bytes::from_slice(b"abcd")?;
    /// assert_eq!(bytes.last(), Some(b'd'));
    ///
    /// Ok::<_, rune::support::Error>(())
    /// ```
    pub fn last(&self) -> Option<u8> {
        self.bytes.last().copied()
    }
}

impl TryClone for Bytes {
    fn try_clone(&self) -> alloc::Result<Self> {
        Ok(Self {
            bytes: self.bytes.try_clone()?,
        })
    }
}

impl From<Vec<u8>> for Bytes {
    #[inline]
    fn from(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }
}

#[cfg(feature = "alloc")]
impl TryFrom<::rust_alloc::vec::Vec<u8>> for Bytes {
    type Error = alloc::Error;

    #[inline]
    fn try_from(bytes: ::rust_alloc::vec::Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Self {
            bytes: Vec::try_from(bytes)?,
        })
    }
}

impl From<Box<[u8]>> for Bytes {
    #[inline]
    fn from(bytes: Box<[u8]>) -> Self {
        Self {
            bytes: Vec::from(bytes),
        }
    }
}

#[cfg(feature = "alloc")]
impl TryFrom<::rust_alloc::boxed::Box<[u8]>> for Bytes {
    type Error = alloc::Error;

    #[inline]
    fn try_from(bytes: ::rust_alloc::boxed::Box<[u8]>) -> Result<Self, Self::Error> {
        Ok(Self {
            bytes: Vec::try_from(bytes.as_ref())?,
        })
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
        VmResult::Ok((value.as_ref().as_slice(), guard))
    }
}

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

impl ser::Serialize for Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_bytes(&self.bytes)
    }
}

impl<'de> de::Deserialize<'de> for Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Bytes;

            #[inline]
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a byte array")
            }

            #[inline]
            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Bytes::from_slice(v).map_err(E::custom)
            }
        }

        deserializer.deserialize_bytes(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::{Bytes, Shared, Value};
    use crate::tests::prelude::*;

    #[test]
    #[allow(clippy::let_and_return)]
    fn test_clone_issue() -> Result<(), Box<dyn std::error::Error>> {
        let shared = Value::Bytes(Shared::new(Bytes::new())?);

        let _ = {
            let shared = shared.into_bytes().into_result()?;
            let out = shared.borrow_ref()?.try_clone()?;
            out
        };

        Ok(())
    }
}
