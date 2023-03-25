//! The `std::io` module.

use std::fmt;
use std::fmt::Write as _;
use std::io;
use std::io::Write as _;

use crate as rune;
use crate::macros::{quote, FormatArgs, MacroContext, TokenStream};
use crate::parse::Parser;
use crate::runtime::{Panic, Protocol, Stack, Value, VmError};
use crate::{ContextError, Module};

/// Construct the `std::io` module.
pub fn module(stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["io"]).with_unique("std::io");

    module.ty::<io::Error>()?;
    module.inst_fn(Protocol::STRING_DISPLAY, format_io_error)?;

    if stdio {
        module.function_meta(print_impl)?;
        module.function_meta(println_impl)?;
        module.raw_fn(["dbg"], dbg_impl)?;
    }

    // These are unconditionally included, but using them might cause a
    // compilation error unless `::std::io::*` functions are provided somehow.
    module.macro_(["dbg"], dbg_macro)?;
    module.macro_(["print"], print_macro)?;
    module.macro_(["println"], println_macro)?;
    Ok(module)
}

fn format_io_error(error: &std::io::Error, buf: &mut String) -> fmt::Result {
    write!(buf, "{}", error)
}

fn dbg_impl(stack: &mut Stack, args: usize) -> Result<(), VmError> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for value in stack.drain(args)? {
        writeln!(stdout, "{:?}", value).map_err(VmError::panic)?;
    }

    stack.push(Value::Unit);
    Ok(())
}

/// Implementation for the `dbg!` macro.
pub(crate) fn dbg_macro(
    ctx: &mut MacroContext<'_>,
    stream: &TokenStream,
) -> crate::Result<TokenStream> {
    Ok(quote!(::std::io::dbg(#stream)).into_token_stream(ctx))
}

/// Implementation for the `print!` macro.
pub(crate) fn print_macro(
    ctx: &mut MacroContext<'_>,
    stream: &TokenStream,
) -> crate::Result<TokenStream> {
    let mut p = Parser::from_token_stream(stream, ctx.stream_span());
    let args = p.parse_all::<FormatArgs>()?;
    let expanded = args.expand(ctx)?;
    Ok(quote!(::std::io::print(#expanded)).into_token_stream(ctx))
}

/// Print to stdout.
///
/// This is provided on top of the [`print!`] macro so that it can be used as
/// a function.
///
/// # Examples
///
/// ```rune
/// print("Hi!");
/// ```
#[rune::function(path = print)]
fn print_impl(m: &str) -> Result<(), Panic> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    write!(stdout, "{}", m).map_err(Panic::custom)
}

/// Implementation for the `println!` macro.
pub(crate) fn println_macro(
    ctx: &mut MacroContext<'_>,
    stream: &TokenStream,
) -> crate::Result<TokenStream> {
    let mut p = Parser::from_token_stream(stream, ctx.stream_span());
    let args = p.parse_all::<FormatArgs>()?;
    let expanded = args.expand(ctx)?;
    Ok(quote!(::std::io::println(#expanded)).into_token_stream(ctx))
}

/// Print to stdout adding a newline to what is being printed.
///
/// This is provided on top of the [`println!`] macro so that it can be used as
/// a function.
///
/// # Examples
///
/// ```rune
/// println("Hi!");
/// ```
#[rune::function(path = println)]
fn println_impl(message: &str) -> Result<(), Panic> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    writeln!(stdout, "{}", message).map_err(Panic::custom)
}
