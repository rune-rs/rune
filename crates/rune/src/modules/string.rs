//! The `std::string` module.

use core::char;
use core::cmp::Ordering;
use core::num::{ParseFloatError, ParseIntError};
use core::str::Utf8Error;

use crate as rune;
use crate::alloc::fmt::TryWrite;
use crate::alloc::prelude::*;
use crate::alloc::string::FromUtf8Error;
use crate::alloc::{String, Vec};
use crate::runtime::{Bytes, Formatter, Iterator, Panic, Value, VmErrorKind, VmResult};
use crate::{Any, ContextError, Module};

/// Construct the `std::string` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["string"])?;

    module.ty::<String>()?;

    module.function_meta(string_from)?;
    module
        .function_meta(string_from_str)?
        .deprecated("Use String::from instead")?;
    module.function_meta(string_new)?;
    module.function_meta(string_with_capacity)?;
    module.function_meta(cmp)?;
    module.function_meta(len)?;
    module.function_meta(starts_with)?;
    module.function_meta(ends_with)?;
    module.function_meta(capacity)?;
    module.function_meta(clear)?;
    module.function_meta(contains)?;
    module.function_meta(push)?;
    module.function_meta(push_str)?;
    module.function_meta(reserve)?;
    module.function_meta(reserve_exact)?;
    module.function_meta(from_utf8)?;
    module.function_meta(as_bytes)?;
    module.function_meta(into_bytes)?;
    module.function_meta(clone)?;
    module.function_meta(shrink_to_fit)?;
    module.function_meta(char_at)?;
    module.function_meta(split)?;
    module
        .associated_function("split_str", __rune_fn__split)?
        .deprecated("Use String::split instead")?;
    module.function_meta(trim)?;
    module.function_meta(trim_end)?;
    module.function_meta(replace)?;
    module.function_meta(is_empty)?;
    module.function_meta(chars)?;
    module.function_meta(get)?;
    module.function_meta(parse_int)?;
    module.function_meta(parse_char)?;

    module.function_meta(add)?;
    module.function_meta(add_assign)?;
    module.function_meta(index_get)?;
    Ok(module)
}

#[derive(Any, Debug, Clone, Copy)]
#[rune(module = crate, item = ::std::string, install_with = NotCharBoundary::install)]
struct NotCharBoundary(());

impl NotCharBoundary {
    #[rune::function(instance, protocol = STRING_DISPLAY)]
    fn string_display(&self, f: &mut Formatter) -> VmResult<()> {
        vm_write!(f, "index outside of character boundary");
        VmResult::Ok(())
    }

    fn install(m: &mut Module) -> Result<(), ContextError> {
        m.function_meta(Self::string_display)?;
        Ok(())
    }
}

/// Converts a vector of bytes to a `String`.
///
/// A string ([`String`]) is made of bytes ([`u8`]), and a vector of bytes
/// ([`Vec<u8>`]) is made of bytes, so this function converts between the two.
/// Not all byte slices are valid `String`s, however: `String` requires that it
/// is valid UTF-8. `from_utf8()` checks to ensure that the bytes are valid
/// UTF-8, and then does the conversion.
///
/// If you are sure that the byte slice is valid UTF-8, and you don't want to
/// incur the overhead of the validity check, there is an unsafe version of this
/// function, [`from_utf8_unchecked`], which has the same behavior but skips the
/// check.
///
/// The inverse of this method is [`into_bytes`].
///
/// # Errors
///
/// Returns [`Err`] if the slice is not UTF-8 with a description as to why the
/// provided bytes are not UTF-8. The vector you moved in is also included.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// // some bytes, in a vector
/// let sparkle_heart = Bytes::from_vec([240u8, 159u8, 146u8, 150u8]);
///
/// // We know these bytes are valid, so we'll use `unwrap()`.
/// let sparkle_heart = String::from_utf8(sparkle_heart).unwrap();
///
/// assert_eq!("💖", sparkle_heart);
/// ```
///
/// Incorrect bytes:
///
/// ```rune
/// // some invalid bytes, in a vector
/// let sparkle_heart = Bytes::from_vec([0u8, 159u8, 146u8, 150u8]);
///
/// assert!(String::from_utf8(sparkle_heart).is_err());
/// ```
///
/// See the docs for [`FromUtf8Error`] for more details on what you can do with
/// this error.
///
/// [`from_utf8_unchecked`]: String::from_utf8_unchecked
/// [`Vec<u8>`]: crate::vec::Vec "Vec"
/// [`&str`]: prim@str "&str"
/// [`into_bytes`]: String::into_bytes
#[rune::function(free, path = String::from_utf8)]
fn from_utf8(bytes: &[u8]) -> VmResult<Result<String, FromUtf8Error>> {
    let vec = vm_try!(Vec::try_from(bytes));
    VmResult::Ok(String::from_utf8(vec))
}

