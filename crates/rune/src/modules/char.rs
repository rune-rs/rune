//! The `std::char` module.

use core::char::ParseCharError;

use crate::runtime::{Value, VmErrorKind, VmResult};
use crate::{ContextError, Module};

use crate as rune;

/// Construct the `std::char` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["char"])?;
    module.ty::<ParseCharError>()?;

    module.function_meta(from_i64)?;
    module.function_meta(to_i64)?;
    module.function_meta(is_alphabetic)?;
    module.function_meta(is_alphanumeric)?;
    module.function_meta(is_control)?;
    module.function_meta(is_lowercase)?;
    module.function_meta(is_numeric)?;
    module.function_meta(is_uppercase)?;
    module.function_meta(is_whitespace)?;
    module.function_meta(to_digit)?;
    Ok(module)
}

/// Try to convert a number into a character.
///
/// # Examples
///
/// ```rune
/// let c = char::from_i64(80);
/// assert!(c.is_some());
/// ```
#[rune::function]
fn from_i64(value: i64) -> VmResult<Option<Value>> {
    if value < 0 {
        VmResult::err(VmErrorKind::Underflow)
    } else if value > u32::MAX as i64 {
        VmResult::err(VmErrorKind::Overflow)
    } else {
        VmResult::Ok(core::char::from_u32(value as u32).map(|v| v.into()))
    }
}

/// Convert a character into an integer.
///
/// # Examples
///
/// ```rune
/// let c = char::from_i64(80)?;
/// assert_eq!(c.to_i64(), 80);
/// ```
#[rune::function(instance)]
fn to_i64(value: char) -> Value {
    (value as i64).into()
}

/// Returns `true` if this `char` has the `Alphabetic` property.
///
/// `Alphabetic` is described in Chapter 4 (Character Properties) of the [Unicode Standard] and
/// specified in the [Unicode Character Database][ucd] [`DerivedCoreProperties.txt`].
///
/// [Unicode Standard]: https://www.unicode.org/versions/latest/
/// [ucd]: https://www.unicode.org/reports/tr44/
/// [`DerivedCoreProperties.txt`]: https://www.unicode.org/Public/UCD/latest/ucd/DerivedCoreProperties.txt
///
/// # Examples
///
/// ```rune
/// assert!('a'.is_alphabetic());
/// assert!('京'.is_alphabetic());
///
/// let c = '💝';
/// // love is many things, but it is not alphabetic
/// assert!(!c.is_alphabetic());
/// ```
#[rune::function(instance)]
#[inline]
fn is_alphabetic(c: char) -> bool {
    char::is_alphabetic(c)
}

/// Returns `true` if this `char` satisfies either [`is_alphabetic()`] or [`is_numeric()`].
///
/// [`is_alphabetic()`]: #method.is_alphabetic
/// [`is_numeric()`]: #method.is_numeric
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert!('٣'.is_alphanumeric());
/// assert!('7'.is_alphanumeric());
/// assert!('৬'.is_alphanumeric());
/// assert!('¾'.is_alphanumeric());
/// assert!('①'.is_alphanumeric());
/// assert!('K'.is_alphanumeric());
/// assert!('و'.is_alphanumeric());
/// assert!('藏'.is_alphanumeric());
/// ```
#[rune::function(instance)]
#[inline]
fn is_alphanumeric(c: char) -> bool {
    char::is_alphanumeric(c)
}

/// Returns `true` if this `char` has the general category for control codes.
///
/// Control codes (code points with the general category of `Cc`) are described
/// in Chapter 4 (Character Properties) of the [Unicode Standard] and specified
/// in the [Unicode Character Database][ucd] [`UnicodeData.txt`].
///
/// [Unicode Standard]: https://www.unicode.org/versions/latest/
/// [ucd]: https://www.unicode.org/reports/tr44/
/// [`UnicodeData.txt`]:
///     https://www.unicode.org/Public/UCD/latest/ucd/UnicodeData.txt
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// // U+009C, STRING TERMINATOR
/// assert!('\u{009c}'.is_control());
/// assert!(!'q'.is_control());
/// ```
#[rune::function(instance)]
#[inline]
fn is_control(c: char) -> bool {
    char::is_control(c)
}

/// Returns `true` if this `char` has the `Lowercase` property.
///
/// `Lowercase` is described in Chapter 4 (Character Properties) of the [Unicode
/// Standard] and specified in the [Unicode Character Database][ucd]
/// [`DerivedCoreProperties.txt`].
///
/// [Unicode Standard]: https://www.unicode.org/versions/latest/
/// [ucd]: https://www.unicode.org/reports/tr44/
/// [`DerivedCoreProperties.txt`]: https://www.unicode.org/Public/UCD/latest/ucd/DerivedCoreProperties.txt
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert!('a'.is_lowercase());
/// assert!('δ'.is_lowercase());
/// assert!(!'A'.is_lowercase());
/// assert!(!'Δ'.is_lowercase());
///
/// // The various Chinese scripts and punctuation do not have case, and so:
/// assert!(!'中'.is_lowercase());
/// assert!(!' '.is_lowercase());
/// ```
#[rune::function(instance)]
#[inline]
fn is_lowercase(c: char) -> bool {
    char::is_lowercase(c)
}

