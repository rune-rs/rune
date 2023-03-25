//! The `std::char` module.

use crate::runtime::{Value, VmError, VmErrorKind};
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
    module.function(["is_alphanumeric"], char::is_alphanumeric)?;
    module.function(["is_control"], char::is_control)?;
    module.function(["is_lowercase"], char::is_lowercase)?;
    module.function(["is_numeric"], char::is_numeric)?;
    module.function(["is_uppercase"], char::is_uppercase)?;
    module.function(["is_whitespace"], char::is_whitespace)?;
    module.function(["to_digit"], char::to_digit)?;
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
fn to_int(value: char) -> Result<Value, VmError> {
    Ok((value as i64).into())
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
fn from_int(value: i64) -> Result<Option<Value>, VmError> {
    if value < 0 {
        Err(VmError::from(VmErrorKind::Underflow))
    } else if value > u32::MAX as i64 {
        Err(VmError::from(VmErrorKind::Overflow))
    } else {
        Ok(std::char::from_u32(value as u32).map(|v| v.into()))
    }
}

/// Convert a character into an integer.
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

crate::__internal_impl_any!(ParseCharError);
