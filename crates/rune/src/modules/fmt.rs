//! The `std::fmt` module.

use core::fmt::{self, Write};

use crate::no_std::prelude::*;

use crate as rune;
use crate::compile;
use crate::macros::{FormatArgs, MacroContext, TokenStream};
use crate::parse::Parser;
use crate::runtime::{Format, Protocol};
use crate::{ContextError, Module};

/// Construct the `std::fmt` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["fmt"]).with_unique("std::fmt");
    module.ty::<fmt::Error>()?;
    module.inst_fn(Protocol::STRING_DISPLAY, format_fmt_error)?;
    module.macro_meta(format)?;

    module.ty::<Format>()?;
    Ok(module)
}

fn format_fmt_error(error: &fmt::Error, buf: &mut String) -> fmt::Result {
    write!(buf, "{}", error)
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
    ctx: &mut MacroContext<'_>,
    stream: &TokenStream,
) -> compile::Result<TokenStream> {
    let mut p = Parser::from_token_stream(stream, ctx.stream_span());
    let args = p.parse::<FormatArgs>()?;
    p.eof()?;
    let expanded = args.expand(ctx)?;
    Ok(expanded.into_token_stream(ctx))
}
