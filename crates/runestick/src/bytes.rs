//! A container of bytes, corresponding to the [Value::Bytes] type.
//!
//! [Value::Bytes]: crate::Value::Bytes.

use crate::{FromValue, Mut, Named, RawMut, RawRef, RawStr, Ref, UnsafeFromValue, Value, VmError};

use std::fmt;
use std::ops;

/// A vector of bytes.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bytes {
    pub(crate) bytes: Vec<u8>,
}

impl Bytes {
    /// Construct a new bytes container.
    pub const fn new() -> Self {
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

    /// Test if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
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

impl FromValue for Bytes {
    fn from_value(value: Value) -> Result<Self, VmError> {
        let bytes = value.into_bytes()?;
        Ok(bytes.borrow_ref()?.clone())
    }
}

impl<'a> UnsafeFromValue for &'a Bytes {
    type Output = *const Bytes;
    type Guard = RawRef;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let bytes = value.into_bytes()?;
        let bytes = bytes.into_ref()?;
        Ok(Ref::into_raw(bytes))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl<'a> UnsafeFromValue for &'a mut Bytes {
    type Output = *mut Bytes;
    type Guard = RawMut;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let bytes = value.into_bytes()?;
        let bytes = bytes.into_mut()?;
        Ok(Mut::into_raw(bytes))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &mut *output
    }
}

impl<'a> UnsafeFromValue for &'a [u8] {
    type Output = *const [u8];
    type Guard = RawRef;

    unsafe fn unsafe_from_value(value: Value) -> Result<(Self::Output, Self::Guard), VmError> {
        let bytes = value.into_bytes()?;
        let bytes = bytes.into_ref()?;
        let (value, guard) = Ref::into_raw(bytes);
        Ok(((*value).bytes.as_slice(), guard))
    }

    unsafe fn to_arg(output: Self::Output) -> Self {
        &*output
    }
}

impl Named for Bytes {
    const NAME: RawStr = RawStr::from_str("Bytes");
}
