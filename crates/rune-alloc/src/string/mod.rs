//! A UTF-8–encoded, growable string.
//!
//! This module contains the [`String`] type, the [`TryToString`] trait for
//! converting to strings, and several error types that may result from working
//! with [`String`]s.
//!
//! # Examples
//!
//! There are multiple ways to create a new [`String`] from a string literal:
//!
//! ```
//! use rune::alloc::String;
//! use rune::alloc::prelude::*;
//!
//! let s = "Hello".try_to_string()?;
//!
//! let s = String::try_from("world")?;
//! let s: String = "also this".try_into()?;
//! # Ok::<_, rune::alloc::Error>(())
//! ```
//!
//! If you have a vector of valid UTF-8 bytes, you can make a [`String`] out of
//! it. You can do the reverse too.
//!
//! ```
//! use rune::alloc::{try_vec, String};
//! use rune::alloc::prelude::*;
//!
//! let sparkle_heart = try_vec![240, 159, 146, 150];
//! let sparkle_heart = String::from_utf8(sparkle_heart)?;
//!
//! assert_eq!("💖", sparkle_heart);
//!
//! let bytes = sparkle_heart.into_bytes();
//!
//! assert_eq!(bytes, [240, 159, 146, 150]);
//! # Ok::<_, Box<dyn std::error::Error>>(())
//! ```

#[cfg(feature = "serde")]
mod serde;

pub use self::try_to_string::TryToString;
pub(crate) mod try_to_string;

#[cfg(feature = "alloc")]
use core::alloc::Layout;
use core::borrow::Borrow;
use core::cmp::Ordering;
use core::fmt;
use core::hash;
use core::iter::FusedIterator;
#[cfg(feature = "alloc")]
use core::mem::ManuallyDrop;
use core::ops::Bound::{Excluded, Included, Unbounded};
use core::ops::{self, Index, IndexMut, Range, RangeBounds};
use core::ptr;
use core::slice;
use core::str::{from_utf8, from_utf8_unchecked, from_utf8_unchecked_mut};
use core::str::{Chars, Utf8Error};

use crate::alloc::{Allocator, Global};
use crate::borrow::Cow;
use crate::boxed::Box;
use crate::clone::TryClone;
use crate::error::Error;
use crate::fmt::TryWrite;
use crate::iter::{TryExtend, TryFromIteratorIn, TryJoin};
use crate::slice::range as slice_range;
#[cfg(test)]
use crate::testing::*;
use crate::vec::Vec;

/// A UTF-8–encoded, growable string.
///
/// The `String` type is the most common string type that has ownership over the
/// contents of the string. It has a close relationship with its borrowed
/// counterpart, the primitive [`str`].
///
/// # Examples
///
/// You can create a `String` from [a literal string][`&str`] with
/// [`String::try_from`]:
///
/// [`String::try_from`]: TryFrom::try_from
///
/// ```
/// use rune::alloc::String;
///
/// let hello = String::try_from("Hello, world!")?;
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// You can append a [`char`] to a `String` with the [`try_push`] method, and
/// append a [`&str`] with the [`try_push_str`] method:
///
/// ```
/// use rune::alloc::String;
///
/// let mut hello = String::try_from("Hello, ")?;
///
/// hello.try_push('w')?;
/// hello.try_push_str("orld!")?;
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// [`try_push`]: String::try_push
/// [`try_push_str`]: String::try_push_str
///
/// If you have a vector of UTF-8 bytes, you can create a `String` from it with
/// the [`from_utf8`] method:
///
/// ```
/// use rune::alloc::{try_vec, String};
///
/// // some bytes, in a vector
/// let sparkle_heart = try_vec![240, 159, 146, 150];
/// let sparkle_heart = String::from_utf8(sparkle_heart)?;
///
/// assert_eq!("💖", sparkle_heart);
/// # Ok::<_, Box<dyn std::error::Error>>(())
/// ```
///
/// [`from_utf8`]: String::from_utf8
///
/// # UTF-8
///
/// `String`s are always valid UTF-8. If you need a non-UTF-8 string, consider
/// [`OsString`]. It is similar, but without the UTF-8 constraint. Because UTF-8
/// is a variable width encoding, `String`s are typically smaller than an array of
/// the same `chars`:
///
/// ```
/// use core::mem;
///
/// // `s` is ASCII which represents each `char` as one byte
/// let s = "hello";
/// assert_eq!(s.len(), 5);
///
/// // A `char` array with the same contents would be longer because
/// // every `char` is four bytes
/// let s = ['h', 'e', 'l', 'l', 'o'];
/// let size: usize = s.into_iter().map(|c| mem::size_of_val(&c)).sum();
/// assert_eq!(size, 20);
///
/// // However, for non-ASCII strings, the difference will be smaller
/// // and sometimes they are the same
/// let s = "💖💖💖💖💖";
/// assert_eq!(s.len(), 20);
///
/// let s = ['💖', '💖', '💖', '💖', '💖'];
/// let size: usize = s.into_iter().map(|c| mem::size_of_val(&c)).sum();
/// assert_eq!(size, 20);
/// ```
///
/// This raises interesting questions as to how `s[i]` should work.
/// What should `i` be here? Several options include byte indices and
/// `char` indices but, because of UTF-8 encoding, only byte indices
/// would provide constant time indexing. Getting the `i`th `char`, for
/// example, is available using [`chars`]:
///
/// ```
/// let s = "hello";
/// let third_character = s.chars().nth(2);
/// assert_eq!(third_character, Some('l'));
///
/// let s = "💖💖💖💖💖";
/// let third_character = s.chars().nth(2);
/// assert_eq!(third_character, Some('💖'));
/// ```
///
/// Next, what should `s[i]` return? Because indexing returns a reference
/// to underlying data it could be `&u8`, `&[u8]`, or something else similar.
/// Since we're only providing one index, `&u8` makes the most sense but that
/// might not be what the user expects and can be explicitly achieved with
/// [`as_bytes()`]:
///
/// ```
/// // The first byte is 104 - the byte value of `'h'`
/// let s = "hello";
/// assert_eq!(s.as_bytes()[0], 104);
/// // or
/// assert_eq!(s.as_bytes()[0], b'h');
///
/// // The first byte is 240 which isn't obviously useful
/// let s = "💖💖💖💖💖";
/// assert_eq!(s.as_bytes()[0], 240);
/// ```
///
/// Due to these ambiguities/restrictions, indexing with a `usize` is simply
/// forbidden:
///
/// ```compile_fail,E0277
/// let s = "hello";
///
/// // The following will not compile!
/// println!("The first letter of s is {}", s[0]);
/// ```
///
/// It is more clear, however, how `&s[i..j]` should work (that is,
/// indexing with a range). It should accept byte indices (to be constant-time)
/// and return a `&str` which is UTF-8 encoded. This is also called "string slicing".
/// Note this will panic if the byte indices provided are not character
/// boundaries - see [`is_char_boundary`] for more details. See the implementations
/// for [`SliceIndex<str>`] for more details on string slicing. For a non-panicking
/// version of string slicing, see [`get`].
///
/// [`OsString`]: ../../std/ffi/struct.OsString.html "ffi::OsString"
/// [`SliceIndex<str>`]: core::slice::SliceIndex
/// [`as_bytes()`]: str::as_bytes
/// [`get`]: str::get
/// [`is_char_boundary`]: str::is_char_boundary
///
/// The [`bytes`] and [`chars`] methods return iterators over the bytes and
/// codepoints of the string, respectively. To iterate over codepoints along
/// with byte indices, use [`char_indices`].
///
/// [`bytes`]: str::bytes
/// [`chars`]: str::chars
/// [`char_indices`]: str::char_indices
///
/// # Deref
///
/// `String` implements <code>[Deref]<Target = [str]></code>, and so inherits all of [`str`]'s
/// methods. In addition, this means that you can pass a `String` to a
/// function which takes a [`&str`] by using an ampersand (`&`):
///
/// ```
/// use rune::alloc::String;
///
/// fn takes_str(s: &str) { }
///
/// let s = String::try_from("Hello")?;
///
/// takes_str(&s);
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// This will create a [`&str`] from the `String` and pass it in. This
/// conversion is very inexpensive, and so generally, functions will accept
/// [`&str`]s as arguments unless they need a `String` for some specific
/// reason.
///
/// In certain cases Rust doesn't have enough information to make this
/// conversion, known as [`Deref`] coercion. In the following example a string
/// slice [`&'a str`][`&str`] implements the trait `TraitExample`, and the function
/// `example_func` takes anything that implements the trait. In this case Rust
/// would need to make two implicit conversions, which Rust doesn't have the
/// means to do. For that reason, the following example will not compile.
///
/// ```compile_fail,E0277
/// use rune::alloc::String;
///
/// trait TraitExample {}
///
/// impl<'a> TraitExample for &'a str {}
///
/// fn example_func<A: TraitExample>(example_arg: A) {}
///
/// let example_string = String::try_from("example_string")?;
/// example_func(&example_string);
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// There are two options that would work instead. The first would be to
/// change the line `example_func(&example_string);` to
/// `example_func(example_string.as_str());`, using the method [`as_str()`]
/// to explicitly extract the string slice containing the string. The second
/// way changes `example_func(&example_string);` to
/// `example_func(&*example_string);`. In this case we are dereferencing a
/// `String` to a [`str`], then referencing the [`str`] back to
/// [`&str`]. The second way is more idiomatic, however both work to do the
/// conversion explicitly rather than relying on the implicit conversion.
///
/// # Representation
///
/// A `String` is made up of three components: a pointer to some bytes, a
/// length, and a capacity. The pointer points to an internal buffer `String`
/// uses to store its data. The length is the number of bytes currently stored
/// in the buffer, and the capacity is the size of the buffer in bytes. As such,
/// the length will always be less than or equal to the capacity.
///
/// This buffer is always stored on the heap.
///
/// You can look at these with the [`as_ptr`], [`len`], and [`capacity`]
/// methods:
///
/// ```
/// use core::mem;
/// use rune::alloc::String;
///
/// let story = String::try_from("Once upon a time...")?;
///
/// // Prevent automatically dropping the String's data
/// let mut story = mem::ManuallyDrop::new(story);
///
/// let ptr = story.as_mut_ptr();
/// let len = story.len();
/// let capacity = story.capacity();
/// let allocator = story.allocator().clone();
///
/// // story has nineteen bytes
/// assert_eq!(19, len);
///
/// // We can re-build a String out of ptr, len, and capacity. This is all
/// // unsafe because we are responsible for making sure the components are
/// // valid:
/// let s = unsafe { String::from_raw_parts_in(ptr, len, capacity, allocator) } ;
///
/// assert_eq!("Once upon a time...", s);
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// [`as_ptr`]: str::as_ptr
/// [`len`]: String::len
/// [`capacity`]: String::capacity
///
/// If a `String` has enough capacity, adding elements to it will not
/// re-allocate. For example, consider this program:
///
/// ```
/// use rune::alloc::String;
///
/// let mut s = String::new();
///
/// println!("{}", s.capacity());
///
/// for _ in 0..5 {
///     s.try_push_str("hello")?;
///     println!("{}", s.capacity());
/// }
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// This will output the following:
///
/// ```text
/// 0
/// 8
/// 16
/// 16
/// 32
/// 32
/// ```
///
/// At first, we have no memory allocated at all, but as we append to the
/// string, it increases its capacity appropriately. If we instead use the
/// [`try_with_capacity_in`] method to allocate the correct capacity initially:
///
/// ```
/// use rune::alloc::String;
/// use rune::alloc::alloc::Global;
///
/// let mut s = String::try_with_capacity_in(25, Global)?;
///
/// println!("{}", s.capacity());
///
/// for _ in 0..5 {
///     s.try_push_str("hello")?;
///     println!("{}", s.capacity());
/// }
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// [`try_with_capacity_in`]: String::try_with_capacity_in
///
/// We end up with a different output:
///
/// ```text
/// 25
/// 25
/// 25
/// 25
/// 25
/// 25
/// ```
///
/// Here, there's no need to allocate more memory inside the loop.
///
/// [str]: prim@str "str"
/// [`str`]: prim@str "str"
/// [`&str`]: prim@str "&str"
/// [Deref]: core::ops::Deref "ops::Deref"
/// [`Deref`]: core::ops::Deref "ops::Deref"
/// [`as_str()`]: String::as_str
pub struct String<A: Allocator = Global> {
    vec: Vec<u8, A>,
}

