//! The bytes module.

use core::cmp::Ordering;
use core::hash::Hasher as _;

use crate as rune;
use crate::alloc::fmt::TryWrite;
use crate::alloc::prelude::*;
use crate::alloc::Vec;
use crate::runtime::{Bytes, Formatter, Hasher, Panic, VmErrorKind, VmResult};
use crate::vm_try;
use crate::{ContextError, Module, Value};

/// The bytes module.
#[rune::module(::std::bytes)]
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module__meta)?;

    m.ty::<Bytes>()?;
    m.function_meta(new)?;
    m.function_meta(with_capacity)?;
    m.function_meta(from_vec)?;
    m.function_meta(into_vec)?;
    m.function_meta(as_vec)?;
    m.function_meta(extend)?;
    m.function_meta(extend_str)?;
    m.function_meta(pop)?;
    m.function_meta(push)?;
    m.function_meta(remove)?;
    m.function_meta(insert)?;
    m.function_meta(index_get)?;
    m.function_meta(index_set)?;
    m.function_meta(first)?;
    m.function_meta(last)?;
    m.function_meta(len)?;
    m.function_meta(is_empty)?;
    m.function_meta(capacity)?;
    m.function_meta(clear)?;
    m.function_meta(reserve)?;
    m.function_meta(reserve_exact)?;
    m.function_meta(shrink_to_fit)?;

    m.function_meta(clone__meta)?;
    m.implement_trait::<Bytes>(rune::item!(::std::clone::Clone))?;

    m.function_meta(partial_eq__meta)?;
    m.implement_trait::<Bytes>(rune::item!(::std::cmp::PartialEq))?;

    m.function_meta(eq__meta)?;
    m.implement_trait::<Bytes>(rune::item!(::std::cmp::Eq))?;

    m.function_meta(partial_cmp__meta)?;
    m.implement_trait::<Bytes>(rune::item!(::std::cmp::PartialOrd))?;

    m.function_meta(cmp__meta)?;
    m.implement_trait::<Bytes>(rune::item!(::std::cmp::Ord))?;

    m.function_meta(hash__meta)?;

    m.function_meta(debug_fmt__meta)?;

    Ok(m)
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

/// Append a byte to the back.
///
/// # Examples
///
/// ```rune
/// let bytes = b"abcd";
/// bytes.push(b'e');
/// assert_eq!(bytes, b"abcde");
/// ```
#[rune::function(instance)]
#[inline]
pub fn push(this: &mut Bytes, value: u8) -> VmResult<()> {
    vm_try!(this.push(value));
    VmResult::Ok(())
}

/// Removes and returns the byte at position `index` within the Bytes,
/// shifting all bytes after it to the left.
///
/// # Panics
///
/// Panics if `index` is out of bounds.
///
/// ```rune,should_panic
/// let bytes = b"abc";
/// bytes.remove(3);
/// ```
///
/// # Examples
///
/// ```rune
/// let bytes = b"abc";
/// assert_eq!(bytes.remove(1), b'b');
/// assert_eq!(bytes, b"ac");
/// ```
#[rune::function(instance)]
fn remove(this: &mut Bytes, index: usize) -> VmResult<u8> {
    if index >= this.len() {
        return VmResult::err(VmErrorKind::OutOfRange {
            index: index.into(),
            length: this.len().into(),
        });
    }

    let value = this.remove(index);
    VmResult::Ok(value)
}