/// Returns a byte slice of this `String`'s contents.
///
/// The inverse of this method is [`from_utf8`].
///
/// [`from_utf8`]: String::from_utf8
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let s = "hello";
///
/// assert_eq!(b"hello", s.as_bytes());
/// assert!(is_readable(s));
/// ```
#[rune::function(instance)]
fn as_bytes(s: &str) -> VmResult<Bytes> {
    VmResult::Ok(Bytes::from_vec(vm_try!(Vec::try_from(s.as_bytes()))))
}

/// Constructs a string from another string.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let s = String::from("hello");
/// assert_eq!(s, "hello");
/// ```
#[rune::function(free, path = String::from)]
fn string_from(value: &str) -> VmResult<String> {
    VmResult::Ok(vm_try!(String::try_from(value)))
}

#[rune::function(free, path = String::from_str)]
fn string_from_str(value: &str) -> VmResult<String> {
    VmResult::Ok(vm_try!(String::try_from(value)))
}

/// Creates a new empty `String`.
///
/// Given that the `String` is empty, this will not allocate any initial buffer.
/// While that means that this initial operation is very inexpensive, it may
/// cause excessive allocation later when you add data. If you have an idea of
/// how much data the `String` will hold, consider the [`with_capacity`] method
/// to prevent excessive re-allocation.
///
/// [`with_capacity`]: String::with_capacity
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let s = String::new();
/// ```
#[rune::function(free, path = String::new)]
fn string_new() -> String {
    String::new()
}

/// Creates a new empty `String` with at least the specified capacity.
///
/// `String`s have an internal buffer to hold their data. The capacity is the
/// length of that buffer, and can be queried with the [`capacity`] method. This
/// method creates an empty `String`, but one with an initial buffer that can
/// hold at least `capacity` bytes. This is useful when you may be appending a
/// bunch of data to the `String`, reducing the number of reallocations it needs
/// to do.
///
/// [`capacity`]: String::capacity
///
/// If the given capacity is `0`, no allocation will occur, and this method is
/// identical to the [`new`] method.
///
/// [`new`]: String::new
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let s = String::with_capacity(10);
///
/// // The String contains no chars, even though it has capacity for more
/// assert_eq!(s.len(), 0);
///
/// // These are all done without reallocating...
/// let cap = s.capacity();
///
/// for _ in 0..10 {
///     s.push('a');
/// }
///
/// assert_eq!(s.capacity(), cap);
///
/// // ...but this may make the string reallocate
/// s.push('a');
/// ```
#[rune::function(free, path = String::with_capacity)]
fn string_with_capacity(capacity: usize) -> VmResult<String> {
    VmResult::Ok(vm_try!(String::try_with_capacity(capacity)))
}

#[rune::function(instance)]
fn cmp(lhs: &str, rhs: &str) -> Ordering {
    lhs.cmp(rhs)
}

/// Returns the length of `self`.
///
/// This length is in bytes, not [`char`]s or graphemes. In other words, it
/// might not be what a human considers the length of the string.
///
/// [`char`]: prim@char
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let len = "foo".len();
/// assert_eq!(3, len);
///
/// assert_eq!("ƒoo".len(), 4); // fancy f!
/// assert_eq!("ƒoo".chars().count(), 3);
/// ```
#[rune::function(instance)]
fn len(this: &str) -> usize {
    this.len()
}

/// Returns `true` if the given pattern matches a prefix of this string slice.
///
/// Returns `false` if it does not.
///
/// The [pattern] can be a `&str`, [`char`], a slice of [`char`]s, or a function
/// or closure that determines if a character matches.
///
/// [`char`]: prim@char
/// [pattern]: self::pattern
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let bananas = "bananas";
///
/// assert!(bananas.starts_with("bana"));
/// assert!(!bananas.starts_with("nana"));
/// ```
#[rune::function(instance)]
fn starts_with(this: &str, other: &str) -> bool {
    this.starts_with(other)
}

