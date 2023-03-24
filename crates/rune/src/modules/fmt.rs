//! The `std::fmt` module.

use std::fmt;
use std::fmt::Write;

use crate::macros::{FormatArgs, MacroContext, TokenStream};
use crate::parse::Parser;
use crate::runtime::{Format, Protocol};
use crate::{ContextError, Module};

/// Construct the `std::fmt` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["fmt"]).with_unique("std::fmt");
    module.ty::<std::fmt::Error>()?;
    module.inst_fn(Protocol::STRING_DISPLAY, format_fmt_error)?;
    module.macro_(["format"], format_macro)?;

    module.ty::<Format>()?;
    Ok(module)
}

fn format_fmt_error(error: &std::fmt::Error, buf: &mut String) -> fmt::Result {
    write!(buf, "{}", error)
}

/// Implementation for the `format!` macro.
pub(crate) fn format_macro(
    ctx: &mut MacroContext<'_>,
    stream: &TokenStream,
) -> crate::Result<TokenStream> {
    let mut p = Parser::from_token_stream(stream, ctx.stream_span());
    let args = p.parse::<FormatArgs>()?;
    p.eof()?;
    let expanded = args.expand(ctx)?;
    Ok(expanded.into_token_stream(ctx))
}