impl String {
    /// Creates a new empty `String`.
    ///
    /// Given that the `String` is empty, this will not allocate any initial
    /// buffer. While that means that this initial operation is very
    /// inexpensive, it may cause excessive allocation later when you add data.
    /// If you have an idea of how much data the `String` will hold, consider
    /// the [`try_with_capacity`] method to prevent excessive re-allocation.
    ///
    /// [`try_with_capacity`]: String::try_with_capacity
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::String;
    ///
    /// let s = String::new();
    /// ```
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        String { vec: Vec::new() }
    }

    /// Creates a new empty `String` with at least the specified capacity.
    ///
    /// `String`s have an internal buffer to hold their data. The capacity is
    /// the length of that buffer, and can be queried with the [`capacity`]
    /// method. This method creates an empty `String`, but one with an initial
    /// buffer that can hold at least `capacity` bytes. This is useful when you
    /// may be appending a bunch of data to the `String`, reducing the number of
    /// reallocations it needs to do.
    ///
    /// [`capacity`]: String::capacity
    ///
    /// If the given capacity is `0`, no allocation will occur, and this method
    /// is identical to the [`new`] method.
    ///
    /// [`new`]: String::new
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::String;
    ///
    /// let mut s = String::try_with_capacity(10)?;
    ///
    /// // The String contains no chars, even though it has capacity for more
    /// assert_eq!(s.len(), 0);
    ///
    /// // These are all done without reallocating...
    /// let cap = s.capacity();
    ///
    /// for _ in 0..10 {
    ///     s.try_push('a')?;
    /// }
    ///
    /// assert_eq!(s.capacity(), cap);
    ///
    /// // ...but this may make the string reallocate
    /// s.try_push('a')?;
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn try_with_capacity(capacity: usize) -> Result<Self, Error> {
        Ok(String {
            vec: Vec::try_with_capacity_in(capacity, Global)?,
        })
    }

    /// Convert a [`String`] into a std `String`.
    ///
    /// The result is allocated on the heap, using the default global allocator
    /// so this is a zero-copy operation.
    ///
    /// The memory previously occupied by this vector will be released.
    #[cfg(feature = "alloc")]
    pub fn into_std(self) -> ::rust_alloc::string::String {
        // SAFETY: The interior vector is valid UTF-8.
        unsafe { ::rust_alloc::string::String::from_utf8_unchecked(self.vec.into_std()) }
    }

    #[cfg(test)]
    pub fn from(value: &str) -> Self {
        Self::try_from(value).abort()
    }
}

/// A possible error value when converting a `String` from a UTF-8 byte vector.
///
/// This type is the error type for the [`from_utf8`] method on [`String`]. It
/// is designed in such a way to carefully avoid reallocations: the
/// [`into_bytes`] method will give back the byte vector that was used in the
/// conversion attempt.
///
/// [`from_utf8`]: String::from_utf8
/// [`into_bytes`]: FromUtf8Error::into_bytes
///
/// The [`Utf8Error`] type provided by [`std::str`] represents an error that may
/// occur when converting a slice of [`u8`]s to a [`&str`]. In this sense, it's
/// an analogue to `FromUtf8Error`, and you can get one from a `FromUtf8Error`
/// through the [`utf8_error`] method.
///
/// [`Utf8Error`]: core::str::Utf8Error "std::str::Utf8Error"
/// [`std::str`]: core::str "std::str"
/// [`&str`]: prim@str "&str"
/// [`utf8_error`]: FromUtf8Error::utf8_error
///
/// # Examples
///
/// ```
/// use rune::alloc::{try_vec, String};
///
/// // some invalid bytes, in a vector
/// let bytes = try_vec![0, 159];
///
/// let value = String::from_utf8(bytes);
///
/// assert!(value.is_err());
/// assert_eq!(try_vec![0, 159], value.unwrap_err().into_bytes());
/// # Ok::<_, rune::alloc::Error>(())
/// ```
pub struct FromUtf8Error<A: Allocator = Global> {
    bytes: Vec<u8, A>,
    error: Utf8Error,
}

impl<A: Allocator> fmt::Debug for FromUtf8Error<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FromUtf8Error")
            .field("bytes", &self.bytes)
            .field("error", &self.error)
            .finish()
    }
}

