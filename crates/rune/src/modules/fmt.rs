//! Formatting text.

use core::fmt;

use crate as rune;
use crate::alloc::fmt::TryWrite;
use crate::alloc::prelude::*;
use crate::compile;
use crate::macros::{FormatArgs, MacroContext, TokenStream};
use crate::parse::Parser;
use crate::runtime::{EnvProtocolCaller, Format, Formatter, VmResult};
use crate::{ContextError, Module};

/// Formatting text.
///
/// This includes types, macros, and functions used to format text.
///
/// # Examples
///
/// ```rune
/// let who = "World";
/// let string = format!("Hello {}", who);
/// assert_eq!(string, "Hello World");
/// ```
#[rune::module(::std::fmt)]
pub fn module() -> Result<Module, ContextError> {
    let mut m = Module::from_meta(self::module_meta)?.with_unique("std::fmt");

    m.ty::<Formatter>()?;
    m.ty::<fmt::Error>()?;
    m.function_meta(fmt_error_display_fmt)?;
    m.macro_meta(format)?;

    m.ty::<Format>()?;
    m.function_meta(format_display_fmt__meta)?;
    m.function_meta(format_debug_fmt__meta)?;
    m.function_meta(format_clone__meta)?;
    m.implement_trait::<Format>(rune::item!(::std::clone::Clone))?;

    Ok(m)
}

#[rune::function(instance, protocol = DISPLAY_FMT)]
fn fmt_error_display_fmt(error: &fmt::Error, f: &mut Formatter) -> VmResult<()> {
    vm_write!(f, "{error}")
}

/// Format a string using a format specifier.
///
/// # Examples
///
/// ```rune
/// let who = "World";
/// let string = format!("Hello {}", who);
/// assert_eq!(string, "Hello World");
/// ```
#[rune::macro_(path = format)]
pub(crate) fn format(
    cx: &mut MacroContext<'_, '_, '_>,
    stream: &TokenStream,
) -> compile::Result<TokenStream> {
    let mut p = Parser::from_token_stream(stream, cx.input_span());
    let args = p.parse::<FormatArgs>()?;
    p.eof()?;
    let expanded = args.expand(cx)?;
    Ok(expanded.into_token_stream(cx)?)
}

/// Write a display representation of a format specification.
///
/// # Examples
///
/// ```rune
/// let value = #[builtin] format!("Hello", fill = '0', width = 10);
/// assert_eq!(format!("{value}"), "Hello00000");
/// ```
#[rune::function(keep, instance, protocol = DISPLAY_FMT)]
fn format_display_fmt(format: &Format, f: &mut Formatter) -> VmResult<()> {
    vm_try!(format.spec.format(&format.value, f, &mut EnvProtocolCaller));
    VmResult::Ok(())
}

/// Write a debug representation of a format specification.
///
/// # Examples
///
/// ```rune
/// let value = #[builtin] format!("Hello", fill = '0', width = 10);
/// let string = format!("{value:?}");
/// assert!(string is String);
/// ```
#[rune::function(keep, instance, protocol = DEBUG_FMT)]
fn format_debug_fmt(format: &Format, f: &mut Formatter) -> VmResult<()> {
    vm_write!(f, "{format:?}")
}

/// Clones a format specification.
///
/// # Examples
///
/// ```rune
/// let value = #[builtin] format!("Hello", fill = '0', width = 10);
/// let vlaue2 = value.clone();
/// assert_eq!(format!("{value}"), "Hello00000");
/// ```
#[rune::function(keep, instance, protocol = CLONE)]
fn format_clone(this: &Format) -> VmResult<Format> {
    VmResult::Ok(vm_try!(this.try_clone()))
}
