//! Utilities for the `str` primitive type.
//!
//! *[See also the `str` primitive type](str).*

use crate::alloc::{Allocator, Global};
use crate::borrow::TryToOwned;
use crate::boxed::Box;
use crate::error::Error;
use crate::string::String;
use crate::vec::Vec;
use crate::Result;

/// Converts a boxed slice of bytes to a boxed string slice without checking
/// that the string contains valid UTF-8.
///
/// # Examples
///
/// ```
/// use rune::alloc::Box;
/// use rune::alloc::str;
///
/// let smile_utf8 = Box::try_from([226, 152, 186])?;
/// let smile = unsafe { str::from_boxed_utf8_unchecked(smile_utf8) };
///
/// assert_eq!("☺", &*smile);
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// # Safety
///
/// The provided buffer must be valid UTF-8.
#[must_use]
#[inline]
pub unsafe fn from_boxed_utf8_unchecked<A>(v: Box<[u8], A>) -> Box<str, A>
where
    A: Allocator,
{
    let (ptr, alloc) = Box::into_raw_with_allocator(v);
    unsafe { Box::from_raw_in(ptr as *mut str, alloc) }
}

/// Converts a [`Box<str>`] into a [`String`] without copying or allocating.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use rune::alloc::String;
/// use rune::alloc::str;
/// use rune::alloc::prelude::*;
///
/// let string = String::try_from("birthday gift")?;
/// let boxed_str = string.try_clone()?.try_into_boxed_str()?;
///
/// assert_eq!(str::into_string(boxed_str), string);
/// # Ok::<_, rune::alloc::Error>(())
/// ```
#[must_use = "`self` will be dropped if the result is not used"]
#[inline]
pub fn into_string<A>(this: Box<str, A>) -> String<A>
where
    A: Allocator,
{
    let slice = Box::<[u8], A>::from(this);
    let vec = crate::slice::into_vec(slice);
    unsafe { String::<A>::from_utf8_unchecked(vec) }
}

/// Replaces all matches of a pattern with another string.
///
/// `replace` creates a new [`String`], and copies the data from this string slice into it.
/// While doing so, it attempts to find matches of a pattern. If it finds any, it
/// replaces them with the replacement string slice.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// let s = "this is old";
///
/// assert_eq!("this is new", rune::alloc::str::replace(s, "old", "new")?);
/// assert_eq!("than an old", rune::alloc::str::replace(s, "is", "an")?);
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// When the pattern doesn't match, it returns this string slice as [`String`]:
///
/// ```
/// let s = "this is old";
/// assert_eq!(s, rune::alloc::str::replace(s, "cookie monster", "little lamb")?);
/// # Ok::<_, rune::alloc::Error>(())
/// ```
///
/// Single ascii-character replacements are optimized for performance:
///
/// ```
/// assert_eq!("say", rune::alloc::str::replace("bay", "b", "s")?);
/// # Ok::<_, rune::alloc::Error>(())
/// ```
pub fn replace(string: &str, from: &str, to: &str) -> Result<String> {
    // Fast path for replacing a single ASCII character with another.
    if let (&[from_byte], &[to_byte]) = (from.as_bytes(), to.as_bytes()) {
        return unsafe { replace_ascii(string.as_bytes(), from_byte, to_byte) };
    }

    // Set result capacity to self.len() when from.len() <= to.len()
    let default_capacity = if from.len() <= to.len() {
        string.len()
    } else {
        0
    };

    let mut result = String::try_with_capacity(default_capacity)?;
    let mut last_end = 0;

    for (start, part) in string.match_indices(from) {
        result.try_push_str(unsafe { string.get_unchecked(last_end..start) })?;
        result.try_push_str(to)?;
        last_end = start + part.len();
    }

    result.try_push_str(unsafe { string.get_unchecked(last_end..string.len()) })?;
    Ok(result)
}

unsafe fn replace_ascii(bytes: &[u8], from: u8, to: u8) -> Result<String> {
    let mut result = Vec::try_with_capacity(bytes.len())?;

    for &b in bytes {
        if b == from {
            result.try_push(to)?;
        } else {
            result.try_push(b)?;
        }
    }

    // SAFETY: We replaced ascii with ascii on valid utf8 strings.
    Ok(String::from_utf8_unchecked(result))
}

impl TryToOwned for str {
    type Owned = String<Global>;

    #[inline]
    fn try_to_owned(&self) -> Result<String<Global>, Error> {
        Ok(unsafe { String::from_utf8_unchecked(self.as_bytes().try_to_owned()?) })
    }
}
