//! A container of bytes, corresponding to the [Value::Bytes] type.
//!
//! [Value::Bytes]: crate::Value::Bytes.

use core::fmt;
use core::ops;

use serde::de;
use serde::ser;

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{self, Box, Vec};
use crate::runtime::VmResult;
use crate::TypeHash as _;
use crate::{Any, FromValue};

use super::{
    IntoOutput, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive,
    RawAnyGuard, Ref, RuntimeError, UnsafeToRef, Value, VmErrorKind,
};

/// A vector of bytes.
#[derive(Any, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[rune(item = ::std::bytes)]
pub struct Bytes {
    bytes: Vec<u8>,
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

    /// Append a byte to the back.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Bytes;
    ///
    /// let mut bytes = Bytes::from_slice(b"abcd")?;
    /// bytes.push(b'e');
    /// assert_eq!(bytes, b"abcde");
    /// Ok::<_, rune::support::Error>(())
    /// ```
    pub fn push(&mut self, value: u8) -> alloc::Result<()> {
        self.bytes.try_push(value)
    }

    /// Removes the byte at the specified index.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Bytes;
    ///
    /// let mut bytes = Bytes::from_slice(b"abcd")?;
    /// bytes.remove(2);
    /// assert_eq!(bytes, b"abd");
    /// Ok::<_, rune::support::Error>(())
    /// ```
    pub fn remove(&mut self, index: usize) -> u8 {
        self.bytes.remove(index)
    }

    /// Inserts a byte at position index within the vector, shifting all
    /// elements after it to the right.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Bytes;
    ///
    /// let mut bytes = Bytes::from_slice(b"abcd")?;
    /// bytes.insert(2, b'e');
    /// assert_eq!(bytes, b"abecd");
    /// Ok::<_, rune::support::Error>(())
    /// ```
    pub fn insert(&mut self, index: usize, value: u8) -> alloc::Result<()> {
        self.bytes.try_insert(index, value)
    }

    /// Returns a subslice of Bytes.
    ///
    /// -  If given a position, returns the byte at that position or `None` if
    ///    out of bounds.
    /// - If given a range, returns the subslice corresponding to that range, or
    ///   `None` if out of bounds.
    pub(crate) fn index_get(&self, index: Value) -> VmResult<Option<Value>> {
        bytes_slice_index_get(&self.bytes, index)
    }

    /// Set by index
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::runtime::Bytes;
    ///
    /// let mut bytes = Bytes::from_slice(b"abcd")?;
    /// bytes.set(0, b'A');
    /// assert_eq!(bytes, b"Abcd");
    /// Ok::<_, rune::support::Error>(())
    /// ```
    pub fn set(&mut self, index: usize, value: u8) -> VmResult<()> {
        let Some(v) = self.bytes.get_mut(index) else {
            return VmResult::err(VmErrorKind::OutOfRange {
                index: index.into(),
                length: self.len().into(),
            });
        };

        *v = value;
        VmResult::Ok(())
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
    #[inline]
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
impl TryFrom<&[u8]> for Bytes {
    type Error = alloc::Error;

    #[inline]
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut bytes = Vec::try_with_capacity(value.len())?;
        bytes.try_extend_from_slice(value)?;
        Ok(Self { bytes })
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

impl UnsafeToRef for [u8] {
    type Guard = RawAnyGuard;

    #[inline]
    unsafe fn unsafe_to_ref<'a>(value: Value) -> Result<(&'a Self, Self::Guard), RuntimeError> {
        let (value, guard) = Ref::into_raw(value.into_ref::<Bytes>()?);
        Ok((value.as_ref().as_slice(), guard))
    }
}

impl<const N: usize> PartialEq<[u8; N]> for Bytes {
    #[inline]
    fn eq(&self, other: &[u8; N]) -> bool {
        self.bytes == other[..]
    }
}

impl<const N: usize> PartialEq<&[u8; N]> for Bytes {
    #[inline]
    fn eq(&self, other: &&[u8; N]) -> bool {
        self.bytes == other[..]
    }
}

impl<const N: usize> PartialEq<Bytes> for [u8; N] {
    #[inline]
    fn eq(&self, other: &Bytes) -> bool {
        self[..] == other.bytes
    }
}

impl<const N: usize> PartialEq<Bytes> for &[u8; N] {
    #[inline]
    fn eq(&self, other: &Bytes) -> bool {
        self[..] == other.bytes
    }
}

impl PartialEq<[u8]> for Bytes {
    #[inline]
    fn eq(&self, other: &[u8]) -> bool {
        self.bytes == other
    }
}

impl PartialEq<Bytes> for [u8] {
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

        impl de::Visitor<'_> for Visitor {
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

impl TryFrom<&[u8]> for Value {
    type Error = alloc::Error;

    #[inline]
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Value::new(Bytes::try_from(value)?)
    }
}

impl IntoOutput for &[u8] {
    #[inline]
    fn into_output(self) -> Result<Value, RuntimeError> {
        Ok(Value::try_from(self)?)
    }
}

/// This is a common index get implementation that is helpfull for custom type to impl `INDEX_GET` protocol.
pub fn bytes_slice_index_get(this: &[u8], index: Value) -> VmResult<Option<Value>> {
    let slice: Option<&[u8]> = 'out: {
        if let Some(value) = index.as_any() {
            match value.type_hash() {
                RangeFrom::HASH => {
                    let range = vm_try!(value.borrow_ref::<RangeFrom>());
                    let start = vm_try!(range.start.as_usize());
                    break 'out this.get(start..);
                }
                RangeFull::HASH => {
                    _ = vm_try!(value.borrow_ref::<RangeFull>());
                    break 'out this.get(..);
                }
                RangeInclusive::HASH => {
                    let range = vm_try!(value.borrow_ref::<RangeInclusive>());
                    let start = vm_try!(range.start.as_usize());
                    let end = vm_try!(range.end.as_usize());
                    break 'out this.get(start..=end);
                }
                RangeToInclusive::HASH => {
                    let range = vm_try!(value.borrow_ref::<RangeToInclusive>());
                    let end = vm_try!(range.end.as_usize());
                    break 'out this.get(..=end);
                }
                RangeTo::HASH => {
                    let range = vm_try!(value.borrow_ref::<RangeTo>());
                    let end = vm_try!(range.end.as_usize());
                    break 'out this.get(..end);
                }
                Range::HASH => {
                    let range = vm_try!(value.borrow_ref::<Range>());
                    let start = vm_try!(range.start.as_usize());
                    let end = vm_try!(range.end.as_usize());
                    break 'out this.get(start..end);
                }
                _ => {}
            }
        };

        let index = vm_try!(usize::from_value(index));
        let Some(value) = this.get(index) else {
            return VmResult::Ok(None);
        };

        return VmResult::Ok(Some((*value).into()));
    };

    let Some(values) = slice else {
        return VmResult::Ok(None);
    };

    let bytes = vm_try!(Bytes::try_from(values));
    VmResult::Ok(Some(vm_try!(bytes.try_into())))
}