impl<A: Allocator> PartialEq for FromUtf8Error<A> {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes && self.error == other.error
    }
}

impl<A: Allocator> Eq for FromUtf8Error<A> {}

impl<A: Allocator> String<A> {
    /// Creates a new empty `String`.
    ///
    /// Given that the `String` is empty, this will not allocate any initial
    /// buffer. While that means that this initial operation is very
    /// inexpensive, it may cause excessive allocation later when you add data.
    /// If you have an idea of how much data the `String` will hold, consider
    /// the [`try_with_capacity_in`] method to prevent excessive re-allocation.
    ///
    /// [`try_with_capacity_in`]: String::try_with_capacity_in
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    /// use rune::alloc::alloc::Global;
    ///
    /// let s = String::new_in(Global);
    /// ```
    #[inline]
    #[must_use]
    pub fn new_in(alloc: A) -> String<A> {
        String {
            vec: Vec::new_in(alloc),
        }
    }

    /// Returns a reference to the underlying allocator.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    /// use rune::alloc::alloc::Global;
    ///
    /// let s = String::new_in(Global);
    /// let alloc: &Global = s.allocator();
    /// ```
    #[inline]
    pub fn allocator(&self) -> &A {
        self.vec.allocator()
    }

    /// Creates a new empty `String` with at least the specified capacity.
    ///
    /// `String`s have an internal buffer to hold their data. The capacity is
    /// the length of that buffer, and can be queried with the [`capacity`]
    /// method. This method creates an empty `String`, but one with an initial
    /// buffer that can hold at least `capacity` bytes. This is useful when you
    /// may be appending a bunch of data to the `String`, reducing the number of
    /// reallocations it needs to do.
    ///
    /// [`capacity`]: String::capacity
    ///
    /// If the given capacity is `0`, no allocation will occur, and this method
    /// is identical to the [`new_in`] method.
    ///
    /// [`new_in`]: String::new_in
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    /// use rune::alloc::alloc::Global;
    ///
    /// let mut s = String::try_with_capacity_in(10, Global)?;
    ///
    /// // The String contains no chars, even though it has capacity for more
    /// assert_eq!(s.len(), 0);
    ///
    /// // These are all done without reallocating...
    /// let cap = s.capacity();
    ///
    /// for _ in 0..10 {
    ///     s.try_push('a')?;
    /// }
    ///
    /// assert_eq!(s.capacity(), cap);
    ///
    /// // ...but this may make the string reallocate
    /// s.try_push('a')?;
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn try_with_capacity_in(capacity: usize, alloc: A) -> Result<String<A>, Error> {
        Ok(String {
            vec: Vec::try_with_capacity_in(capacity, alloc)?,
        })
    }

    /// Converts a vector of bytes to a `String`.
    ///
    /// A string ([`String`]) is made of bytes ([`u8`]), and a vector of bytes
    /// ([`Vec<u8>`]) is made of bytes, so this function converts between the
    /// two. Not all byte slices are valid `String`s, however: `String` requires
    /// that it is valid UTF-8. `from_utf8()` checks to ensure that the bytes
    /// are valid UTF-8, and then does the conversion.
    ///
    /// If you are sure that the byte slice is valid UTF-8, and you don't want
    /// to incur the overhead of the validity check, there is an unsafe version
    /// of this function, [`from_utf8_unchecked`], which has the same behavior
    /// but skips the check.
    ///
    /// This method will take care to not copy the vector, for efficiency's
    /// sake.
    ///
    /// If you need a [`&str`] instead of a `String`, consider
    /// [`str::from_utf8`].
    ///
    /// The inverse of this method is [`into_bytes`].
    ///
    /// [`str::from_utf8`]: core::str::from_utf8
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the slice is not UTF-8 with a description as to why
    /// the provided bytes are not UTF-8. The vector you moved in is also
    /// included.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::{try_vec, String};
    ///
    /// // some bytes, in a vector
    /// let sparkle_heart = try_vec![240, 159, 146, 150];
    /// let sparkle_heart = String::from_utf8(sparkle_heart)?;
    ///
    /// assert_eq!("💖", sparkle_heart);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// Incorrect bytes:
    ///
    /// ```
    /// use rune::alloc::{try_vec, String};
    ///
    /// // some invalid bytes, in a vector
    /// let sparkle_heart = try_vec![0, 159, 146, 150];
    ///
    /// assert!(String::from_utf8(sparkle_heart).is_err());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// See the docs for [`FromUtf8Error`] for more details on what you can do
    /// with this error.
    ///
    /// [`from_utf8_unchecked`]: String::from_utf8_unchecked
    /// [`Vec<u8>`]: crate::vec::Vec "Vec"
    /// [`&str`]: prim@str "&str"
    /// [`into_bytes`]: String::into_bytes
    #[inline]
    pub fn from_utf8(vec: Vec<u8, A>) -> Result<String<A>, FromUtf8Error<A>> {
        match from_utf8(&vec) {
            Ok(..) => Ok(String { vec }),
            Err(e) => Err(FromUtf8Error {
                bytes: vec,
                error: e,
            }),
        }
    }

    /// Creates a new `String` from a length, capacity, and pointer.
    ///
    /// # Safety
    ///
    /// This is highly unsafe, due to the number of invariants that aren't
    /// checked:
    ///
    /// * The memory at `buf` needs to have been previously allocated by the
    ///   same allocator the standard library uses, with a required alignment of exactly 1.
    /// * `length` needs to be less than or equal to `capacity`.
    /// * `capacity` needs to be the correct value.
    /// * The first `length` bytes at `buf` need to be valid UTF-8.
    ///
    /// Violating these may cause problems like corrupting the allocator's
    /// internal data structures. For example, it is normally **not** safe to
    /// build a `String` from a pointer to a C `char` array containing UTF-8
    /// _unless_ you are certain that array was originally allocated by the
    /// Rust standard library's allocator.
    ///
    /// The ownership of `buf` is effectively transferred to the
    /// `String` which may then deallocate, reallocate or change the
    /// contents of memory pointed to by the pointer at will. Ensure
    /// that nothing else uses the pointer after calling this
    /// function.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    /// use core::mem;
    ///
    /// unsafe {
    ///     let s = String::try_from("hello")?;
    ///
    ///     // Prevent automatically dropping the String's data
    ///     let mut s = mem::ManuallyDrop::new(s);
    ///
    ///     let ptr = s.as_mut_ptr();
    ///     let len = s.len();
    ///     let capacity = s.capacity();
    ///     let allocator = s.allocator().clone();
    ///
    ///     let s = String::from_raw_parts_in(ptr, len, capacity, allocator);
    ///
    ///     assert_eq!("hello", s);
    /// }
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub unsafe fn from_raw_parts_in(
        buf: *mut u8,
        length: usize,
        capacity: usize,
        alloc: A,
    ) -> String<A> {
        unsafe {
            String {
                vec: Vec::from_raw_parts_in(buf, length, capacity, alloc),
            }
        }
    }

    /// Converts a vector of bytes to a `String` without checking that the
    /// string contains valid UTF-8.
    ///
    /// See the safe version, [`from_utf8`], for more details.
    ///
    /// [`from_utf8`]: String::from_utf8
    ///
    /// # Safety
    ///
    /// This function is unsafe because it does not check that the bytes passed
    /// to it are valid UTF-8. If this constraint is violated, it may cause
    /// memory unsafety issues with future users of the `String`, as the rest of
    /// the standard library assumes that `String`s are valid UTF-8.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::{try_vec, String};
    ///
    /// // some bytes, in a vector
    /// let sparkle_heart = try_vec![240, 159, 146, 150];
    ///
    /// let sparkle_heart = unsafe {
    ///     String::from_utf8_unchecked(sparkle_heart)
    /// };
    ///
    /// assert_eq!("💖", sparkle_heart);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    #[must_use]
    pub unsafe fn from_utf8_unchecked(bytes: Vec<u8, A>) -> String<A> {
        String { vec: bytes }
    }

    /// Converts a `String` into a byte vector.
    ///
    /// This consumes the `String`, so we do not need to copy its contents.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    ///
    /// let s = String::try_from("hello")?;
    /// let bytes = s.into_bytes();
    ///
    /// assert_eq!(&[104, 101, 108, 108, 111][..], &bytes[..]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    #[must_use = "`self` will be dropped if the result is not used"]
    pub fn into_bytes(self) -> Vec<u8, A> {
        self.vec
    }

