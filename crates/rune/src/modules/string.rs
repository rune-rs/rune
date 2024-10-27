//! Strings.

use core::char;
use core::cmp::Ordering;
use core::num::{ParseFloatError, ParseIntError};
use core::str::Utf8Error;

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::string::FromUtf8Error;
use crate::alloc::{String, Vec};
use crate::compile::Named;
use crate::runtime::{
    Bytes, FromValue, Function, Inline, MaybeTypeOf, Mutable, Panic, Range, RangeFrom, RangeFull,
    RangeInclusive, RangeTo, RangeToInclusive, Ref, ToValue, TypeOf, Value, ValueBorrowRef,
    VmErrorKind, VmResult,
};
use crate::{Any, ContextError, Module, TypeHash};

/// Strings.
///
/// Strings in Rune are declared with the literal `"string"` syntax, but can also be
/// interacted with through the fundamental [`String`] type.
///
/// ```rune
/// let string1 = "Hello";
/// let string2 = String::new();
/// string2.push_str("Hello");
///
/// assert_eq!(string1, string2);
/// ```
#[rune::module(::std::string)]
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module_meta)?;

    m.ty::<String>()?;

    m.function_meta(string_from)?;
    m.function_meta(string_from_str)?;
    m.function_meta(string_new)?;
    m.function_meta(string_with_capacity)?;
    m.function_meta(cmp)?;
    m.function_meta(len)?;
    m.function_meta(starts_with)?;
    m.function_meta(ends_with)?;
    m.function_meta(capacity)?;
    m.function_meta(clear)?;
    m.function_meta(contains)?;
    m.function_meta(push)?;
    m.function_meta(push_str)?;
    m.function_meta(reserve)?;
    m.function_meta(reserve_exact)?;
    m.function_meta(from_utf8)?;
    m.function_meta(as_bytes)?;
    m.function_meta(into_bytes)?;
    m.function_meta(shrink_to_fit)?;
    m.function_meta(char_at)?;
    m.function_meta(split)?;
    m.function_meta(split_once)?;
    m.associated_function("split_str", __rune_fn__split)?;
    m.function_meta(trim)?;
    m.function_meta(trim_end)?;
    m.function_meta(replace)?;
    m.function_meta(is_empty)?;
    m.function_meta(chars)?;
    m.function_meta(get)?;
    m.function_meta(parse_int)?;
    m.function_meta(parse_char)?;
    m.function_meta(to_lowercase)?;
    m.function_meta(to_uppercase)?;

    m.function_meta(add)?;
    m.function_meta(add_assign)?;
    m.function_meta(index_get)?;

    m.function_meta(clone__meta)?;
    m.implement_trait::<String>(rune::item!(::std::clone::Clone))?;

    m.ty::<Chars>()?;
    m.function_meta(Chars::next__meta)?;
    m.function_meta(Chars::next_back__meta)?;
    m.implement_trait::<Chars>(rune::item!(::std::iter::Iterator))?;
    m.implement_trait::<Chars>(rune::item!(::std::iter::DoubleEndedIterator))?;

    macro_rules! split {
        ($ty:ty) => {
            m.ty::<Split<$ty>>()?;
            m.function_meta(Split::<$ty>::next__meta)?;
            m.implement_trait::<Split<$ty>>(rune::item!(::std::iter::Iterator))?;
        };
    }

    split!(Function);
    split!(Box<str>);
    split!(char);
    Ok(m)
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

