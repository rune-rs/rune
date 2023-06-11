//! The `std::io` module.

use std::fmt::{self, Write as _};
use std::io::{self, Write as _};

use crate::no_std::prelude::*;

use crate as rune;
use crate::compile;
use crate::macros::{quote, FormatArgs, MacroContext, TokenStream};
use crate::parse::Parser;
use crate::runtime::{Panic, Protocol, Stack, Value, VmResult};
use crate::{ContextError, Module};

/// Construct the `std::io` module.
pub fn module(stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["io"]).with_unique("std::io");

    module.item_mut().docs([
        "The std::io module contains a number of common things",
        "you’ll need when doing input and output.",
        "The most core parts of this module are the [print()], [println()], ",
        "and [dbg()] functions which are used to hook up printing for a Rune project.",
        "",
        "With complete names:",
        "* `::std::io::print`",
        "* `::std::io::println`",
        "* `::std::io::dbg`",
        "",
        "Their definitions can be omitted from the built-in standard library, and",
        "can then easily be defined by third party modules allowing for printing",
        "to be hooked up to whatever system you want.",
    ]);

    module.ty::<io::Error>()?;
    module.associated_function(Protocol::STRING_DISPLAY, format_io_error)?;

    if stdio {
        module.function_meta(print_impl)?;
        module.function_meta(println_impl)?;

        module.raw_fn(["dbg"], dbg_impl)?.docs([
            "Debug to output.",
            "",
            "This is the actual output hook, and if you install rune modules without",
            "`I/O` enabled this will not be defined. It is then up to someone else to",
            "provide an implementation.",
            "",
            "# Examples",
            "",
            "```rune",
            "let number = 10;",
            "let number = number * 4;",
            "",
            "let who = \"World\";",
            "let string = format!(\"Hello {}\", who);",
            "",
            "dbg(number, string);",
            "```",
        ]);
    }

    // These are unconditionally included, but using them might cause a
    // compilation error unless `::std::io::*` functions are provided somehow.
    module.macro_meta(dbg_macro)?;
    module.macro_meta(print_macro)?;
    module.macro_meta(println_macro)?;
    Ok(module)
}

fn format_io_error(error: &io::Error, buf: &mut String) -> fmt::Result {
    write!(buf, "{}", error)
}

fn dbg_impl(stack: &mut Stack, args: usize) -> VmResult<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for value in vm_try!(stack.drain(args)) {
        vm_try!(writeln!(stdout, "{:?}", value).map_err(Panic::custom));
    }

    stack.push(Value::Unit);
    VmResult::Ok(())
}

/// Debug print the given argument.
///
/// Everything in rune can be "debug printed" in one way or another. This is
/// provided as a cheap an dirty way to introspect values.
///
/// See also the [`dbg!`] macro.
///
/// # Examples
///
/// ```rune
/// let number = 10;
/// let number = number * 4;
///
/// let who = "World";
/// let string = format!("Hello {}", who);
///
/// dbg!(number, string);
/// ```
#[rune::macro_(path = dbg)]
pub(crate) fn dbg_macro(
    cx: &mut MacroContext<'_, '_, '_>,
    stream: &TokenStream,
) -> compile::Result<TokenStream> {
    Ok(quote!(::std::io::dbg(#stream)).into_token_stream(cx))
}

/// Prints to output.
///
/// Output printing is performed by calling the [`print()`] function, this is
/// just a convenience wrapper around it which allows for formatting.
///
/// # Examples
///
/// ```rune
/// let who = "World";
/// print!("Hello {}!", who);
/// ```
#[rune::macro_(path = print)]
pub(crate) fn print_macro(
    cx: &mut MacroContext<'_, '_, '_>,
    stream: &TokenStream,
) -> compile::Result<TokenStream> {
    let mut p = Parser::from_token_stream(stream, cx.input_span());
    let args = p.parse_all::<FormatArgs>()?;
    let expanded = args.expand(cx)?;
    Ok(quote!(::std::io::print(#expanded)).into_token_stream(cx))
}

/// Prints to output.
///
/// This is the actual output hook, and if you install rune modules without
/// `I/O` enabled this will not be defined. It is then up to someone else to
/// provide an implementation.
///
/// See also the [`print!`] macro.
///
/// # Examples
///
/// ```rune
/// print("Hi!");
/// ```
#[rune::function(path = print)]
fn print_impl(m: &str) -> VmResult<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    if let Err(error) = write!(stdout, "{}", m) {
        return VmResult::err(Panic::custom(error));
    }

    VmResult::Ok(())
}

/// Prints to output, with a newline.
///
/// Output printing is performed by calling the [`println()`] function, this is
/// just a convenience wrapper around it which allows for formatting.
///
/// # Examples
///
/// ```rune
/// let who = "World";
/// println!("Hello {}!", who);
/// ```
#[rune::macro_(path = println)]
pub(crate) fn println_macro(
    cx: &mut MacroContext<'_, '_, '_>,
    stream: &TokenStream,
) -> compile::Result<TokenStream> {
    let mut p = Parser::from_token_stream(stream, cx.input_span());
    let args = p.parse_all::<FormatArgs>()?;
    let expanded = args.expand(cx)?;
    Ok(quote!(::std::io::println(#expanded)).into_token_stream(cx))
}

/// Prints to output, with a newline.
///
/// This is the actual output hook, and if you install rune modules without
/// `I/O` enabled this will not be defined. It is then up to someone else to
/// provide an implementation.
///
/// # Examples
///
/// ```rune
/// println("Hi!");
/// ```
#[rune::function(path = println)]
fn println_impl(message: &str) -> VmResult<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    if let Err(error) = writeln!(stdout, "{}", message) {
        return VmResult::err(Panic::custom(error));
    }

    VmResult::Ok(())
}