/// Returns `true` if the given pattern matches a suffix of this string slice.
///
/// Returns `false` if it does not.
///
/// The [pattern] can be a `&str`, [`char`], a slice of [`char`]s, or a function
/// or closure that determines if a character matches.
///
/// [`char`]: prim@char
/// [pattern]: self::pattern
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let bananas = "bananas";
///
/// assert!(bananas.ends_with("anas"));
/// assert!(!bananas.ends_with("nana"));
/// ```
#[rune::function(instance)]
fn ends_with(this: &str, other: &str) -> bool {
    this.ends_with(other)
}

/// Returns this `String`'s capacity, in bytes.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let s = String::with_capacity(10);
///
/// assert!(s.capacity() >= 10);
/// ```
#[rune::function(instance)]
fn capacity(this: &String) -> usize {
    this.capacity()
}

/// Truncates this `String`, removing all contents.
///
/// While this means the `String` will have a length of zero, it does not touch
/// its capacity.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let s = "foo";
///
/// s.clear();
///
/// assert!(s.is_empty());
/// assert_eq!(0, s.len());
/// assert_eq!(3, s.capacity());
/// ```
#[rune::function(instance)]
fn clear(this: &mut String) {
    this.clear();
}

/// Returns `true` if the given pattern matches a sub-slice of this string
/// slice.
///
/// Returns `false` if it does not.
///
/// The [pattern] can be a `String`, [`char`], or a function or closure that
/// determines if a character matches.
///
/// [`char`]: prim@char
/// [pattern]: self::pattern
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let bananas = "bananas";
///
/// assert!(bananas.contains("nana"));
/// assert!(!bananas.contains("apples"));
/// ```
#[rune::function(instance)]
fn contains(this: &str, other: &str) -> bool {
    this.contains(other)
}

/// Appends the given [`char`] to the end of this `String`.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let s = "abc";
///
/// s.push('1');
/// s.push('2');
/// s.push('3');
///
/// assert_eq!("abc123", s);
/// ```
#[rune::function(instance)]
fn push(this: &mut String, c: char) -> VmResult<()> {
    vm_try!(this.try_push(c));
    VmResult::Ok(())
}

/// Appends a given string slice onto the end of this `String`.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let s = "foo";
///
/// s.push_str("bar");
///
/// assert_eq!("foobar", s);
/// ```
#[rune::function(instance)]
fn push_str(this: &mut String, other: &str) -> VmResult<()> {
    vm_try!(this.try_push_str(other));
    VmResult::Ok(())
}

/// Reserves capacity for at least `additional` bytes more than the current
/// length. The allocator may reserve more space to speculatively avoid frequent
/// allocations. After calling `reserve`, capacity will be greater than or equal
/// to `self.len() + additional`. Does nothing if capacity is already
/// sufficient.
///
/// # Panics
///
/// Panics if the new capacity overflows [`usize`].
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let s = String::new();
///
/// s.reserve(10);
///
/// assert!(s.capacity() >= 10);
/// ```
///
/// This might not actually increase the capacity:
///
/// ```rune
/// let s = String::with_capacity(10);
/// s.push('a');
/// s.push('b');
///
/// // s now has a length of 2 and a capacity of at least 10
/// let capacity = s.capacity();
/// assert_eq!(2, s.len());
/// assert!(capacity >= 10);
///
/// // Since we already have at least an extra 8 capacity, calling this...
/// s.reserve(8);
///
/// // ... doesn't actually increase.
/// assert_eq!(capacity, s.capacity());
/// ```
#[rune::function(instance)]
fn reserve(this: &mut String, additional: usize) -> VmResult<()> {
    vm_try!(this.try_reserve(additional));
    VmResult::Ok(())
}