#[rune::function(free, path = String::from_str, deprecated = "Use String::from instead")]
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
/// assert_eq!(b"hello", s.into_bytes());
/// assert!(is_readable(s));
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
#[rune::function(keep, instance, protocol = CLONE)]
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
#[rune::function(instance, deprecated = "Use String::split instead")]
fn split(this: Ref<str>, value: Value) -> VmResult<Value> {
    let split = match vm_try!(value.borrow_ref()) {
        ValueBorrowRef::Inline(Inline::Char(c)) => {
            vm_try!(rune::to_value(Split::new(this, *c)))
        }
        ValueBorrowRef::Mutable(value) => match &*value {
            Mutable::String(ref s) => {
                vm_try!(rune::to_value(Split::new(
                    this,
                    vm_try!(Box::try_from(s.as_str()))
                )))
            }
            Mutable::Function(ref f) => {
                vm_try!(rune::to_value(Split::new(this, vm_try!(f.try_clone()))))
            }
            actual => {
                return VmResult::err([
                    VmErrorKind::expected::<String>(actual.type_info()),
                    VmErrorKind::bad_argument(0),
                ])
            }
        },
        actual => {
            return VmResult::err([
                VmErrorKind::expected::<String>(actual.type_info()),
                VmErrorKind::bad_argument(0),
            ])
        }
    };

    VmResult::Ok(split)
}

