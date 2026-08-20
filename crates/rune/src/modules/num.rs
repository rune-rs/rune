//! Working with numbers.

use core::num::{ParseFloatError, ParseIntError};

use crate as rune;
use crate::alloc;
use crate::alloc::fmt::TryWrite;
use crate::runtime::Formatter;
use crate::{ContextError, Module};

/// Working with numbers.
///
/// This module provides types generic for working over numbers, such as errors
/// when a number cannot be parsed.
#[rune::module(::std::num)]
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::from_meta(self::module__meta)?;

    module.ty::<ParseFloatError>()?;
    module.function_meta(parse_float_error_display_fmt)?;
    module.function_meta(parse_float_error_debug_fmt)?;

    module.ty::<ParseIntError>()?;
    module.function_meta(parse_int_error_display_fmt)?;
    module.function_meta(parse_int_error_debug_fmt)?;

    Ok(module)
}

/// Write why a float could not be parsed.
///
/// # Examples
///
/// ```rune
/// let text = if let Err(error) = f64::parse("x") {
///     format!("{error}")
/// } else {
///     ""
/// };
///
/// assert_eq!(text, "invalid float literal");
/// ```
#[rune::function(instance, protocol = DISPLAY_FMT)]
fn parse_float_error_display_fmt(error: &ParseFloatError, f: &mut Formatter) -> alloc::Result<()> {
    write!(f, "{error}")
}

/// Write a debug representation of why a float could not be parsed.
///
/// # Examples
///
/// ```rune
/// let text = if let Err(error) = f64::parse("x") {
///     format!("{error:?}")
/// } else {
///     ""
/// };
///
/// assert!(text.starts_with("ParseFloatError"));
/// ```
#[rune::function(instance, protocol = DEBUG_FMT)]
fn parse_float_error_debug_fmt(error: &ParseFloatError, f: &mut Formatter) -> alloc::Result<()> {
    write!(f, "{error:?}")
}

/// Write why an integer could not be parsed.
///
/// # Examples
///
/// ```rune
/// let text = if let Err(error) = i64::parse("x") {
///     format!("{error}")
/// } else {
///     ""
/// };
///
/// assert_eq!(text, "invalid digit found in string");
/// ```
#[rune::function(instance, protocol = DISPLAY_FMT)]
fn parse_int_error_display_fmt(error: &ParseIntError, f: &mut Formatter) -> alloc::Result<()> {
    write!(f, "{error}")
}

/// Write a debug representation of why an integer could not be parsed.
///
/// # Examples
///
/// ```rune
/// let text = if let Err(error) = i64::parse("x") {
///     format!("{error:?}")
/// } else {
///     ""
/// };
///
/// assert!(text.starts_with("ParseIntError"));
/// ```
#[rune::function(instance, protocol = DEBUG_FMT)]
fn parse_int_error_debug_fmt(error: &ParseIntError, f: &mut Formatter) -> alloc::Result<()> {
    write!(f, "{error:?}")
}
