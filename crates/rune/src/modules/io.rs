//! I/O functions.

#[cfg(feature = "std")]
use std::io::{self, Write as _};

use crate as rune;
#[cfg(feature = "std")]
use crate::alloc::fmt::TryWrite;
use crate::compile;
use crate::macros::{quote, FormatArgs, MacroContext, TokenStream};
use crate::parse::Parser;
#[cfg(feature = "std")]
use crate::runtime::{Formatter, InstAddress, Memory, Output, Panic, VmResult};
use crate::{ContextError, Module};

/// I/O functions.
#[rune::module(::std::io)]
pub fn module(
    #[cfg_attr(not(feature = "std"), allow(unused))] stdio: bool,
) -> Result<Module, ContextError> {
    let mut module = Module::from_meta(self::module_meta)?.with_unique("std::io");

    module.item_mut().docs(docstring! {
        /// The std::io module contains a number of common things
        /// you’ll need when doing input and output.
        /// The most core parts of this module are the [print()], [println()],
        /// and [dbg()] functions which are used to hook up printing for a Rune project.
        ///
        /// With complete names:
        /// * `::std::io::print`
        /// * `::std::io::println`
        /// * `::std::io::dbg`
        ///
        /// Their definitions can be omitted from the built-in standard library, and
        /// can then easily be defined by third party modules allowing for printing
        /// to be hooked up to whatever system you want.
    })?;

    #[cfg(feature = "std")]
    module.ty::<io::Error>()?;
    #[cfg(feature = "std")]
    module.function_meta(io_error_display_fmt)?;
    #[cfg(feature = "std")]
    module.function_meta(io_error_debug_fmt)?;

    #[cfg(feature = "std")]
    if stdio {
        module.function_meta(print_impl)?;
        module.function_meta(println_impl)?;

        module
            .raw_function("dbg", dbg_impl)
            .build()?
            .docs(docstring! {
                /// Debug to output.
                ///
                /// This is the actual output hook, and if you install rune modules without
                /// `I/O` enabled this will not be defined. It is then up to someone else to
                /// provide an implementation.
                ///
                /// # Examples
                ///
                /// ```rune
                /// let number = 10;
                /// let number = number * 4;
                ///
                /// let who = "World";
                /// let string = format!("Hello {who}");
                ///
                /// dbg(number, string);
                /// ```
            })?;
    }

    // These are unconditionally included, but using them might cause a
    // compilation error unless `::std::io::*` functions are provided somehow.
    module.macro_meta(dbg_macro)?;
    module.macro_meta(print_macro)?;
    module.macro_meta(println_macro)?;
    Ok(module)
}

#[rune::function(instance, protocol = DISPLAY_FMT)]
#[cfg(feature = "std")]
fn io_error_display_fmt(error: &io::Error, f: &mut Formatter) -> VmResult<()> {
    vm_write!(f, "{error}")
}

#[rune::function(instance, protocol = DEBUG_FMT)]
#[cfg(feature = "std")]
fn io_error_debug_fmt(error: &io::Error, f: &mut Formatter) -> VmResult<()> {
    vm_write!(f, "{error:?}")
}

#[cfg(feature = "std")]
fn dbg_impl(stack: &mut dyn Memory, addr: InstAddress, args: usize, out: Output) -> VmResult<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for value in vm_try!(stack.slice_at(addr, args)) {
        vm_try!(writeln!(stdout, "{:?}", value).map_err(Panic::custom));
    }

    vm_try!(out.store(stack, ()));
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
    Ok(quote!(::std::io::dbg(#stream)).into_token_stream(cx)?)
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
    Ok(quote!(::std::io::print(#expanded)).into_token_stream(cx)?)
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
#[cfg(feature = "std")]
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
    Ok(quote!(::std::io::println(#expanded)).into_token_stream(cx)?)
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
#[cfg(feature = "std")]
fn println_impl(message: &str) -> VmResult<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    if let Err(error) = writeln!(stdout, "{}", message) {
        return VmResult::err(Panic::custom(error));
    }

    VmResult::Ok(())
}