/// Splits the string on the first occurrence of the specified delimiter and
/// returns prefix before delimiter and suffix after delimiter.
///
/// # Examples
///
/// ```rune
/// assert_eq!("cfg".split_once('='), None);
/// assert_eq!("cfg=".split_once('='), Some(("cfg", "")));
/// assert_eq!("cfg=foo".split_once('='), Some(("cfg", "foo")));
/// assert_eq!("cfg=foo=bar".split_once('='), Some(("cfg", "foo=bar")));
/// ```
#[rune::function(instance)]
fn split_once(this: &str, value: Value) -> VmResult<Option<(String, String)>> {
    let outcome = match vm_try!(value.borrow_ref()) {
        ValueBorrowRef::Inline(Inline::Char(pat)) => this.split_once(*pat),
        ValueBorrowRef::Mutable(value) => match &*value {
            Mutable::String(s) => this.split_once(s.as_str()),
            Mutable::Function(f) => {
                let mut err = None;

                let outcome = this.split_once(|c: char| match f.call::<bool>((c,)) {
                    VmResult::Ok(b) => b,
                    VmResult::Err(e) => {
                        if err.is_none() {
                            err = Some(e);
                        }

                        false
                    }
                });

                if let Some(e) = err.take() {
                    return VmResult::Err(e);
                }

                outcome
            }
            actual => {
                return VmResult::err([
                    VmErrorKind::expected::<String>(actual.type_info()),
                    VmErrorKind::bad_argument(0),
                ])
            }
        },
        ref actual => {
            return VmResult::err([
                VmErrorKind::expected::<String>(actual.type_info()),
                VmErrorKind::bad_argument(0),
            ])
        }
    };

    let Some((a, b)) = outcome else {
        return VmResult::Ok(None);
    };

    VmResult::Ok(Some((vm_try!(a.try_to_owned()), vm_try!(b.try_to_owned()))))
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
fn chars(s: Ref<str>) -> Chars {
    Chars::new(s)
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

    let slice = match vm_try!(key.as_any()) {
        Some(value) => match value.type_hash() {
            RangeFrom::HASH => {
                let range = vm_try!(value.borrow_ref::<RangeFrom>());
                let start = vm_try!(range.start.as_usize());
                this.get(start..)
            }
            RangeFull::HASH => {
                _ = vm_try!(value.borrow_ref::<RangeFull>());
                this.get(..)
            }
            RangeInclusive::HASH => {
                let range = vm_try!(value.borrow_ref::<RangeInclusive>());
                let start = vm_try!(range.start.as_usize());
                let end = vm_try!(range.end.as_usize());
                this.get(start..=end)
            }
            RangeToInclusive::HASH => {
                let range = vm_try!(value.borrow_ref::<RangeToInclusive>());
                let end = vm_try!(range.end.as_usize());
                this.get(..=end)
            }
            RangeTo::HASH => {
                let range = vm_try!(value.borrow_ref::<RangeTo>());
                let end = vm_try!(range.end.as_usize());
                this.get(..end)
            }
            Range::HASH => {
                let range = vm_try!(value.borrow_ref::<Range>());
                let start = vm_try!(range.start.as_usize());
                let end = vm_try!(range.end.as_usize());
                this.get(start..end)
            }
            _ => {
                return VmResult::err(VmErrorKind::UnsupportedIndexGet {
                    target: String::type_info(),
                    index: value.type_info(),
                })
            }
        },
        _ => {
            return VmResult::err(VmErrorKind::UnsupportedIndexGet {
                target: String::type_info(),
                index: vm_try!(key.type_info()),
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

/// Returns the lowercase equivalent of this string slice, as a new [`String`].
///
/// 'Lowercase' is defined according to the terms of the Unicode Derived Core Property
/// `Lowercase`.
///
/// Since some characters can expand into multiple characters when changing
/// the case, this function returns a [`String`] instead of modifying the
/// parameter in-place.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let s = "HELLO";
///
/// assert_eq!("hello", s.to_lowercase());
/// ```
///
/// A tricky example, with sigma:
///
/// ```rune
/// let sigma = "Σ";
///
/// assert_eq!("σ", sigma.to_lowercase());
///
/// // but at the end of a word, it's ς, not σ:
/// let odysseus = "ὈΔΥΣΣΕΎΣ";
///
/// assert_eq!("ὀδυσσεύς", odysseus.to_lowercase());
/// ```
///
/// Languages without case are not changed:
///
/// ```rune
/// let new_year = "农历新年";
///
/// assert_eq!(new_year, new_year.to_lowercase());
/// ```
#[rune::function(instance)]
fn to_lowercase(s: &str) -> VmResult<String> {
    let mut lowercase = vm_try!(String::try_with_capacity(s.len()));
    for (i, c) in s.char_indices() {
        // Inlined code to from std::str to handle upper-case sigma,
        // since it is the only Unicode character that is context-dependent
        // See https://github.com/rust-lang/rust/issues/26035 for more context
        if c == 'Σ' {
            vm_try!(lowercase.try_push_str(map_uppercase_sigma(s, i)));
        } else {
            vm_try!(lowercase.try_extend(c.to_lowercase()));
        }
    }

    return VmResult::Ok(lowercase);

    fn map_uppercase_sigma(from: &str, i: usize) -> &'static str {
        // See https://www.unicode.org/versions/Unicode7.0.0/ch03.pdf#G33992
        // for the definition of `Final_Sigma`.
        debug_assert!('Σ'.len_utf8() == 2);
        let is_word_final = case_ignorable_then_cased(from[..i].chars().rev())
            && !case_ignorable_then_cased(from[i + 2..].chars());
        if is_word_final {
            "ς"
        } else {
            "σ"
        }
    }

    fn case_ignorable_then_cased<I: core::iter::Iterator<Item = char>>(mut iter: I) -> bool {
        match iter.find(|&c| !unicode::case_ignorable::lookup(c)) {
            Some(c) => unicode::cased::lookup(c),
            None => false,
        }
    }
}

/// Returns the uppercase equivalent of this string slice, as a new [`String`].
///
/// 'Uppercase' is defined according to the terms of the Unicode Derived Core Property
/// `Uppercase`.
///
/// Since some characters can expand into multiple characters when changing
/// the case, this function returns a [`String`] instead of modifying the
/// parameter in-place.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// let s = "hello";
///
/// assert_eq!("HELLO", s.to_uppercase());
/// ```
///
/// Scripts without case are not changed:
///
/// ```rune
/// let new_year = "农历新年";
///
/// assert_eq!(new_year, new_year.to_uppercase());
/// ```
///
/// One character can become multiple:
/// ```rune
/// let s = "tschüß";
///
/// assert_eq!("TSCHÜSS", s.to_uppercase());
/// ```
#[rune::function(instance)]
fn to_uppercase(s: &str) -> VmResult<String> {
    let mut uppercase = vm_try!(String::try_with_capacity(s.len()));
    vm_try!(uppercase.try_extend(s.chars().flat_map(|c| c.to_uppercase())));
    VmResult::Ok(uppercase)
}

crate::__internal_impl_any!(::std::string, FromUtf8Error);
crate::__internal_impl_any!(::std::string, Utf8Error);

#[derive(Any)]
#[rune(item = ::std::string)]
struct Chars {
    string: Ref<str>,
    start: usize,
    end: usize,
}

impl Chars {
    fn new(string: Ref<str>) -> Self {
        let end = string.len();
        Self {
            string,
            start: 0,
            end,
        }
    }

    #[rune::function(keep, protocol = NEXT)]
    fn next(&mut self) -> Option<char> {
        let string = self.string.get(self.start..self.end)?;
        let c = string.chars().next()?;
        self.start += c.len_utf8();
        Some(c)
    }

    #[rune::function(keep, protocol = NEXT_BACK)]
    fn next_back(&mut self) -> Option<char> {
        let string = self.string.get(self.start..self.end)?;
        let c = string.chars().next_back()?;
        self.end -= c.len_utf8();
        Some(c)
    }
}

trait Pattern: 'static + TryClone + Named + FromValue + ToValue + MaybeTypeOf + TypeOf {
    fn test(&self, tail: &str) -> VmResult<(bool, usize)>;

    fn is_empty(&self) -> bool;
}

impl Pattern for Box<str> {
    fn test(&self, tail: &str) -> VmResult<(bool, usize)> {
        if tail.starts_with(self.as_ref()) {
            VmResult::Ok((true, self.len()))
        } else {
            let Some(c) = tail.chars().next() else {
                return VmResult::Ok((false, 0));
            };

            VmResult::Ok((false, c.len_utf8()))
        }
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.as_ref().is_empty()
    }
}

impl Pattern for char {
    fn test(&self, tail: &str) -> VmResult<(bool, usize)> {
        let Some(c) = tail.chars().next() else {
            return VmResult::Ok((false, 0));
        };

        VmResult::Ok((c == *self, c.len_utf8()))
    }

    #[inline]
    fn is_empty(&self) -> bool {
        false
    }
}

impl Pattern for Function {
    fn test(&self, tail: &str) -> VmResult<(bool, usize)> {
        let Some(c) = tail.chars().next() else {
            return VmResult::Ok((false, 0));
        };

        VmResult::Ok((vm_try!(self.call::<bool>((c,))), c.len_utf8()))
    }

    #[inline]
    fn is_empty(&self) -> bool {
        false
    }
}

#[derive(Any)]
#[rune(item = ::std::string)]
struct Split<T>
where
    T: Pattern,
{
    string: Option<Ref<str>>,
    pattern: T,
    from: usize,
    to: usize,
}

impl<T> Split<T>
where
    T: Pattern,
{
    fn new(string: Ref<str>, pattern: T) -> Self {
        Self {
            string: Some(string),
            pattern,
            from: 0,
            to: 0,
        }
    }

    #[rune::function(keep, protocol = NEXT)]
    fn next(&mut self) -> VmResult<Option<String>> {
        let Some(string) = &self.string else {
            return VmResult::Ok(None);
        };

        if self.from == string.len() && self.from == self.to {
            self.string = None;
            let out = vm_try!("".try_to_owned());
            return VmResult::Ok(Some(out));
        }

        while self.to < string.len() {
            let Some(tail) = string.get(self.to..) else {
                return VmResult::Ok(None);
            };

            let (m, len) = vm_try!(self.pattern.test(tail));

            if m {
                let head = string.get(self.from..self.to).unwrap_or_default();
                let out = vm_try!(head.try_to_owned());

                if len == 0 {
                    self.from = self.to;
                    self.to += tail.chars().next().map_or(0, |c| c.len_utf8());
                } else {
                    self.to += len;
                    self.from = self.to;
                }

                return VmResult::Ok(Some(out));
            } else {
                self.to += len;
            }
        }

        let tail = string.get(self.from..self.to).unwrap_or_default();
        self.from = self.to;
        let out = vm_try!(tail.try_to_owned());

        if !self.pattern.is_empty() {
            self.string = None;
        }

        VmResult::Ok(Some(out))
    }

    #[rune::function(keep, protocol = INTO_ITER)]
    fn into_iter(self) -> Self {
        self
    }
}

// Inlined code from core::unicode, since using it directly is marked as using an
// unstable library feature
mod unicode {
    fn decode_prefix_sum(short_offset_run_header: u32) -> u32 {
        short_offset_run_header & ((1 << 21) - 1)
    }

    fn decode_length(short_offset_run_header: u32) -> usize {
        (short_offset_run_header >> 21) as usize
    }

    #[inline(always)]
    fn skip_search<const SOR: usize, const OFFSETS: usize>(
        needle: u32,
        short_offset_runs: &[u32; SOR],
        offsets: &[u8; OFFSETS],
    ) -> bool {
        // Note that this *cannot* be past the end of the array, as the last
        // element is greater than std::char::MAX (the largest possible needle).
        //
        // So, we cannot have found it (i.e. Ok(idx) + 1 != length) and the correct
        // location cannot be past it, so Err(idx) != length either.
        //
        // This means that we can avoid bounds checking for the accesses below, too.
        let last_idx =
            match short_offset_runs.binary_search_by_key(&(needle << 11), |header| header << 11) {
                Ok(idx) => idx + 1,
                Err(idx) => idx,
            };

        let mut offset_idx = decode_length(short_offset_runs[last_idx]);
        let length = if let Some(next) = short_offset_runs.get(last_idx + 1) {
            decode_length(*next) - offset_idx
        } else {
            offsets.len() - offset_idx
        };
        let prev = last_idx
            .checked_sub(1)
            .map(|prev| decode_prefix_sum(short_offset_runs[prev]))
            .unwrap_or(0);

        let total = needle - prev;
        let mut prefix_sum = 0;
        for _ in 0..(length - 1) {
            let offset = offsets[offset_idx];
            prefix_sum += offset as u32;
            if prefix_sum > total {
                break;
            }
            offset_idx += 1;
        }
        offset_idx % 2 == 1
    }

    #[rustfmt::skip]
    pub mod case_ignorable {
        static SHORT_OFFSET_RUNS: [u32; 35] = [
            688, 44045149, 572528402, 576724925, 807414908, 878718981, 903913493, 929080568, 933275148,
            937491230, 1138818560, 1147208189, 1210124160, 1222707713, 1235291428, 1260457643,
            1264654383, 1499535675, 1507925040, 1566646003, 1629566000, 1650551536, 1658941263,
            1671540720, 1688321181, 1700908800, 1709298023, 1717688832, 1738661888, 1763828398,
            1797383403, 1805773008, 1809970171, 1819148289, 1824457200,
        ];
        static OFFSETS: [u8; 875] = [
            39, 1, 6, 1, 11, 1, 35, 1, 1, 1, 71, 1, 4, 1, 1, 1, 4, 1, 2, 2, 0, 192, 4, 2, 4, 1, 9, 2,
            1, 1, 251, 7, 207, 1, 5, 1, 49, 45, 1, 1, 1, 2, 1, 2, 1, 1, 44, 1, 11, 6, 10, 11, 1, 1, 35,
            1, 10, 21, 16, 1, 101, 8, 1, 10, 1, 4, 33, 1, 1, 1, 30, 27, 91, 11, 58, 11, 4, 1, 2, 1, 24,
            24, 43, 3, 44, 1, 7, 2, 6, 8, 41, 58, 55, 1, 1, 1, 4, 8, 4, 1, 3, 7, 10, 2, 13, 1, 15, 1,
            58, 1, 4, 4, 8, 1, 20, 2, 26, 1, 2, 2, 57, 1, 4, 2, 4, 2, 2, 3, 3, 1, 30, 2, 3, 1, 11, 2,
            57, 1, 4, 5, 1, 2, 4, 1, 20, 2, 22, 6, 1, 1, 58, 1, 2, 1, 1, 4, 8, 1, 7, 2, 11, 2, 30, 1,
            61, 1, 12, 1, 50, 1, 3, 1, 55, 1, 1, 3, 5, 3, 1, 4, 7, 2, 11, 2, 29, 1, 58, 1, 2, 1, 6, 1,
            5, 2, 20, 2, 28, 2, 57, 2, 4, 4, 8, 1, 20, 2, 29, 1, 72, 1, 7, 3, 1, 1, 90, 1, 2, 7, 11, 9,
            98, 1, 2, 9, 9, 1, 1, 7, 73, 2, 27, 1, 1, 1, 1, 1, 55, 14, 1, 5, 1, 2, 5, 11, 1, 36, 9, 1,
            102, 4, 1, 6, 1, 2, 2, 2, 25, 2, 4, 3, 16, 4, 13, 1, 2, 2, 6, 1, 15, 1, 94, 1, 0, 3, 0, 3,
            29, 2, 30, 2, 30, 2, 64, 2, 1, 7, 8, 1, 2, 11, 3, 1, 5, 1, 45, 5, 51, 1, 65, 2, 34, 1, 118,
            3, 4, 2, 9, 1, 6, 3, 219, 2, 2, 1, 58, 1, 1, 7, 1, 1, 1, 1, 2, 8, 6, 10, 2, 1, 39, 1, 8, 31,
            49, 4, 48, 1, 1, 5, 1, 1, 5, 1, 40, 9, 12, 2, 32, 4, 2, 2, 1, 3, 56, 1, 1, 2, 3, 1, 1, 3,
            58, 8, 2, 2, 64, 6, 82, 3, 1, 13, 1, 7, 4, 1, 6, 1, 3, 2, 50, 63, 13, 1, 34, 101, 0, 1, 1,
            3, 11, 3, 13, 3, 13, 3, 13, 2, 12, 5, 8, 2, 10, 1, 2, 1, 2, 5, 49, 5, 1, 10, 1, 1, 13, 1,
            16, 13, 51, 33, 0, 2, 113, 3, 125, 1, 15, 1, 96, 32, 47, 1, 0, 1, 36, 4, 3, 5, 5, 1, 93, 6,
            93, 3, 0, 1, 0, 6, 0, 1, 98, 4, 1, 10, 1, 1, 28, 4, 80, 2, 14, 34, 78, 1, 23, 3, 103, 3, 3,
            2, 8, 1, 3, 1, 4, 1, 25, 2, 5, 1, 151, 2, 26, 18, 13, 1, 38, 8, 25, 11, 46, 3, 48, 1, 2, 4,
            2, 2, 17, 1, 21, 2, 66, 6, 2, 2, 2, 2, 12, 1, 8, 1, 35, 1, 11, 1, 51, 1, 1, 3, 2, 2, 5, 2,
            1, 1, 27, 1, 14, 2, 5, 2, 1, 1, 100, 5, 9, 3, 121, 1, 2, 1, 4, 1, 0, 1, 147, 17, 0, 16, 3,
            1, 12, 16, 34, 1, 2, 1, 169, 1, 7, 1, 6, 1, 11, 1, 35, 1, 1, 1, 47, 1, 45, 2, 67, 1, 21, 3,
            0, 1, 226, 1, 149, 5, 0, 6, 1, 42, 1, 9, 0, 3, 1, 2, 5, 4, 40, 3, 4, 1, 165, 2, 0, 4, 0, 2,
            80, 3, 70, 11, 49, 4, 123, 1, 54, 15, 41, 1, 2, 2, 10, 3, 49, 4, 2, 2, 2, 1, 4, 1, 10, 1,
            50, 3, 36, 5, 1, 8, 62, 1, 12, 2, 52, 9, 10, 4, 2, 1, 95, 3, 2, 1, 1, 2, 6, 1, 2, 1, 157, 1,
            3, 8, 21, 2, 57, 2, 3, 1, 37, 7, 3, 5, 195, 8, 2, 3, 1, 1, 23, 1, 84, 6, 1, 1, 4, 2, 1, 2,
            238, 4, 6, 2, 1, 2, 27, 2, 85, 8, 2, 1, 1, 2, 106, 1, 1, 1, 2, 6, 1, 1, 101, 3, 2, 4, 1, 5,
            0, 9, 1, 2, 0, 2, 1, 1, 4, 1, 144, 4, 2, 2, 4, 1, 32, 10, 40, 6, 2, 4, 8, 1, 9, 6, 2, 3, 46,
            13, 1, 2, 0, 7, 1, 6, 1, 1, 82, 22, 2, 7, 1, 2, 1, 2, 122, 6, 3, 1, 1, 2, 1, 7, 1, 1, 72, 2,
            3, 1, 1, 1, 0, 2, 11, 2, 52, 5, 5, 1, 1, 1, 0, 17, 6, 15, 0, 5, 59, 7, 9, 4, 0, 1, 63, 17,
            64, 2, 1, 2, 0, 4, 1, 7, 1, 2, 0, 2, 1, 4, 0, 46, 2, 23, 0, 3, 9, 16, 2, 7, 30, 4, 148, 3,
            0, 55, 4, 50, 8, 1, 14, 1, 22, 5, 1, 15, 0, 7, 1, 17, 2, 7, 1, 2, 1, 5, 5, 62, 33, 1, 160,
            14, 0, 1, 61, 4, 0, 5, 0, 7, 109, 8, 0, 5, 0, 1, 30, 96, 128, 240, 0,
        ];
        pub fn lookup(c: char) -> bool {
            super::skip_search(
                c as u32,
                &SHORT_OFFSET_RUNS,
                &OFFSETS,
            )
        }
    }

    #[rustfmt::skip]
    pub mod cased {
        static SHORT_OFFSET_RUNS: [u32; 22] = [
            4256, 115348384, 136322176, 144711446, 163587254, 320875520, 325101120, 350268208,
            392231680, 404815649, 413205504, 421595008, 467733632, 484513952, 492924480, 497144832,
            501339814, 578936576, 627171376, 639756544, 643952944, 649261450,
        ];
        static OFFSETS: [u8; 315] = [
            65, 26, 6, 26, 47, 1, 10, 1, 4, 1, 5, 23, 1, 31, 1, 195, 1, 4, 4, 208, 1, 36, 7, 2, 30, 5,
            96, 1, 42, 4, 2, 2, 2, 4, 1, 1, 6, 1, 1, 3, 1, 1, 1, 20, 1, 83, 1, 139, 8, 166, 1, 38, 9,
            41, 0, 38, 1, 1, 5, 1, 2, 43, 1, 4, 0, 86, 2, 6, 0, 9, 7, 43, 2, 3, 64, 192, 64, 0, 2, 6, 2,
            38, 2, 6, 2, 8, 1, 1, 1, 1, 1, 1, 1, 31, 2, 53, 1, 7, 1, 1, 3, 3, 1, 7, 3, 4, 2, 6, 4, 13,
            5, 3, 1, 7, 116, 1, 13, 1, 16, 13, 101, 1, 4, 1, 2, 10, 1, 1, 3, 5, 6, 1, 1, 1, 1, 1, 1, 4,
            1, 6, 4, 1, 2, 4, 5, 5, 4, 1, 17, 32, 3, 2, 0, 52, 0, 229, 6, 4, 3, 2, 12, 38, 1, 1, 5, 1,
            0, 46, 18, 30, 132, 102, 3, 4, 1, 59, 5, 2, 1, 1, 1, 5, 24, 5, 1, 3, 0, 43, 1, 14, 6, 80, 0,
            7, 12, 5, 0, 26, 6, 26, 0, 80, 96, 36, 4, 36, 116, 11, 1, 15, 1, 7, 1, 2, 1, 11, 1, 15, 1,
            7, 1, 2, 0, 1, 2, 3, 1, 42, 1, 9, 0, 51, 13, 51, 0, 64, 0, 64, 0, 85, 1, 71, 1, 2, 2, 1, 2,
            2, 2, 4, 1, 12, 1, 1, 1, 7, 1, 65, 1, 4, 2, 8, 1, 7, 1, 28, 1, 4, 1, 5, 1, 1, 3, 7, 1, 0, 2,
            25, 1, 25, 1, 31, 1, 25, 1, 31, 1, 25, 1, 31, 1, 25, 1, 31, 1, 25, 1, 8, 0, 10, 1, 20, 6, 6,
            0, 62, 0, 68, 0, 26, 6, 26, 6, 26, 0,
        ];
        pub fn lookup(c: char) -> bool {
            super::skip_search(
                c as u32,
                &SHORT_OFFSET_RUNS,
                &OFFSETS,
            )
        }
    }
}
