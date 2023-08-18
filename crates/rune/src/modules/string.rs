//! The `std::string` module.

use core::char;
use core::fmt::{self, Write};
use core::num;

use crate::no_std::prelude::*;

use crate as rune;
use crate::runtime::{Bytes, Iterator, Panic, Protocol, Value, VmErrorKind, VmResult};
use crate::{Any, ContextError, Module};

/// Construct the `std::string` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["string"]);

    module.ty::<String>()?;

    module.function(["String", "from_str"], |s: &str| {
        <String as From<&str>>::from(s)
    })?;
    module.function(["String", "new"], String::new)?;
    module.function(["String", "with_capacity"], String::with_capacity)?;

    module.associated_function("cmp", str::cmp)?;
    module.associated_function("len", String::len)?;
    module.associated_function("starts_with", |a: &str, b: &str| a.starts_with(b))?;
    module.associated_function("ends_with", |a: &str, b: &str| a.ends_with(b))?;
    module.associated_function("capacity", String::capacity)?;
    module.associated_function("clear", String::clear)?;
    module.associated_function("contains", |a: &str, b: &str| a.contains(b))?;
    module.associated_function("push", String::push)?;
    module.associated_function("push_str", String::push_str)?;
    module.associated_function("reserve", String::reserve)?;
    module.associated_function("reserve_exact", String::reserve_exact)?;
    module.associated_function("into_bytes", into_bytes)?;
    module.associated_function("clone", String::clone)?;
    module.associated_function("shrink_to_fit", String::shrink_to_fit)?;
    module.associated_function("char_at", char_at)?;
    module.function_meta(split)?;
    module.function_meta(trim)?;
    module.function_meta(trim_end)?;
    module.function_meta(replace)?;
    // TODO: deprecate this variant.
    module.associated_function("split_str", __rune_fn__split)?;
    module.associated_function("is_empty", str::is_empty)?;
    module.function_meta(chars)?;
    module.associated_function(Protocol::ADD, add)?;
    module.associated_function(Protocol::ADD_ASSIGN, String::push_str)?;
    module.associated_function(Protocol::INDEX_GET, string_index_get)?;
    module.function_meta(get)?;
    module.function_meta(parse_int)?;
    module.function_meta(parse_char)?;
    Ok(module)
}

#[derive(Any, Debug, Clone, Copy)]
#[rune(module = crate, item = ::std::string, install_with = NotCharBoundary::install)]
struct NotCharBoundary(());

impl NotCharBoundary {
    fn string_display(&self, s: &mut String) -> fmt::Result {
        write!(s, "index outside of character boundary")
    }

    fn install(m: &mut Module) -> Result<(), ContextError> {
        m.associated_function(Protocol::STRING_DISPLAY, Self::string_display)?;
        Ok(())
    }
}

/// into_bytes shim for strings.
fn into_bytes(s: String) -> Bytes {
    Bytes::from_vec(s.into_bytes())
}

fn char_at(s: &str, index: usize) -> Option<char> {
    if !s.is_char_boundary(index) {
        return None;
    }

    s[index..].chars().next()
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
    let lines = match value {
        Value::String(s) => this
            .split(vm_try!(s.borrow_ref()).as_str())
            .map(String::from)
            .collect::<Vec<String>>(),
        Value::StaticString(s) => this
            .split(s.as_str())
            .map(String::from)
            .collect::<Vec<String>>(),
        Value::Char(pat) => this.split(pat).map(String::from).collect::<Vec<String>>(),
        Value::Function(f) => {
            let f = vm_try!(f.borrow_ref());
            let mut err = None;

            let lines = this.split(|c: char| match f.call::<_, bool>((c,)) {
                VmResult::Ok(b) => b,
                VmResult::Err(e) => {
                    if err.is_none() {
                        err = Some(e);
                    }

                    false
                }
            });

            let lines = lines.map(String::from).collect::<Vec<String>>();

            if let Some(e) = err.take() {
                return VmResult::Err(e);
            }

            lines
        }
        value => return VmResult::err(vm_try!(VmErrorKind::bad_argument::<String>(0, &value))),
    };

    VmResult::Ok(Iterator::from_double_ended(
        "std::str::Split",
        lines.into_iter(),
    ))
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
fn trim(this: &str) -> String {
    this.trim().to_owned()
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
fn trim_end(this: &str) -> String {
    this.trim_end().to_owned()
}

/// The add operation for strings.
fn add(a: &str, b: &str) -> String {
    let mut string = String::with_capacity(a.len() + b.len());
    string.push_str(a);
    string.push_str(b);
    string
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
/// ```
/// let s = "this is old";
/// assert_eq!(s, s.replace("cookie monster", "little lamb"));
/// ```
#[rune::function(instance)]
fn replace(a: &str, from: &str, to: &str) -> String {
    a.replace(from, to)
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
fn chars(s: &str) -> Iterator {
    let iter = s.chars().collect::<Vec<_>>().into_iter();
    Iterator::from_double_ended("std::str::Chars", iter)
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
fn get(s: &str, key: Value) -> VmResult<Option<String>> {
    use crate::runtime::{FromValue, RangeLimits, TypeOf};

    match key {
        Value::Range(range) => {
            let range = vm_try!(range.borrow_ref());

            let start = match range.start.clone() {
                Some(value) => Some(vm_try!(<usize>::from_value(value))),
                None => None,
            };

            let end = match range.end.clone() {
                Some(value) => Some(vm_try!(<usize>::from_value(value))),
                None => None,
            };

            let out = match range.limits {
                RangeLimits::HalfOpen => match (start, end) {
                    (Some(start), Some(end)) => s.get(start..end),
                    (Some(start), None) => s.get(start..),
                    (None, Some(end)) => s.get(..end),
                    (None, None) => s.get(..),
                },
                RangeLimits::Closed => match (start, end) {
                    (Some(start), Some(end)) => s.get(start..=end),
                    (None, Some(end)) => s.get(..=end),
                    _ => return VmResult::err(VmErrorKind::UnsupportedRange),
                },
            };

            VmResult::Ok(out.map(|out| out.to_owned()))
        }
        index => VmResult::err(VmErrorKind::UnsupportedIndexGet {
            target: String::type_info(),
            index: vm_try!(index.type_info()),
        }),
    }
}

/// Get a specific string index.
fn string_index_get(s: &str, key: Value) -> VmResult<String> {
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
fn parse_int(s: &str) -> Result<i64, num::ParseIntError> {
    str::parse::<i64>(s)
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