    /// Extracts a string slice containing the entire `String`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    ///
    /// let s = String::try_from("foo")?;
    ///
    /// assert_eq!("foo", s.as_str());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        self
    }

    /// Converts a `String` into a mutable string slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    ///
    /// let mut s = String::try_from("foobar")?;
    /// let s_mut_str = s.as_mut_str();
    ///
    /// s_mut_str.make_ascii_uppercase();
    ///
    /// assert_eq!("FOOBAR", s_mut_str);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    #[must_use]
    pub fn as_mut_str(&mut self) -> &mut str {
        self
    }

    /// Appends a given string slice onto the end of this `String`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    /// use rune::alloc::alloc::Global;
    ///
    /// let mut s = String::try_with_capacity_in(3, Global)?;
    ///
    /// s.try_push_str("foo")?;
    /// s.try_push_str("bar")?;
    ///
    /// assert_eq!("foobar", s);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn try_push_str(&mut self, string: &str) -> Result<(), Error> {
        self.vec.try_extend_from_slice(string.as_bytes())
    }

    #[cfg(test)]
    pub(crate) fn push_str(&mut self, string: &str) {
        self.try_push_str(string).abort()
    }

    /// Returns this `String`'s capacity, in bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    /// use rune::alloc::alloc::Global;
    ///
    /// let s = String::try_with_capacity_in(10, Global)?;
    ///
    /// assert!(s.capacity() >= 10);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.vec.capacity()
    }

    /// Tries to reserve capacity for at least `additional` bytes more than the
    /// current length. The allocator may reserve more space to speculatively
    /// avoid frequent allocations. After calling `try_reserve`, capacity will be
    /// greater than or equal to `self.len() + additional` if it returns
    /// `Ok(())`. Does nothing if capacity is already sufficient. This method
    /// preserves the contents even if an error occurs.
    ///
    /// # Errors
    ///
    /// If the capacity overflows, or the allocator reports a failure, then an error
    /// is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::{String, Error};
    ///
    /// fn process_data(data: &str) -> Result<String, Error> {
    ///     let mut output = String::new();
    ///
    ///     // Pre-reserve the memory, exiting if we can't
    ///     output.try_reserve(data.len())?;
    ///
    ///     // Now we know this can't OOM in the middle of our complex work
    ///     output.try_push_str(data)?;
    ///
    ///     Ok(output)
    /// }
    /// # process_data("rust").expect("why is the test harness OOMing on 4 bytes?");
    /// ```
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), Error> {
        self.vec.try_reserve(additional)
    }

    /// Tries to reserve the minimum capacity for at least `additional` bytes
    /// more than the current length. Unlike [`try_reserve`], this will not
    /// deliberately over-allocate to speculatively avoid frequent allocations.
    /// After calling `try_reserve_exact`, capacity will be greater than or
    /// equal to `self.len() + additional` if it returns `Ok(())`.
    /// Does nothing if the capacity is already sufficient.
    ///
    /// Note that the allocator may give the collection more space than it
    /// requests. Therefore, capacity can not be relied upon to be precisely
    /// minimal. Prefer [`try_reserve`] if future insertions are expected.
    ///
    /// [`try_reserve`]: String::try_reserve
    ///
    /// # Errors
    ///
    /// If the capacity overflows, or the allocator reports a failure, then an error
    /// is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::{String, Error};
    ///
    /// fn process_data(data: &str) -> Result<String, Error> {
    ///     let mut output = String::new();
    ///
    ///     // Pre-reserve the memory, exiting if we can't
    ///     output.try_reserve_exact(data.len())?;
    ///
    ///     // Now we know this can't OOM in the middle of our complex work
    ///     output.try_push_str(data);
    ///
    ///     Ok(output)
    /// }
    /// # process_data("rust").expect("why is the test harness OOMing on 4 bytes?");
    /// ```
    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), Error> {
        self.vec.try_reserve_exact(additional)
    }

    /// Shrinks the capacity of this `String` to match its length.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    /// let mut s = String::try_from("foo")?;
    ///
    /// s.try_reserve(100)?;
    /// assert!(s.capacity() >= 100);
    ///
    /// s.try_shrink_to_fit()?;
    /// assert_eq!(3, s.capacity());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn try_shrink_to_fit(&mut self) -> Result<(), Error> {
        self.vec.try_shrink_to_fit()
    }

    /// Shrinks the capacity of this `String` with a lower bound.
    ///
    /// The capacity will remain at least as large as both the length
    /// and the supplied value.
    ///
    /// If the current capacity is less than the lower limit, this is a no-op.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    ///
    /// let mut s = String::try_from("foo")?;
    ///
    /// s.try_reserve(100)?;
    /// assert!(s.capacity() >= 100);
    ///
    /// s.try_shrink_to(10)?;
    /// assert!(s.capacity() >= 10);
    /// s.try_shrink_to(0)?;
    /// assert!(s.capacity() >= 3);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn try_shrink_to(&mut self, min_capacity: usize) -> Result<(), Error> {
        self.vec.try_shrink_to(min_capacity)
    }

    /// Appends the given [`char`] to the end of this `String`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    /// use rune::alloc::alloc::Global;
    ///
    /// let mut s = String::try_with_capacity_in(3, Global)?;
    /// s.try_push_str("abc")?;
    ///
    /// s.try_push('1')?;
    /// s.try_push('2')?;
    /// s.try_push('3')?;
    ///
    /// assert_eq!("abc123", s);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn try_push(&mut self, ch: char) -> Result<(), Error> {
        match ch.len_utf8() {
            1 => self.vec.try_push(ch as u8),
            _ => self
                .vec
                .try_extend_from_slice(ch.encode_utf8(&mut [0; 4]).as_bytes()),
        }
    }

    /// Returns a byte slice of this `String`'s contents.
    ///
    /// The inverse of this method is [`from_utf8`].
    ///
    /// [`from_utf8`]: String::from_utf8
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    ///
    /// let s = String::try_from("hello")?;
    ///
    /// assert_eq!(&[104, 101, 108, 108, 111], s.as_bytes());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.vec
    }

    /// Shortens this `String` to the specified length.
    ///
    /// If `new_len` is greater than the string's current length, this has no
    /// effect.
    ///
    /// Note that this method has no effect on the allocated capacity
    /// of the string
    ///
    /// # Panics
    ///
    /// Panics if `new_len` does not lie on a [`char`] boundary.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    ///
    /// let mut s = String::try_from("hello")?;
    ///
    /// s.truncate(2);
    ///
    /// assert_eq!("he", s);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn truncate(&mut self, new_len: usize) {
        if new_len <= self.len() {
            assert!(self.is_char_boundary(new_len));
            self.vec.truncate(new_len)
        }
    }

    /// Removes the last character from the string buffer and returns it.
    ///
    /// Returns [`None`] if this `String` is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    ///
    /// let mut s = String::try_from("abč")?;
    ///
    /// assert_eq!(s.pop(), Some('č'));
    /// assert_eq!(s.pop(), Some('b'));
    /// assert_eq!(s.pop(), Some('a'));
    ///
    /// assert_eq!(s.pop(), None);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn pop(&mut self) -> Option<char> {
        let ch = self.chars().next_back()?;
        let newlen = self.len() - ch.len_utf8();
        unsafe {
            self.vec.set_len(newlen);
        }
        Some(ch)
    }

    /// Removes a [`char`] from this `String` at a byte position and returns it.
    ///
    /// This is an *O*(*n*) operation, as it requires copying every element in the
    /// buffer.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is larger than or equal to the `String`'s length,
    /// or if it does not lie on a [`char`] boundary.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    ///
    /// let mut s = String::try_from("abç")?;
    ///
    /// assert_eq!(s.remove(0), 'a');
    /// assert_eq!(s.remove(1), 'ç');
    /// assert_eq!(s.remove(0), 'b');
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn remove(&mut self, idx: usize) -> char {
        let ch = match self[idx..].chars().next() {
            Some(ch) => ch,
            None => panic!("cannot remove a char from the end of a string"),
        };

        let next = idx + ch.len_utf8();
        let len = self.len();
        unsafe {
            ptr::copy(
                self.vec.as_ptr().add(next),
                self.vec.as_mut_ptr().add(idx),
                len - next,
            );
            self.vec.set_len(len - (next - idx));
        }
        ch
    }

    /// Retains only the characters specified by the predicate.
    ///
    /// In other words, remove all characters `c` such that `f(c)` returns `false`.
    /// This method operates in place, visiting each character exactly once in the
    /// original order, and preserves the order of the retained characters.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    ///
    /// let mut s = String::try_from("f_o_ob_ar")?;
    ///
    /// s.retain(|c| c != '_');
    ///
    /// assert_eq!(s, "foobar");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    ///
    /// Because the elements are visited exactly once in the original order,
    /// external state may be used to decide which elements to keep.
    ///
    /// ```
    /// use rune::alloc::String;
    ///
    /// let mut s = String::try_from("abcde")?;
    /// let keep = [false, true, true, false, true];
    /// let mut iter = keep.iter();
    /// s.retain(|_| *iter.next().unwrap());
    /// assert_eq!(s, "bce");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(char) -> bool,
    {
        struct SetLenOnDrop<'a, A: Allocator> {
            s: &'a mut String<A>,
            idx: usize,
            del_bytes: usize,
        }

        impl<'a, A: Allocator> Drop for SetLenOnDrop<'a, A> {
            fn drop(&mut self) {
                let new_len = self.idx - self.del_bytes;
                debug_assert!(new_len <= self.s.len());
                unsafe { self.s.vec.set_len(new_len) };
            }
        }

        let len = self.len();
        let mut guard = SetLenOnDrop {
            s: self,
            idx: 0,
            del_bytes: 0,
        };

        while guard.idx < len {
            let ch =
                // SAFETY: `guard.idx` is positive-or-zero and less that len so the `get_unchecked`
                // is in bound. `self` is valid UTF-8 like string and the returned slice starts at
                // a unicode code point so the `Chars` always return one character.
                unsafe { guard.s.get_unchecked(guard.idx..len).chars().next().unwrap_unchecked() };
            let ch_len = ch.len_utf8();

            if !f(ch) {
                guard.del_bytes += ch_len;
            } else if guard.del_bytes > 0 {
                // SAFETY: `guard.idx` is in bound and `guard.del_bytes` represent the number of
                // bytes that are erased from the string so the resulting `guard.idx -
                // guard.del_bytes` always represent a valid unicode code point.
                //
                // `guard.del_bytes` >= `ch.len_utf8()`, so taking a slice with `ch.len_utf8()` len
                // is safe.
                ch.encode_utf8(unsafe {
                    slice::from_raw_parts_mut(
                        guard.s.as_mut_ptr().add(guard.idx - guard.del_bytes),
                        ch.len_utf8(),
                    )
                });
            }

            // Point idx to the next char
            guard.idx += ch_len;
        }

        drop(guard);
    }

    /// Inserts a character into this `String` at a byte position.
    ///
    /// This is an *O*(*n*) operation as it requires copying every element in the
    /// buffer.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is larger than the `String`'s length, or if it does not
    /// lie on a [`char`] boundary.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    /// use rune::alloc::alloc::Global;
    ///
    /// let mut s = String::try_with_capacity_in(3, Global)?;
    ///
    /// s.try_insert(0, 'f')?;
    /// s.try_insert(1, 'o')?;
    /// s.try_insert(2, 'o')?;
    ///
    /// assert_eq!(s, "foo");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn try_insert(&mut self, idx: usize, ch: char) -> Result<(), Error> {
        assert!(self.is_char_boundary(idx));
        let mut bits = [0; 4];
        let bits = ch.encode_utf8(&mut bits).as_bytes();

        unsafe {
            self.insert_bytes(idx, bits)?;
        }

        Ok(())
    }

    unsafe fn insert_bytes(&mut self, idx: usize, bytes: &[u8]) -> Result<(), Error> {
        let len = self.len();
        let amt = bytes.len();
        self.vec.try_reserve(amt)?;

        unsafe {
            ptr::copy(
                self.vec.as_ptr().add(idx),
                self.vec.as_mut_ptr().add(idx + amt),
                len - idx,
            );
            ptr::copy_nonoverlapping(bytes.as_ptr(), self.vec.as_mut_ptr().add(idx), amt);
            self.vec.set_len(len + amt);
        }

        Ok(())
    }

    /// Inserts a string slice into this `String` at a byte position.
    ///
    /// This is an *O*(*n*) operation as it requires copying every element in the
    /// buffer.
    ///
    /// # Panics
    ///
    /// Panics if `idx` is larger than the `String`'s length, or if it does not
    /// lie on a [`char`] boundary.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    ///
    /// let mut s = String::try_from("bar")?;
    ///
    /// s.try_insert_str(0, "foo")?;
    ///
    /// assert_eq!("foobar", s);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn try_insert_str(&mut self, idx: usize, string: &str) -> Result<(), Error> {
        assert!(self.is_char_boundary(idx));

        unsafe {
            self.insert_bytes(idx, string.as_bytes())?;
        }

        Ok(())
    }

    /// Returns a mutable reference to the contents of this `String`.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the returned `&mut Vec` allows writing
    /// bytes which are not valid UTF-8. If this constraint is violated, using
    /// the original `String` after dropping the `&mut Vec` may violate memory
    /// safety, as the rest of the standard library assumes that `String`s are
    /// valid UTF-8.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    ///
    /// let mut s = String::try_from("hello")?;
    ///
    /// unsafe {
    ///     let vec = s.as_mut_vec();
    ///     assert_eq!(&[104, 101, 108, 108, 111][..], &vec[..]);
    ///
    ///     vec.reverse();
    /// }
    /// assert_eq!(s, "olleh");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub unsafe fn as_mut_vec(&mut self) -> &mut Vec<u8, A> {
        &mut self.vec
    }

    /// Returns the length of this `String`, in bytes, not [`char`]s or
    /// graphemes. In other words, it might not be what a human considers the
    /// length of the string.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    ///
    /// let a = String::try_from("foo")?;
    /// assert_eq!(a.len(), 3);
    ///
    /// let fancy_f = String::try_from("ƒoo")?;
    /// assert_eq!(fancy_f.len(), 4);
    /// assert_eq!(fancy_f.chars().count(), 3);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    /// Returns `true` if this `String` has a length of zero, and `false`
    /// otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    ///
    /// let mut v = String::new();
    /// assert!(v.is_empty());
    ///
    /// v.try_push('a')?;
    /// assert!(!v.is_empty());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Splits the string into two at the given byte index.
    ///
    /// Returns a newly allocated `String`. `self` contains bytes `[0, at)`, and
    /// the returned `String` contains bytes `[at, len)`. `at` must be on the
    /// boundary of a UTF-8 code point.
    ///
    /// Note that the capacity of `self` does not change.
    ///
    /// # Panics
    ///
    /// Panics if `at` is not on a `UTF-8` code point boundary, or if it is beyond the last
    /// code point of the string.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    ///
    /// let mut hello = String::try_from("Hello, World!")?;
    /// let world = hello.try_split_off(7)?;
    /// assert_eq!(hello, "Hello, ");
    /// assert_eq!(world, "World!");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    #[must_use = "use `.truncate()` if you don't need the other half"]
    pub fn try_split_off(&mut self, at: usize) -> Result<String<A>, Error>
    where
        A: Clone,
    {
        assert!(self.is_char_boundary(at));
        let other = self.vec.try_split_off(at)?;
        Ok(unsafe { String::from_utf8_unchecked(other) })
    }

    /// Truncates this `String`, removing all contents.
    ///
    /// While this means the `String` will have a length of zero, it does not
    /// touch its capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    ///
    /// let mut s = String::try_from("foo")?;
    ///
    /// s.clear();
    ///
    /// assert!(s.is_empty());
    /// assert_eq!(0, s.len());
    /// assert_eq!(3, s.capacity());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.vec.clear()
    }

    /// Removes the specified range from the string in bulk, returning all
    /// removed characters as an iterator.
    ///
    /// The returned iterator keeps a mutable borrow on the string to optimize
    /// its implementation.
    ///
    /// # Panics
    ///
    /// Panics if the starting point or end point do not lie on a [`char`]
    /// boundary, or if they're out of bounds.
    ///
    /// # Leaking
    ///
    /// If the returned iterator goes out of scope without being dropped (due to
    /// [`core::mem::forget`], for example), the string may still contain a copy
    /// of any drained characters, or may have lost characters arbitrarily,
    /// including characters outside the range.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    /// use rune::alloc::prelude::*;
    ///
    /// let mut s = String::try_from("α is alpha, β is beta")?;
    /// let beta_offset = s.find('β').unwrap_or(s.len());
    ///
    /// // Remove the range up until the β from the string
    /// let t: String = s.drain(..beta_offset).try_collect()?;
    /// assert_eq!(t, "α is alpha, ");
    /// assert_eq!(s, "β is beta");
    ///
    /// // A full range clears the string, like `clear()` does
    /// s.drain(..);
    /// assert_eq!(s, "");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn drain<R>(&mut self, range: R) -> Drain<'_, A>
    where
        R: RangeBounds<usize>,
    {
        // Memory safety
        //
        // The String version of Drain does not have the memory safety issues
        // of the vector version. The data is just plain bytes.
        // Because the range removal happens in Drop, if the Drain iterator is leaked,
        // the removal will not happen.
        let Range { start, end } = slice_range(range, ..self.len());
        assert!(self.is_char_boundary(start));
        assert!(self.is_char_boundary(end));

        // Take out two simultaneous borrows. The &mut String won't be accessed
        // until iteration is over, in Drop.
        let self_ptr = self as *mut _;
        // SAFETY: `slice::range` and `is_char_boundary` do the appropriate bounds checks.
        let chars_iter = unsafe { self.get_unchecked(start..end) }.chars();

        Drain {
            start,
            end,
            iter: chars_iter,
            string: self_ptr,
        }
    }

    /// Removes the specified range in the string,
    /// and replaces it with the given string.
    /// The given string doesn't need to be the same length as the range.
    ///
    /// # Panics
    ///
    /// Panics if the starting point or end point do not lie on a [`char`]
    /// boundary, or if they're out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    ///
    /// let mut s = String::try_from("α is alpha, β is beta")?;
    /// let beta_offset = s.find('β').unwrap_or(s.len());
    ///
    /// // Replace the range up until the β from the string
    /// s.try_replace_range(..beta_offset, "Α is capital alpha; ")?;
    /// assert_eq!(s, "Α is capital alpha; β is beta");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    pub fn try_replace_range<R>(&mut self, range: R, replace_with: &str) -> Result<(), Error>
    where
        R: RangeBounds<usize>,
    {
        // Memory safety
        //
        // Replace_range does not have the memory safety issues of a vector Splice.
        // of the vector version. The data is just plain bytes.

        // WARNING: Inlining this variable would be unsound (#81138)
        let start = range.start_bound();
        match start {
            Included(&n) => assert!(self.is_char_boundary(n)),
            Excluded(&n) => assert!(self.is_char_boundary(n + 1)),
            Unbounded => {}
        };
        // WARNING: Inlining this variable would be unsound (#81138)
        let end = range.end_bound();
        match end {
            Included(&n) => assert!(self.is_char_boundary(n + 1)),
            Excluded(&n) => assert!(self.is_char_boundary(n)),
            Unbounded => {}
        };

        // Using `range` again would be unsound (#81138)
        // We assume the bounds reported by `range` remain the same, but
        // an adversarial implementation could change between calls
        unsafe { self.as_mut_vec() }.try_splice_in_place((start, end), replace_with.bytes())?;
        Ok(())
    }

    /// Converts this `String` into a <code>[Box]<[str]></code>.
    ///
    /// This will drop any excess capacity.
    ///
    /// [str]: prim@str "str"
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    /// let s = String::try_from("hello")?;
    ///
    /// let b = s.try_into_boxed_str()?;
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[must_use = "`self` will be dropped if the result is not used"]
    #[inline]
    pub fn try_into_boxed_str(self) -> Result<Box<str, A>, Error> {
        let slice = self.vec.try_into_boxed_slice()?;
        Ok(unsafe { crate::str::from_boxed_utf8_unchecked(slice) })
    }

    /// Consumes and leaks the `String`, returning a mutable reference to the contents,
    /// `&'a mut str`.
    ///
    /// The caller has free choice over the returned lifetime, including `'static`. Indeed,
    /// this function is ideally used for data that lives for the remainder of the program's life,
    /// as dropping the returned reference will cause a memory leak.
    ///
    /// It does not reallocate or shrink the `String`,
    /// so the leaked allocation may include unused capacity that is not part
    /// of the returned slice. If you don't want that, call [`try_into_boxed_str`],
    /// and then [`Box::leak`].
    ///
    /// [`try_into_boxed_str`]: Self::try_into_boxed_str
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(not(miri))]
    /// # fn main() -> Result<(), rune_alloc::Error> {
    /// use rune::alloc::String;
    ///
    /// let x = String::try_from("bucket")?;
    /// let static_ref: &'static mut str = x.leak();
    /// assert_eq!(static_ref, "bucket");
    /// # Ok(())
    /// # }
    /// # #[cfg(miri)] fn main() {}
    /// ```
    #[inline]
    pub fn leak<'a>(self) -> &'a mut str
    where
        A: 'a,
    {
        let slice = self.vec.leak();
        unsafe { from_utf8_unchecked_mut(slice) }
    }
}

