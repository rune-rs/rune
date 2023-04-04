//! The `std::char` module.

use crate::runtime::{Value, VmError, VmErrorKind, VmResult};
use crate::{ContextError, Module};
use std::char::ParseCharError;

use crate as rune;

/// Construct the `std::char` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["char"]);
    module.ty::<ParseCharError>()?;

    module.function_meta(from_int)?;
    module.function_meta(to_int)?;
    module.function_meta(is_alphabetic)?;
    module.function_meta(is_alphanumeric)?;
    module.inst_fn("is_control", char::is_control)?;
    module.inst_fn("is_lowercase", char::is_lowercase)?;
    module.inst_fn("is_numeric", char::is_numeric)?;
    module.inst_fn("is_uppercase", char::is_uppercase)?;
    module.inst_fn("is_whitespace", char::is_whitespace)?;
    module.inst_fn("to_digit", char::to_digit)?;
    Ok(module)
}

/// Convert a character into an integer.
///
/// # Examples
///
/// ```rune
/// let c = char::from_int(80)?;
/// assert_eq!(c.to_int(), 80);
/// ```
#[rune::function(instance)]
fn to_int(value: char) -> Value {
    (value as i64).into()
}

/// Try to convert a number into a character.
///
/// # Examples
///
/// ```rune
/// let c = char::from_int(80);
/// assert!(c.is_some());
/// ```
#[rune::function]
fn from_int(value: i64) -> VmResult<Option<Value>> {
    if value < 0 {
        VmResult::Err(VmError::from(VmErrorKind::Underflow))
    } else if value > u32::MAX as i64 {
        VmResult::Err(VmError::from(VmErrorKind::Overflow))
    } else {
        VmResult::Ok(std::char::from_u32(value as u32).map(|v| v.into()))
    }
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

crate::__internal_impl_any!(ParseCharError);
