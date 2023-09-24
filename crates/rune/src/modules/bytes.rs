//! The `std::bytes` module.

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::Vec;
use crate::runtime::{Bytes, VmResult};
use crate::{ContextError, Module};

/// Construct the `std::bytes` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["bytes"])?;

    module.ty::<Bytes>()?;
    module.function_meta(new)?;
    module.function_meta(with_capacity)?;
    module.function_meta(from_vec)?;
    module.function_meta(into_vec)?;
    module.function_meta(as_vec)?;
    module.function_meta(extend)?;
    module.function_meta(extend_str)?;
    module.function_meta(pop)?;
    module.function_meta(last)?;
    module.function_meta(len)?;
    module.function_meta(is_empty)?;
    module.function_meta(capacity)?;
    module.function_meta(clear)?;
    module.function_meta(reserve)?;
    module.function_meta(reserve_exact)?;
    module.function_meta(clone)?;
    module.function_meta(shrink_to_fit)?;
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
#[rune::function(free, path = Bytes::new)]
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
#[rune::function(free, path = Bytes::with_capacity)]
#[inline]
pub fn with_capacity(capacity: usize) -> VmResult<Bytes> {
    VmResult::Ok(vm_try!(Bytes::with_capacity(capacity)))
}

/// Convert a byte array into bytes.
///
/// # Examples
///
/// ```rune
/// let bytes = Bytes::from_vec([b'a', b'b', b'c', b'd']);
/// assert_eq!(bytes, b"abcd");
/// ```
#[rune::function(free, path = Bytes::from_vec)]
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
/// assert_eq!([b'a', b'b', b'c', b'd'], bytes.into_vec());
///
/// assert!(!is_readable(bytes));
/// ```
#[rune::function(instance)]
#[inline]
pub fn into_vec(bytes: Bytes) -> Vec<u8> {
    bytes.into_vec()
}

/// Convert the byte array into a vector of bytes without consuming it.
///
/// # Examples
///
/// ```rune
/// let bytes = b"abcd";
/// assert_eq!([b'a', b'b', b'c', b'd'], bytes.as_vec());
///
/// assert!(is_readable(bytes));
/// ```
#[rune::function(instance)]
#[inline]
pub fn as_vec(bytes: &Bytes) -> VmResult<Vec<u8>> {
    VmResult::Ok(vm_try!(Vec::try_from(bytes.as_slice())))
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
pub fn extend(this: &mut Bytes, other: &Bytes) -> VmResult<()> {
    vm_try!(this.extend(other));
    VmResult::Ok(())
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
pub fn extend_str(this: &mut Bytes, s: &str) -> VmResult<()> {
    vm_try!(this.extend(s.as_bytes()));
    VmResult::Ok(())
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

/// Returns the total number of elements the vector can hold without
/// reallocating.
///
/// # Examples
///
/// ```rune
/// let bytes = Bytes::with_capacity(10);
/// bytes.extend(b"abc");
/// assert!(bytes.capacity() >= 10);
/// ```
#[rune::function(instance)]
fn capacity(this: &Bytes) -> usize {
    this.capacity()
}

/// Clears the vector, removing all values.
///
/// Note that this method has no effect on the allocated capacity of the vector.
///
/// # Examples
///
/// ```rune
/// let bytes = b"abc";
/// bytes.clear();
/// assert!(bytes.is_empty());
/// ```
#[rune::function(instance)]
fn clear(this: &mut Bytes) {
    this.clear();
}

/// Reserves capacity for at least `additional` more elements to be inserted in
/// the given `Bytes`. The collection may reserve more space to speculatively
/// avoid frequent reallocations. After calling `reserve`, capacity will be
/// greater than or equal to `self.len() + additional`. Does nothing if capacity
/// is already sufficient.
///
/// # Panics
///
/// Panics if the new capacity exceeds `isize::MAX` bytes.
///
/// # Examples
///
/// ```rune
/// let vec = b"a";
/// vec.reserve(10);
/// assert!(vec.capacity() >= 11);
/// ```
#[rune::function(instance)]
fn reserve(this: &mut Bytes, additional: usize) -> VmResult<()> {
    vm_try!(this.reserve(additional));
    VmResult::Ok(())
}

/// Reserves the minimum capacity for at least `additional` more elements to be
/// inserted in the given `Bytes`. Unlike [`reserve`], this will not
/// deliberately over-allocate to speculatively avoid frequent allocations.
/// After calling `reserve_exact`, capacity will be greater than or equal to
/// `self.len() + additional`. Does nothing if the capacity is already
/// sufficient.
///
/// Note that the allocator may give the collection more space than it requests.
/// Therefore, capacity can not be relied upon to be precisely minimal. Prefer
/// [`reserve`] if future insertions are expected.
///
/// [`reserve`]: Bytes::reserve
///
/// # Panics
///
/// Panics if the new capacity exceeds `isize::MAX` bytes.
///
/// # Examples
///
/// ```rune
/// let vec = b"a";
/// vec.reserve_exact(10);
/// assert!(vec.capacity() >= 11);
/// ```
#[rune::function(instance)]
fn reserve_exact(this: &mut Bytes, additional: usize) -> VmResult<()> {
    vm_try!(this.reserve_exact(additional));
    VmResult::Ok(())
}

/// Clone the byte array.
///
/// # Examples
///
/// ```rune
/// let a = b"hello world";
/// let b = a.clone();
///
/// a.extend(b"!");
///
/// assert_eq!(a, b"hello world!");
/// assert_eq!(b, b"hello world");
/// ```
#[rune::function(instance)]
fn clone(this: &Bytes) -> VmResult<Bytes> {
    VmResult::Ok(vm_try!(this.try_clone()))
}

/// Shrinks the capacity of the byte array as much as possible.
///
/// It will drop down as close as possible to the length but the allocator may
/// still inform the byte array that there is space for a few more elements.
///
/// # Examples
///
/// ```rune
/// let bytes = Bytes::with_capacity(10);
/// bytes.extend(b"abc");
/// assert!(bytes.capacity() >= 10);
/// bytes.shrink_to_fit();
/// assert!(bytes.capacity() >= 3);
/// ```
#[rune::function(instance)]
fn shrink_to_fit(this: &mut Bytes) -> VmResult<()> {
    vm_try!(this.shrink_to_fit());
    VmResult::Ok(())
}