impl<A: Allocator> FromUtf8Error<A> {
    /// Returns a slice of [`u8`]s bytes that were attempted to convert to a `String`.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::{try_vec, String};
    ///
    /// // some invalid bytes, in a vector
    /// let bytes = try_vec![0, 159];
    ///
    /// let value = String::from_utf8(bytes);
    ///
    /// assert_eq!(&[0, 159], value.unwrap_err().as_bytes());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..]
    }

    /// Returns the bytes that were attempted to convert to a `String`.
    ///
    /// This method is carefully constructed to avoid allocation. It will
    /// consume the error, moving out the bytes, so that a copy of the bytes
    /// does not need to be made.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::{try_vec, String};
    ///
    /// // some invalid bytes, in a vector
    /// let bytes = try_vec![0, 159];
    ///
    /// let value = String::from_utf8(bytes);
    ///
    /// assert_eq!(try_vec![0, 159], value.unwrap_err().into_bytes());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[must_use = "`self` will be dropped if the result is not used"]
    pub fn into_bytes(self) -> Vec<u8, A> {
        self.bytes
    }

    /// Fetch a `Utf8Error` to get more details about the conversion failure.
    ///
    /// The [`Utf8Error`] type provided by [`std::str`] represents an error that
    /// may occur when converting a slice of [`u8`]s to a [`&str`]. In this
    /// sense, it's an analogue to `FromUtf8Error`. See its documentation for
    /// more details on using it.
    ///
    /// [`std::str`]: core::str "std::str"
    /// [`&str`]: prim@str "&str"
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::{try_vec, String};
    ///
    /// // some invalid bytes, in a vector
    /// let bytes = try_vec![0, 159];
    ///
    /// let error = String::from_utf8(bytes).unwrap_err().utf8_error();
    ///
    /// // the first byte is invalid here
    /// assert_eq!(1, error.valid_up_to());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[must_use]
    pub fn utf8_error(&self) -> Utf8Error {
        self.error
    }
}

