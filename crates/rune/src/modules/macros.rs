//! `std::macros` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io

use crate::compile;
use crate::macros::{quote, MacroContext, TokenStream};
use crate::parse::Parser;
use crate::{ContextError, Module};

/// Construct the `std::macros` module.
pub fn module() -> Result<Module, ContextError> {
    let mut builtins =
        Module::with_crate_item("std", ["macros", "builtin"]).with_unique("std::macros::builtin");
    builtins.macro_(["file"], emit_file)?;
    builtins.macro_(["line"], emit_line)?;
    Ok(builtins)
}

/// Implementation for the `line!()` macro
pub(crate) fn emit_line(
    ctx: &mut MacroContext<'_>,
    stream: &TokenStream,
) -> compile::Result<TokenStream> {
    use crate as rune;

    let mut parser = Parser::from_token_stream(stream, ctx.stream_span());
    parser.eof()?;

    Ok(quote!(
        #[builtin]
        line!()
    )
    .into_token_stream(ctx))
}

/// Implementation for the `file!()` macro
pub(crate) fn emit_file(
    ctx: &mut MacroContext<'_>,
    stream: &TokenStream,
) -> compile::Result<TokenStream> {
    use crate as rune;

    let mut parser = Parser::from_token_stream(stream, ctx.stream_span());
    parser.eof()?;

    Ok(quote!(
        #[builtin]
        file!()
    )
    .into_token_stream(ctx))
}