/// Returns `true` if this `char` has one of the general categories for numbers.
///
/// The general categories for numbers (`Nd` for decimal digits, `Nl` for
/// letter-like numeric characters, and `No` for other numeric characters) are
/// specified in the [Unicode Character Database][ucd] [`UnicodeData.txt`].
///
/// This method doesn't cover everything that could be considered a number, e.g.
/// ideographic numbers like '三'. If you want everything including characters
/// with overlapping purposes then you might want to use a unicode or
/// language-processing library that exposes the appropriate character
/// properties instead of looking at the unicode categories.
///
/// If you want to parse ASCII decimal digits (0-9) or ASCII base-N, use
/// `is_ascii_digit` or `is_digit` instead.
///
/// [Unicode Standard]: https://www.unicode.org/versions/latest/
/// [ucd]: https://www.unicode.org/reports/tr44/
/// [`UnicodeData.txt`]: https://www.unicode.org/Public/UCD/latest/ucd/UnicodeData.txt
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert!('٣'.is_numeric());
/// assert!('7'.is_numeric());
/// assert!('৬'.is_numeric());
/// assert!('¾'.is_numeric());
/// assert!('①'.is_numeric());
/// assert!(!'K'.is_numeric());
/// assert!(!'و'.is_numeric());
/// assert!(!'藏'.is_numeric());
/// assert!(!'三'.is_numeric());
/// ```
#[rune::function(instance)]
#[inline]
fn is_numeric(c: char) -> bool {
    char::is_numeric(c)
}

/// Returns `true` if this `char` has the `Uppercase` property.
///
/// `Uppercase` is described in Chapter 4 (Character Properties) of the [Unicode
/// Standard] and specified in the [Unicode Character Database][ucd]
/// [`DerivedCoreProperties.txt`].
///
/// [Unicode Standard]: https://www.unicode.org/versions/latest/
/// [ucd]: https://www.unicode.org/reports/tr44/
/// [`DerivedCoreProperties.txt`]: https://www.unicode.org/Public/UCD/latest/ucd/DerivedCoreProperties.txt
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert!(!'a'.is_uppercase());
/// assert!(!'δ'.is_uppercase());
/// assert!('A'.is_uppercase());
/// assert!('Δ'.is_uppercase());
///
/// // The various Chinese scripts and punctuation do not have case, and so:
/// assert!(!'中'.is_uppercase());
/// assert!(!' '.is_uppercase());
/// ```
#[rune::function(instance)]
#[inline]
fn is_uppercase(c: char) -> bool {
    char::is_uppercase(c)
}

/// Returns `true` if this `char` has the `White_Space` property.
///
/// `White_Space` is specified in the [Unicode Character Database][ucd]
/// [`PropList.txt`].
///
/// [ucd]: https://www.unicode.org/reports/tr44/
/// [`PropList.txt`]: https://www.unicode.org/Public/UCD/latest/ucd/PropList.txt
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert!(' '.is_whitespace());
///
/// // line break
/// assert!('\n'.is_whitespace());
///
/// // a non-breaking space
/// assert!('\u{A0}'.is_whitespace());
///
/// assert!(!'越'.is_whitespace());
/// ```
#[rune::function(instance)]
#[inline]
fn is_whitespace(c: char) -> bool {
    char::is_whitespace(c)
}

/// Converts a `char` to a digit in the given radix.
///
/// A 'radix' here is sometimes also called a 'base'. A radix of two
/// indicates a binary number, a radix of ten, decimal, and a radix of
/// sixteen, hexadecimal, to give some common values. Arbitrary
/// radices are supported.
///
/// 'Digit' is defined to be only the following characters:
///
/// * `0-9`
/// * `a-z`
/// * `A-Z`
///
/// # Errors
///
/// Returns `None` if the `char` does not refer to a digit in the given radix.
///
/// # Panics
///
/// Panics if given a radix larger than 36.
///
/// # Examples
///
/// Basic usage:
///
/// ```rune
/// assert_eq!('1'.to_digit(10), Some(1));
/// assert_eq!('f'.to_digit(16), Some(15));
/// ```
///
/// Passing a non-digit results in failure:
///
/// ```rune
/// assert_eq!('f'.to_digit(10), None);
/// assert_eq!('z'.to_digit(16), None);
/// ```
///
/// Passing a large radix, causing a panic:
///
/// ```rune,should_panic
/// // this panics
/// let _ = '1'.to_digit(37);
/// ```
#[rune::function(instance)]
#[inline]
fn to_digit(c: char, radix: u32) -> VmResult<Option<u32>> {
    if radix > 36 {
        return VmResult::panic("to_digit: radix is too high (maximum 36)");
    }

    VmResult::Ok(char::to_digit(c, radix))
}

crate::__internal_impl_any!(::std::char, ParseCharError);