impl<A: Allocator> Default for String<A>
where
    A: Default,
{
    /// Construct a default string.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    /// let s = String::default();
    /// assert_eq!(s, "");
    /// ```
    fn default() -> Self {
        Self::new_in(A::default())
    }
}

impl<A: Allocator> Borrow<str> for String<A> {
    #[inline]
    fn borrow(&self) -> &str {
        &self[..]
    }
}

impl<A: Allocator> fmt::Display for FromUtf8Error<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}

#[cfg(feature = "std")]
impl<A: Allocator> std::error::Error for FromUtf8Error<A> {}

impl<A: Allocator + Clone> TryClone for String<A> {
    fn try_clone(&self) -> Result<Self, Error> {
        Ok(String {
            vec: self.vec.try_clone()?,
        })
    }
}

#[cfg(test)]
impl<A: Allocator + Clone> Clone for String<A> {
    fn clone(&self) -> Self {
        self.try_clone().abort()
    }
}

impl<A: Allocator> PartialEq for String<A> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.vec == other.vec
    }
}

impl<A: Allocator> Eq for String<A> {}

impl<A: Allocator> PartialOrd for String<A> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<A: Allocator> Ord for String<A> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.vec.cmp(&other.vec)
    }
}

macro_rules! impl_eq {
    ($lhs:ty, $rhs: ty) => {
        #[allow(unused_lifetimes)]
        #[allow(clippy::partialeq_ne_impl)]
        impl<'a, 'b> PartialEq<$rhs> for $lhs {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool {
                PartialEq::eq(&self[..], &other[..])
            }
            #[inline]
            fn ne(&self, other: &$rhs) -> bool {
                PartialEq::ne(&self[..], &other[..])
            }
        }

        #[allow(unused_lifetimes)]
        #[allow(clippy::partialeq_ne_impl)]
        impl<'a, 'b> PartialEq<$lhs> for $rhs {
            #[inline]
            fn eq(&self, other: &$lhs) -> bool {
                PartialEq::eq(&self[..], &other[..])
            }
            #[inline]
            fn ne(&self, other: &$lhs) -> bool {
                PartialEq::ne(&self[..], &other[..])
            }
        }
    };
}