/// Reserves the minimum capacity for at least `additional` bytes more than the
/// current length. Unlike [`reserve`], this will not deliberately over-allocate
/// to speculatively avoid frequent allocations. After calling `reserve_exact`,
/// capacity will be greater than or equal to `self.len() + additional`. Does
/// nothing if the capacity is already sufficient.
///
/// [`reserve`]: String::reserve
///
/// # Panics
///
/// Panics if the new capacity overflows [`usize`].
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let s = String::new();
///
/// s.reserve_exact(10);
///
/// assert!(s.capacity() >= 10);
/// ```
///
/// This might not actually increase the capacity:
///
/// ```rune
/// let s = String::with_capacity(10);
/// s.push('a');
/// s.push('b');
///
/// // s now has a length of 2 and a capacity of at least 10
/// let capacity = s.capacity();
/// assert_eq!(2, s.len());
/// assert!(capacity >= 10);
///
/// // Since we already have at least an extra 8 capacity, calling this...
/// s.reserve_exact(8);
///
/// // ... doesn't actually increase.
/// assert_eq!(capacity, s.capacity());
/// ```
#[rune::function(instance)]
fn reserve_exact(this: &mut String, additional: usize) -> VmResult<()> {
    vm_try!(this.try_reserve_exact(additional));
    VmResult::Ok(())
}

/// Returns a byte slice of this `String`'s contents while moving the string.
///
/// The inverse of this method is [`from_utf8`].
///
/// [`from_utf8`]: String::from_utf8
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let s = "hello";
///
/// assert_eq!(b"hello", s.into_bytes());
/// assert!(!is_readable(s));
/// ```
#[rune::function(instance)]
fn into_bytes(s: String) -> Bytes {
    Bytes::from_vec(s.into_bytes())
}

/// Checks that `index`-th byte is the first byte in a UTF-8 code point sequence
/// or the end of the string.
///
/// The start and end of the string (when `index == self.len()`) are considered
/// to be boundaries.
///
/// Returns `false` if `index` is greater than `self.len()`.
///
/// # Examples
///
/// ```rune
/// let s = "Löwe 老虎 Léopard";
/// assert!(s.is_char_boundary(0));
/// // start of `老`
/// assert!(s.is_char_boundary(6));
/// assert!(s.is_char_boundary(s.len()));
///
/// // second byte of `ö`
/// assert!(!s.is_char_boundary(2));
///
/// // third byte of `老`
/// assert!(!s.is_char_boundary(8));
/// ```
#[rune::function(instance)]
fn is_char_boundary(s: &str, index: usize) -> bool {
    s.is_char_boundary(index)
}

/// Access the character at the given byte index.
///
/// Returns `None` if the index is out of bounds or not a character boundary.
///
/// # Examples
///
/// ```rune
/// let s = "おはよう";
/// assert_eq!(s.char_at(0), Some('お'));
/// assert_eq!(s.char_at(1), None);
/// assert_eq!(s.char_at(2), None);
/// assert_eq!(s.char_at(3), Some('は'));
/// ```
#[rune::function(instance)]
fn char_at(s: &str, index: usize) -> Option<char> {
    if !s.is_char_boundary(index) {
        return None;
    }

    s[index..].chars().next()
}

/// Clones the string and its underlying storage.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let a = "h";
/// let b = a;
/// b.push('i');
///
/// // `a` and `b` refer to the same underlying string.
/// assert_eq!(a, b);
///
/// let c = b.clone();
/// c.push('!');
/// assert_ne!(a, c);
/// ```
#[rune::function(instance)]
#[allow(clippy::ptr_arg)]
fn clone(s: &String) -> VmResult<String> {
    VmResult::Ok(vm_try!(s.try_clone()))
}

/// Shrinks the capacity of this `String` to match its length.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let s = "foo";
///
/// s.reserve(100);
/// assert!(s.capacity() >= 100);
///
/// s.shrink_to_fit();
/// assert_eq!(3, s.capacity());
/// ```
#[rune::function(instance)]
fn shrink_to_fit(s: &mut String) -> VmResult<()> {
    vm_try!(s.try_shrink_to_fit());
    VmResult::Ok(())
}

