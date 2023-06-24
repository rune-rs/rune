//! The `std::bytes` module.

use crate::no_std::prelude::*;

use crate as rune;
use crate::runtime::Bytes;
use crate::{ContextError, Module};

/// Construct the `std::bytes` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["bytes"]);

    module.ty::<Bytes>()?;
    module.function_meta(new)?;
    module.function_meta(with_capacity)?;
    module.function_meta(from_vec)?;
    module.function_meta(into_vec)?;
    module.function_meta(extend)?;
    module.function_meta(extend_str)?;
    module.function_meta(pop)?;
    module.function_meta(last)?;
    module.function_meta(len)?;
    module.function_meta(is_empty)?;

    module.associated_function("capacity", Bytes::capacity)?;
    module.associated_function("clear", Bytes::clear)?;
    module.associated_function("reserve", Bytes::reserve)?;
    module.associated_function("reserve_exact", Bytes::reserve_exact)?;
    module.associated_function("clone", Bytes::clone)?;
    module.associated_function("shrink_to_fit", Bytes::shrink_to_fit)?;
    Ok(module)
}

/// Construct a new byte array.
///
/// # Examples
///
/// ```rune
/// let bytes = Bytes::new();
/// assert_eq!(bytes, b"");
/// ```
#[rune::function(path = Bytes::new)]
#[inline]
pub const fn new() -> Bytes {
    Bytes::new()
}

/// Construct a byte array with the given preallocated capacity.
///
/// # Examples
///
/// ```rune
/// let bytes = Bytes::with_capacity(32);
/// assert_eq!(bytes, b"");
/// bytes.extend(b"abcd");
/// assert_eq!(bytes, b"abcd");
/// ```
#[rune::function(path = Bytes::with_capacity)]
#[inline]
pub fn with_capacity(capacity: usize) -> Bytes {
    Bytes::with_capacity(capacity)
}

/// Convert a byte array into bytes.
///
/// # Examples
///
/// ```rune
/// let bytes = Bytes::from_vec([b'a', b'b', b'c', b'd']);
/// assert_eq!(bytes, b"abcd");
/// ```
#[rune::function(path = Bytes::from_vec)]
#[inline]
pub fn from_vec(bytes: Vec<u8>) -> Bytes {
    Bytes::from_vec(bytes)
}

/// Convert the byte array into a vector of bytes.
///
/// # Examples
///
/// ```rune
/// let bytes = b"abcd";
/// assert_eq!(bytes.into_vec(), [b'a', b'b', b'c', b'd']);
/// ```
#[rune::function(instance, path = Bytes::into_vec)]
#[inline]
pub fn into_vec(bytes: Bytes) -> Vec<u8> {
    bytes.into_vec()
}

/// Extend these bytes with another collection of bytes.
///
/// # Examples
///
/// ```rune
/// let bytes = b"abcd";
/// bytes.extend(b"efgh");
/// assert_eq!(bytes, b"abcdefgh");
/// ```
#[rune::function(instance)]
#[inline]
pub fn extend(this: &mut Bytes, other: &Bytes) {
    this.extend(other);
}

/// Extend this bytes collection with a string.
///
/// # Examples
///
/// ```rune
/// let bytes = b"abcd";
/// bytes.extend_str("efgh");
/// assert_eq!(bytes, b"abcdefgh");
/// ```
#[rune::function(instance)]
pub fn extend_str(this: &mut Bytes, s: &str) {
    this.extend(s.as_bytes());
}

/// Pop the last byte.
///
/// # Examples
///
/// ```rune
/// let bytes = b"abcd";
/// assert_eq!(bytes.pop(), Some(b'd'));
/// assert_eq!(bytes, b"abc");
/// ```
#[rune::function(instance)]
#[inline]
pub fn pop(this: &mut Bytes) -> Option<u8> {
    this.pop()
}

/// Get the first byte.
///
/// # Examples
///
/// ```rune
/// let bytes = b"abcd";
/// assert_eq!(bytes.first(), Some(b'a'));
/// ```
#[rune::function(instance)]
#[inline]
pub fn first(this: &Bytes) -> Option<u8> {
    this.first()
}

/// Get the last byte.
///
/// # Examples
///
/// ```rune
/// let bytes = b"abcd";
/// assert_eq!(bytes.last(), Some(b'd'));
/// ```
#[rune::function(instance)]
#[inline]
pub fn last(this: &Bytes) -> Option<u8> {
    this.last()
}

/// Get the length of the bytes collection.
///
/// # Examples
///
/// ```rune
/// let bytes = Bytes::new();
/// assert_eq!(bytes.len(), 0);
/// bytes.extend(b"abcd");
/// assert_eq!(bytes.len(), 4);
/// ```
#[rune::function(instance)]
#[inline]
pub fn len(this: &Bytes) -> usize {
    this.len()
}

/// Test if the collection is empty.
///
/// # Examples
///
/// ```rune
/// let bytes = Bytes::new();
/// assert!(bytes.is_empty());
/// ```
#[rune::function(instance)]
#[inline]
pub fn is_empty(this: &Bytes) -> bool {
    this.is_empty()
}