impl_eq! { String, str }
impl_eq! { String, &'a str }
impl_eq! { Cow<'a, str>, str }
impl_eq! { Cow<'a, str>, &'b str }
impl_eq! { Cow<'a, str>, String }

impl<A: Allocator> fmt::Display for String<A> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<A: Allocator> fmt::Debug for String<A> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<A: Allocator> hash::Hash for String<A> {
    #[inline]
    fn hash<H: hash::Hasher>(&self, hasher: &mut H) {
        (**self).hash(hasher)
    }
}

impl<A: Allocator> ops::Index<ops::Range<usize>> for String<A> {
    type Output = str;

    #[inline]
    fn index(&self, index: ops::Range<usize>) -> &str {
        &self[..][index]
    }
}

impl<A: Allocator> ops::Index<ops::RangeTo<usize>> for String<A> {
    type Output = str;

    #[inline]
    fn index(&self, index: ops::RangeTo<usize>) -> &str {
        &self[..][index]
    }
}

impl<A: Allocator> ops::Index<ops::RangeFrom<usize>> for String<A> {
    type Output = str;

    #[inline]
    fn index(&self, index: ops::RangeFrom<usize>) -> &str {
        &self[..][index]
    }
}

impl<A: Allocator> ops::Index<ops::RangeFull> for String<A> {
    type Output = str;

    #[inline]
    fn index(&self, _index: ops::RangeFull) -> &str {
        unsafe { from_utf8_unchecked(&self.vec) }
    }
}

impl<A: Allocator> ops::Index<ops::RangeInclusive<usize>> for String<A> {
    type Output = str;

    #[inline]
    fn index(&self, index: ops::RangeInclusive<usize>) -> &str {
        Index::index(&**self, index)
    }
}

impl<A: Allocator> ops::Index<ops::RangeToInclusive<usize>> for String<A> {
    type Output = str;

    #[inline]
    fn index(&self, index: ops::RangeToInclusive<usize>) -> &str {
        Index::index(&**self, index)
    }
}

impl<A: Allocator> ops::IndexMut<ops::Range<usize>> for String<A> {
    #[inline]
    fn index_mut(&mut self, index: ops::Range<usize>) -> &mut str {
        &mut self[..][index]
    }
}

impl<A: Allocator> ops::IndexMut<ops::RangeTo<usize>> for String<A> {
    #[inline]
    fn index_mut(&mut self, index: ops::RangeTo<usize>) -> &mut str {
        &mut self[..][index]
    }
}

impl<A: Allocator> ops::IndexMut<ops::RangeFrom<usize>> for String<A> {
    #[inline]
    fn index_mut(&mut self, index: ops::RangeFrom<usize>) -> &mut str {
        &mut self[..][index]
    }
}

impl<A: Allocator> ops::IndexMut<ops::RangeFull> for String<A> {
    #[inline]
    fn index_mut(&mut self, _index: ops::RangeFull) -> &mut str {
        unsafe { from_utf8_unchecked_mut(&mut self.vec) }
    }
}

impl<A: Allocator> ops::IndexMut<ops::RangeInclusive<usize>> for String<A> {
    #[inline]
    fn index_mut(&mut self, index: ops::RangeInclusive<usize>) -> &mut str {
        IndexMut::index_mut(&mut **self, index)
    }
}

impl<A: Allocator> ops::IndexMut<ops::RangeToInclusive<usize>> for String<A> {
    #[inline]
    fn index_mut(&mut self, index: ops::RangeToInclusive<usize>) -> &mut str {
        IndexMut::index_mut(&mut **self, index)
    }
}

impl<A: Allocator> ops::Deref for String<A> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        unsafe { from_utf8_unchecked(&self.vec) }
    }
}

impl<A: Allocator> ops::DerefMut for String<A> {
    #[inline]
    fn deref_mut(&mut self) -> &mut str {
        unsafe { from_utf8_unchecked_mut(&mut self.vec) }
    }
}

impl<A: Allocator> AsRef<str> for String<A> {
    #[inline]
    fn as_ref(&self) -> &str {
        self
    }
}

impl<A: Allocator> AsMut<str> for String<A> {
    #[inline]
    fn as_mut(&mut self) -> &mut str {
        self
    }
}

#[cfg(feature = "std")]
impl<A: Allocator> AsRef<std::ffi::OsStr> for String<A> {
    #[inline]
    fn as_ref(&self) -> &std::ffi::OsStr {
        (**self).as_ref()
    }
}

impl<A: Allocator> AsRef<[u8]> for String<A> {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<A: Allocator> From<Box<str, A>> for String<A> {
    /// Converts the given boxed `str` slice to a [`String`].
    /// It is notable that the `str` slice is owned.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::{Box, String};
    ///
    /// let s1: String = String::try_from("hello world")?;
    /// let s2: Box<str> = s1.try_into_boxed_str()?;
    /// let s3: String = String::from(s2);
    ///
    /// assert_eq!("hello world", s3);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn from(s: Box<str, A>) -> String<A> {
        crate::str::into_string(s)
    }
}

#[cfg(feature = "alloc")]
impl TryFrom<::rust_alloc::boxed::Box<str>> for String<Global> {
    type Error = Error;

    /// Try to convert a std `Box<str>` into a [`String`].
    ///
    /// The result is fallibly allocated on the heap.
    fn try_from(s: ::rust_alloc::boxed::Box<str>) -> Result<Self, Error> {
        Self::try_from(s.as_ref())
    }
}

#[cfg(feature = "alloc")]
impl TryFrom<::rust_alloc::string::String> for String<Global> {
    type Error = Error;

    /// Try to convert a std `String` into a [`String`].
    ///
    /// The result is fallibly allocated on the heap.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc;
    ///
    /// let s1 = String::from("Hello World");
    /// let s2 = alloc::String::try_from(s1)?;
    ///
    /// assert_eq!("Hello World", s2);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn try_from(string: ::rust_alloc::string::String) -> Result<Self, Error> {
        let mut string = ManuallyDrop::new(string.into_bytes());

        let buf = string.as_mut_ptr();
        let length = string.len();
        let capacity = string.capacity();

        if let Ok(layout) = Layout::array::<u8>(capacity) {
            Global.take(layout)?;
        }

        // SAFETY: The layout of the string is identical to the std string and
        // it uses the same underlying allocator.
        unsafe { Ok(String::from_raw_parts_in(buf, length, capacity, Global)) }
    }
}

#[cfg(feature = "alloc")]
impl<A: Allocator> From<String<A>> for ::rust_alloc::string::String {
    /// Try to convert a [`String`] into a std `String`.
    ///
    /// The result is allocated on the heap.
    fn from(s: String<A>) -> Self {
        Self::from(s.as_str())
    }
}

impl TryFrom<&str> for String<Global> {
    type Error = Error;

    /// Converts a `&str` into a [`String`].
    ///
    /// The result is fallibly allocated on the heap.
    ///
    /// ```
    /// use rune::alloc::String;
    ///
    /// let s = String::try_from("Hello World")?;
    /// assert_eq!(s, "Hello World");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn try_from(s: &str) -> Result<Self, Error> {
        let mut out = String::try_with_capacity_in(s.len(), Global)?;
        out.try_push_str(s)?;
        Ok(out)
    }
}