/// An iterator over substrings of this string slice, separated by
/// characters matched by a pattern.
///
/// The [pattern] can be a `&str`, [`char`], a slice of [`char`]s, or a
/// function or closure that determines if a character matches.
///
/// [`char`]: prim@char
/// [pattern]: self::pattern
///
/// # Iterator behavior
///
/// The returned iterator will be a [`DoubleEndedIterator`] if the pattern
/// allows a reverse search and forward/reverse search yields the same
/// elements. This is true for, e.g., [`char`], but not for `&str`.
///
/// If the pattern allows a reverse search but its results might differ
/// from a forward search, the [`rsplit`] method can be used.
///
/// [`rsplit`]: str::rsplit
///
/// # Examples
///
/// Simple patterns:
///
/// ```rune
/// let v = "Mary had a little lamb".split(' ').collect::<Vec>();
/// assert_eq!(v, ["Mary", "had", "a", "little", "lamb"]);
///
/// let v = "".split('X').collect::<Vec>();
/// assert_eq!(v, [""]);
///
/// let v = "lionXXtigerXleopard".split('X').collect::<Vec>();
/// assert_eq!(v, ["lion", "", "tiger", "leopard"]);
///
/// let v = "lion::tiger::leopard".split("::").collect::<Vec>();
/// assert_eq!(v, ["lion", "tiger", "leopard"]);
///
/// let v = "abc1def2ghi".split(char::is_numeric).collect::<Vec>();
/// assert_eq!(v, ["abc", "def", "ghi"]);
///
/// let v = "lionXtigerXleopard".split(char::is_uppercase).collect::<Vec>();
/// assert_eq!(v, ["lion", "tiger", "leopard"]);
/// ```
///
/// A more complex pattern, using a closure:
///
/// ```rune
/// let v = "abc1defXghi".split(|c| c == '1' || c == 'X').collect::<Vec>();
/// assert_eq!(v, ["abc", "def", "ghi"]);
/// ```
///
/// If a string contains multiple contiguous separators, you will end up
/// with empty strings in the output:
///
/// ```rune
/// let x = "||||a||b|c";
/// let d = x.split('|').collect::<Vec>();
///
/// assert_eq!(d, ["", "", "", "", "a", "", "b", "c"]);
/// ```
///
/// Contiguous separators are separated by the empty string.
///
/// ```rune
/// let x = "(///)";
/// let d = x.split('/').collect::<Vec>();
///
/// assert_eq!(d, ["(", "", "", ")"]);
/// ```
///
/// Separators at the start or end of a string are neighbored
/// by empty strings.
///
/// ```rune
/// let d = "010".split("0").collect::<Vec>();
/// assert_eq!(d, ["", "1", ""]);
/// ```
///
/// When the empty string is used as a separator, it separates
/// every character in the string, along with the beginning
/// and end of the string.
///
/// ```rune
/// let f = "rust".split("").collect::<Vec>();
/// assert_eq!(f, ["", "r", "u", "s", "t", ""]);
/// ```
///
/// Contiguous separators can lead to possibly surprising behavior
/// when whitespace is used as the separator. This code is correct:
///
/// ```rune
/// let x = "    a  b c";
/// let d = x.split(' ').collect::<Vec>();
///
/// assert_eq!(d, ["", "", "", "", "a", "", "b", "c"]);
/// ```
///
/// It does _not_ give you:
///
/// ```rune,ignore
/// assert_eq!(d, ["a", "b", "c"]);
/// ```
///
/// Use [`split_whitespace`] for this behavior.
///
/// [`split_whitespace`]: str::split_whitespace
#[rune::function(instance)]
fn split(this: &str, value: Value) -> VmResult<Iterator> {
    const NAME: &str = "std::str::Split";

    let lines = match value {
        Value::String(s) => {
            let mut out = Vec::new();

            for value in this.split(vm_try!(s.borrow_ref()).as_str()) {
                let value = vm_try!(String::try_from(value));
                vm_try!(out.try_push(value));
            }

            out
        }
        Value::Char(pat) => {
            let mut out = Vec::new();

            for value in this.split(pat) {
                let value = vm_try!(String::try_from(value));
                vm_try!(out.try_push(value));
            }

            out
        }
        Value::Function(f) => {
            let f = vm_try!(f.borrow_ref());
            let mut err = None;

            let iter = this.split(|c: char| match f.call::<_, bool>((c,)) {
                VmResult::Ok(b) => b,
                VmResult::Err(e) => {
                    if err.is_none() {
                        err = Some(e);
                    }

                    false
                }
            });

            let mut out = Vec::new();

            for value in iter {
                let value = vm_try!(String::try_from(value));
                vm_try!(out.try_push(value));
            }

            if let Some(e) = err.take() {
                return VmResult::Err(e);
            }

            out
        }
        actual => {
            return VmResult::err([
                VmErrorKind::expected::<String>(vm_try!(actual.type_info())),
                VmErrorKind::bad_argument(0),
            ])
        }
    };

    VmResult::Ok(Iterator::from_double_ended(NAME, lines.into_iter()))
}