/// Inserts a byte at position `index` within the inner vector, shifting all
/// elements after it to the right.
///
/// # Panics
///
/// Panics if `index` is out of bounds.
///
/// # Examples
///
/// ```rune
/// let bytes = b"abc";
/// bytes.insert(1, b'e');
/// assert_eq!(bytes, b"aebc");
/// bytes.insert(4, b'd');
/// assert_eq!(bytes, b"aebcd");
/// ```
#[rune::function(instance)]
fn insert(this: &mut Bytes, index: usize, value: u8) -> VmResult<()> {
    if index > this.len() {
        return VmResult::err(VmErrorKind::OutOfRange {
            index: index.into(),
            length: this.len().into(),
        });
    }

    vm_try!(this.insert(index, value));
    VmResult::Ok(())
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
#[rune::function(keep, instance, protocol = CLONE)]
fn clone(this: &Bytes) -> VmResult<Bytes> {
    VmResult::Ok(vm_try!(this.try_clone()))
}

/// Test two byte arrays for partial equality.
///
/// # Examples
///
/// ```rune
/// use std::ops::partial_eq;
///
/// assert_eq!(partial_eq(b"a", b"a"), true);
/// assert_eq!(partial_eq(b"a", b"ab"), false);
/// assert_eq!(partial_eq(b"ab", b"a"), false);
/// ```
#[rune::function(keep, instance, protocol = PARTIAL_EQ)]
#[inline]
fn partial_eq(this: &[u8], rhs: &[u8]) -> bool {
    this.eq(rhs)
}

/// Test two byte arrays for total equality.
///
/// # Examples
///
/// ```rune
/// use std::ops::eq;
///
/// assert_eq!(eq(b"a", b"a"), true);
/// assert_eq!(eq(b"a", b"ab"), false);
/// assert_eq!(eq(b"ab", b"a"), false);
/// ```
#[rune::function(keep, instance, protocol = EQ)]
#[inline]
fn eq(this: &[u8], rhs: &[u8]) -> bool {
    this.eq(rhs)
}

/// Perform a partial ordered comparison between two byte arrays.
///
/// # Examples
///
/// ```rune
/// assert!(b"a" < b"ab");
/// assert!(b"ab" > b"a");
/// assert!(b"a" == b"a");
/// ```
///
/// Using explicit functions:
///
/// ```rune
/// use std::cmp::Ordering;
/// use std::ops::partial_cmp;
///
/// assert_eq!(partial_cmp(b"a", b"ab"), Some(Ordering::Less));
/// assert_eq!(partial_cmp(b"ab", b"a"), Some(Ordering::Greater));
/// assert_eq!(partial_cmp(b"a", b"a"), Some(Ordering::Equal));
/// ```
#[rune::function(keep, instance, protocol = PARTIAL_CMP)]
#[inline]
fn partial_cmp(this: &[u8], rhs: &[u8]) -> Option<Ordering> {
    this.partial_cmp(rhs)
}

/// Perform a totally ordered comparison between two byte arrays.
///
/// # Examples
///
/// ```rune
/// use std::cmp::Ordering;
/// use std::ops::cmp;
///
/// assert_eq!(cmp(b"a", b"ab"), Ordering::Less);
/// assert_eq!(cmp(b"ab", b"a"), Ordering::Greater);
/// assert_eq!(cmp(b"a", b"a"), Ordering::Equal);
/// ```
#[rune::function(keep, instance, protocol = CMP)]
#[inline]
fn cmp(this: &[u8], rhs: &[u8]) -> Ordering {
    this.cmp(rhs)
}

/// Hash the byte array.
///
/// # Examples
///
/// ```rune
/// use std::ops::hash;
///
/// let a = "hello";
/// let b = "hello";
///
/// assert_eq!(hash(a), hash(b));
/// ```
#[rune::function(keep, instance, protocol = HASH)]
fn hash(this: &[u8], hasher: &mut Hasher) {
    hasher.write(this);
}

/// Write a debug representation of a byte array.
///
/// # Examples
///
/// ```rune
/// println!("{:?}", b"Hello");
/// ```
#[rune::function(keep, instance, protocol = DEBUG_FMT)]
#[inline]
fn debug_fmt(this: &[u8], f: &mut Formatter) -> VmResult<()> {
    rune::vm_write!(f, "{this:?}")
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

/// Returns a subslice of Bytes.
///
/// - If given a position, returns the byte at that position.
/// - If given a range, returns the subslice corresponding to that range.
///
/// # Panics
///
/// Panics if `index` is out of bounds.
///
/// ```rune,should_panic
/// let bytes = b"abc";
/// assert_eq!(None, bytes[1..4]);
/// ```
///
/// ```rune,should_panic
/// let bytes = b"abc";
/// assert_eq!(None, bytes[3]);
/// ```
///
/// # Examples
///
/// ```rune
/// let bytes = b"abcd";
/// assert_eq!(bytes[0..2], b"ab");
/// assert_eq!(bytes[0], b'a');
/// ```
#[rune::function(instance, protocol = INDEX_GET)]
fn index_get(this: &Bytes, index: Value) -> VmResult<Value> {
    match vm_try!(this.index_get(index)) {
        Some(bytes) => VmResult::Ok(bytes),
        None => VmResult::err(Panic::custom("missing bytes slice")),
    }
}

/// Inserts a byte into the Bytes.
///
/// # Examples
///
/// ```rune
/// let bytes = b"abcd";
/// bytes[1] = b'e';
/// assert_eq!(bytes, b"aecd");
/// ```
#[rune::function(instance, protocol = INDEX_SET)]
fn index_set(this: &mut Bytes, index: usize, value: u8) -> VmResult<()> {
    this.set(index, value)
}