impl TryFrom<Cow<'_, str>> for String<Global> {
    type Error = Error;

    /// Converts a `Cow<str>` into a [`String`].
    ///
    /// The result is fallibly allocated on the heap unless the values is
    /// `Cow::Owned`.
    ///
    /// ```
    /// use rune::alloc::String;
    /// use rune::alloc::borrow::Cow;
    ///
    /// let s = Cow::Borrowed("Hello World");
    /// let s = String::try_from(s)?;
    /// assert_eq!(s, "Hello World");
    ///
    /// let s = Cow::Owned(String::try_from("Hello World")?);
    /// let s = String::try_from(s)?;
    /// assert_eq!(s, "Hello World");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn try_from(s: Cow<'_, str>) -> Result<Self, Error> {
        match s {
            Cow::Borrowed(s) => Self::try_from(s),
            Cow::Owned(s) => Ok(s),
        }
    }
}

impl<A: Allocator> TryFrom<String<A>> for Box<str, A> {
    type Error = Error;

    /// Converts the given [`String`] to a boxed `str` slice that is owned.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::{String, Box};
    ///
    /// let s1: String = String::try_from("Hello World")?;
    /// let s2: Box<str> = Box::try_from("Hello World")?;
    ///
    /// assert_eq!("Hello World", s2.as_ref());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn try_from(s: String<A>) -> Result<Box<str, A>, Error> {
        s.try_into_boxed_str()
    }
}

impl TryFrom<Cow<'_, str>> for Box<str> {
    type Error = Error;

    /// Converts the given [`String`] to a boxed `str` slice that is owned.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::Box;
    /// use rune::alloc::borrow::Cow;
    ///
    /// let s2: Box<str> = Box::try_from(Cow::Borrowed("Hello World"))?;
    ///
    /// assert_eq!("Hello World", s2.as_ref());
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn try_from(s: Cow<'_, str>) -> Result<Self, Error> {
        Self::try_from(s.as_ref())
    }
}

impl<A: Allocator> From<String<A>> for Vec<u8, A> {
    /// Converts the given [`String`] to a vector [`Vec`] that holds values of type [`u8`].
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::{String, Vec};
    ///
    /// let s1 = String::try_from("hello world")?;
    /// let v1 = Vec::from(s1);
    ///
    /// for b in v1 {
    ///     println!("{b}");
    /// }
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn from(string: String<A>) -> Vec<u8, A> {
        string.into_bytes()
    }
}

/// A draining iterator for `String`.
///
/// This struct is created by the [`drain`] method on [`String`]. See its
/// documentation for more.
///
/// [`drain`]: String::drain
pub struct Drain<'a, A: Allocator> {
    /// Will be used as &'a mut String in the destructor
    string: *mut String<A>,
    /// Start of part to remove
    start: usize,
    /// End of part to remove
    end: usize,
    /// Current remaining range to remove
    iter: Chars<'a>,
}

impl<A: Allocator> fmt::Debug for Drain<'_, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Drain").field(&self.as_str()).finish()
    }
}

unsafe impl<A: Allocator> Sync for Drain<'_, A> {}
unsafe impl<A: Allocator> Send for Drain<'_, A> {}

impl<A: Allocator> Drop for Drain<'_, A> {
    fn drop(&mut self) {
        unsafe {
            // Use Vec::drain. "Reaffirm" the bounds checks to avoid
            // panic code being inserted again.
            let self_vec = (*self.string).as_mut_vec();

            if self.start <= self.end && self.end <= self_vec.len() {
                self_vec.drain(self.start..self.end);
            }
        }
    }
}

impl<'a, A: Allocator> Drain<'a, A> {
    /// Returns the remaining (sub)string of this iterator as a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    ///
    /// let mut s = String::try_from("abc")?;
    /// let mut drain = s.drain(..);
    /// assert_eq!(drain.as_str(), "abc");
    /// assert!(drain.next().is_some());
    /// assert_eq!(drain.as_str(), "bc");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.iter.as_str()
    }
}

impl<'a, A: Allocator> AsRef<str> for Drain<'a, A> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<'a, A: Allocator> AsRef<[u8]> for Drain<'a, A> {
    fn as_ref(&self) -> &[u8] {
        self.as_str().as_bytes()
    }
}

impl<A: Allocator> Iterator for Drain<'_, A> {
    type Item = char;

    #[inline]
    fn next(&mut self) -> Option<char> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    #[inline]
    fn last(mut self) -> Option<char> {
        self.next_back()
    }
}

impl<A: Allocator> DoubleEndedIterator for Drain<'_, A> {
    #[inline]
    fn next_back(&mut self) -> Option<char> {
        self.iter.next_back()
    }
}

impl<A: Allocator> FusedIterator for Drain<'_, A> {}

impl<A: Allocator> TryWrite for String<A> {
    #[inline]
    fn try_write_str(&mut self, s: &str) -> Result<(), Error> {
        self.try_push_str(s)
    }

    #[inline]
    fn try_write_char(&mut self, c: char) -> Result<(), Error> {
        self.try_push(c)
    }
}

impl<A: Allocator> TryFromIteratorIn<char, A> for String<A> {
    /// Construct a string from an iterator of characters.
    ///
    /// ```
    /// use rune::alloc::String;
    /// use rune::alloc::prelude::*;
    ///
    /// let string = String::try_from_iter(['a', 'b', 'c'].into_iter())?;
    /// assert_eq!(string, "abc");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn try_from_iter_in<I>(iter: I, alloc: A) -> Result<Self, Error>
    where
        I: IntoIterator<Item = char>,
    {
        let mut this = String::new_in(alloc);
        this.try_extend(iter)?;
        Ok(this)
    }
}

impl<'a, A: Allocator> TryFromIteratorIn<&'a str, A> for String<A> {
    /// Construct a string from an iterator of characters.
    ///
    /// ```
    /// use rune::alloc::String;
    /// use rune::alloc::prelude::*;
    ///
    /// let string = String::try_from_iter(["hello", " ", "world"].into_iter())?;
    /// assert_eq!(string, "hello world");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn try_from_iter_in<I>(iter: I, alloc: A) -> Result<Self, Error>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut this = String::new_in(alloc);
        this.try_extend(iter)?;
        Ok(this)
    }
}

impl<T, A: Allocator> TryJoin<char, T, A> for String<A>
where
    T: AsRef<str>,
{
    fn try_join_in<I>(iter: I, sep: char, alloc: A) -> Result<Self, Error>
    where
        I: IntoIterator<Item = T>,
    {
        let mut string = String::new_in(alloc);

        let mut iter = iter.into_iter().peekable();

        while let Some(value) = iter.next() {
            string.try_push_str(value.as_ref())?;

            if iter.peek().is_some() {
                string.try_push(sep)?;
            }
        }

        Ok(string)
    }
}

impl<T, A: Allocator> TryJoin<&str, T, A> for String<A>
where
    T: AsRef<str>,
{
    fn try_join_in<I>(iter: I, sep: &str, alloc: A) -> Result<Self, Error>
    where
        I: IntoIterator<Item = T>,
    {
        let mut string = String::new_in(alloc);

        let mut iter = iter.into_iter().peekable();

        while let Some(value) = iter.next() {
            string.try_push_str(value.as_ref())?;

            if iter.peek().is_some() {
                string.try_push_str(sep)?;
            }
        }

        Ok(string)
    }
}

impl<A: Allocator> TryExtend<char> for String<A> {
    /// Extend a string using a character iterator.
    ///
    /// ```
    /// use rune::alloc::String;
    /// use rune::alloc::prelude::*;
    ///
    /// let mut string = String::new();
    /// string.try_extend(['a', 'b', 'c'])?;
    /// assert_eq!(string, "abc");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    fn try_extend<I: IntoIterator<Item = char>>(&mut self, iter: I) -> Result<(), Error> {
        for value in iter {
            self.try_push(value)?;
        }

        Ok(())
    }
}

impl<'a, A: Allocator> TryExtend<&'a str> for String<A> {
    /// Extend a string using a character iterator.
    ///
    /// ```
    /// use rune::alloc::String;
    /// use rune::alloc::prelude::*;
    ///
    /// let mut string = String::new();
    /// string.try_extend(["hello", " ", "world"])?;
    /// assert_eq!(string, "hello world");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    #[inline]
    fn try_extend<I: IntoIterator<Item = &'a str>>(&mut self, iter: I) -> Result<(), Error> {
        for value in iter {
            self.try_push_str(value)?;
        }

        Ok(())
    }
}