/// Returns a string slice with leading and trailing whitespace removed.
///
/// 'Whitespace' is defined according to the terms of the Unicode Derived Core
/// Property `White_Space`, which includes newlines.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let s = "\n Hello\tworld\t\n";
///
/// assert_eq!("Hello\tworld", s.trim());
/// ```
#[rune::function(instance)]
fn trim(this: &str) -> VmResult<String> {
    VmResult::Ok(vm_try!(this.trim().try_to_owned()))
}

/// Returns a string slice with trailing whitespace removed.
///
/// 'Whitespace' is defined according to the terms of the Unicode Derived Core
/// Property `White_Space`, which includes newlines.
///
/// # Text directionality
///
/// A string is a sequence of bytes. `end` in this context means the last
/// position of that byte string; for a left-to-right language like English or
/// Russian, this will be right side, and for right-to-left languages like
/// Arabic or Hebrew, this will be the left side.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let s = "\n Hello\tworld\t\n";
/// assert_eq!("\n Hello\tworld", s.trim_end());
/// ```
///
/// Directionality:
///
/// ```rune
/// let s = "  English  ";
/// assert!(Some('h') == s.trim_end().chars().rev().next());
///
/// let s = "  עברית  ";
/// assert!(Some('ת') == s.trim_end().chars().rev().next());
/// ```
#[rune::function(instance)]
fn trim_end(this: &str) -> VmResult<String> {
    VmResult::Ok(vm_try!(this.trim_end().try_to_owned()))
}

/// Returns `true` if `self` has a length of zero bytes.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let s = "";
/// assert!(s.is_empty());
///
/// let s = "not empty";
/// assert!(!s.is_empty());
/// ```
#[rune::function(instance)]
fn is_empty(this: &str) -> bool {
    this.is_empty()
}

/// Replaces all matches of a pattern with another string.
///
/// `replace` creates a new [`String`], and copies the data from this string
/// slice into it. While doing so, it attempts to find matches of a pattern. If
/// it finds any, it replaces them with the replacement string slice.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let s = "this is old";
///
/// assert_eq!("this is new", s.replace("old", "new"));
/// assert_eq!("than an old", s.replace("is", "an"));
/// ```
///
/// When the pattern doesn't match, it returns this string slice as [`String`]:
///
/// ```rune
/// let s = "this is old";
/// assert_eq!(s, s.replace("cookie monster", "little lamb"));
/// ```
#[rune::function(instance)]
fn replace(a: &str, from: &str, to: &str) -> VmResult<String> {
    VmResult::Ok(vm_try!(String::try_from(a.replace(from, to))))
}

/// Returns an iterator over the [`char`]s of a string slice.
///
/// As a string slice consists of valid UTF-8, we can iterate through a string
/// slice by [`char`]. This method returns such an iterator.
///
/// It's important to remember that [`char`] represents a Unicode Scalar Value,
/// and might not match your idea of what a 'character' is. Iteration over
/// grapheme clusters may be what you actually want. This functionality is not
/// provided by Rust's standard library, check crates.io instead.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let word = "goodbye";
///
/// let count = word.chars().count();
/// assert_eq!(7, count);
///
/// let chars = word.chars();
///
/// assert_eq!(Some('g'), chars.next());
/// assert_eq!(Some('o'), chars.next());
/// assert_eq!(Some('o'), chars.next());
/// assert_eq!(Some('d'), chars.next());
/// assert_eq!(Some('b'), chars.next());
/// assert_eq!(Some('y'), chars.next());
/// assert_eq!(Some('e'), chars.next());
///
/// assert_eq!(None, chars.next());
/// ```
///
/// Remember, [`char`]s might not match your intuition about characters:
///
/// [`char`]: prim@char
///
/// ```rune
/// let y = "y̆";
///
/// let chars = y.chars();
///
/// assert_eq!(Some('y'), chars.next()); // not 'y̆'
/// assert_eq!(Some('\u{0306}'), chars.next());
///
/// assert_eq!(None, chars.next());
/// ```
#[rune::function(instance)]
fn chars(s: &str) -> VmResult<Iterator> {
    // TODO: perform lazy iteration.
    let iter = vm_try!(s.chars().try_collect::<Vec<_>>()).into_iter();
    VmResult::Ok(Iterator::from_double_ended("std::str::Chars", iter))
}

/// Returns a subslice of `str`.
///
/// This is the non-panicking alternative to indexing the `str`. Returns
/// [`None`] whenever equivalent indexing operation would panic.
///
/// # Examples
///
/// ```rune
/// let v = "🗻∈🌏";
///
/// assert_eq!(Some("🗻"), v.get(0..4));
///
/// // indices not on UTF-8 sequence boundaries
/// assert!(v.get(1..).is_none());
/// assert!(v.get(..8).is_none());
///
/// // out of bounds
/// assert!(v.get(..42).is_none());
/// ```
#[rune::function(instance)]
fn get(this: &str, key: Value) -> VmResult<Option<String>> {
    use crate::runtime::TypeOf;

    let slice = match key {
        Value::RangeFrom(range) => {
            let range = vm_try!(range.borrow_ref());
            let start = vm_try!(range.start.as_usize());
            this.get(start..)
        }
        Value::RangeFull(..) => this.get(..),
        Value::RangeInclusive(range) => {
            let range = vm_try!(range.borrow_ref());
            let start = vm_try!(range.start.as_usize());
            let end = vm_try!(range.end.as_usize());
            this.get(start..=end)
        }
        Value::RangeToInclusive(range) => {
            let range = vm_try!(range.borrow_ref());
            let end = vm_try!(range.end.as_usize());
            this.get(..=end)
        }
        Value::RangeTo(range) => {
            let range = vm_try!(range.borrow_ref());
            let end = vm_try!(range.end.as_usize());
            this.get(..end)
        }
        Value::Range(range) => {
            let range = vm_try!(range.borrow_ref());
            let start = vm_try!(range.start.as_usize());
            let end = vm_try!(range.end.as_usize());
            this.get(start..end)
        }
        index => {
            return VmResult::err(VmErrorKind::UnsupportedIndexGet {
                target: String::type_info(),
                index: vm_try!(index.type_info()),
            })
        }
    };

    let Some(slice) = slice else {
        return VmResult::Ok(None);
    };

    VmResult::Ok(Some(vm_try!(slice.try_to_owned())))
}

/// The add operation for strings.
#[rune::function(instance, protocol = ADD)]
fn add(a: &str, b: &str) -> VmResult<String> {
    let mut string = vm_try!(String::try_with_capacity(a.len() + b.len()));
    vm_try!(string.try_push_str(a));
    vm_try!(string.try_push_str(b));
    VmResult::Ok(string)
}

/// The add assign operation for strings.
#[rune::function(instance, protocol = ADD_ASSIGN)]
fn add_assign(this: &mut String, other: &str) -> VmResult<()> {
    vm_try!(this.try_push_str(other));
    VmResult::Ok(())
}

/// Get a specific string index.
#[rune::function(instance, protocol = INDEX_GET)]
fn index_get(s: &str, key: Value) -> VmResult<String> {
    match vm_try!(__rune_fn__get(s, key)) {
        Some(slice) => VmResult::Ok(slice),
        None => VmResult::err(Panic::custom("missing string slice")),
    }
}

/// Parses this string into an integer.
///
/// # Errors
///
/// Will return [`Err`] if it's not possible to parse this string slice into an
/// integer.
///
/// # Examples
///
/// Basic usage
///
/// ```rune
/// let four = "4".parse::<i64>()?;
/// assert_eq!(4, four);
/// ```
#[rune::function(instance, path = parse::<i64>)]
fn parse_int(s: &str) -> Result<i64, ParseIntError> {
    str::parse::<i64>(s)
}

/// Parses this string into a float.
///
/// # Errors
///
/// Will return [`Err`] if it's not possible to parse this string slice into an
/// float.
///
/// # Examples
///
/// Basic usage
///
/// ```rune
/// let pi = "3.1415".parse::<f64>()?;
/// assert_eq!(3.1415, pi);
/// ```
#[rune::function(instance, path = parse::<f64>)]
fn parse_float(s: &str) -> Result<f64, ParseFloatError> {
    str::parse::<f64>(s)
}

/// Parses this string into a character.
///
/// # Errors
///
/// Will return [`Err`] if it's not possible to parse this string slice into an
/// integer.
///
/// # Examples
///
/// Basic usage
///
/// ```rune
/// let a = "a".parse::<char>()?;
/// assert_eq!('a', a);
/// ```
#[rune::function(instance, path = parse::<char>)]
fn parse_char(s: &str) -> Result<char, char::ParseCharError> {
    str::parse::<char>(s)
}

crate::__internal_impl_any!(::std::string, FromUtf8Error);
crate::__internal_impl_any!(::std::string, Utf8Error);
