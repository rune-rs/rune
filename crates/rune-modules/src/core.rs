//! `std::experiments` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = {version = "0.9.1", features = ["test"]}
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> runestick::Result<()> {
//! let mut context = runestick::Context::with_default_modules()?;
//! context.install(&rune_modules::core::module(true)?)?;
//! # Ok(())
//! # }
//! ```

use rune::ast;
use rune::macros;
use rune::{quote, Parser, MacroContext, TokenStream};

/// Construct the `std::core` module.
pub fn module(_stdio: bool) -> Result<runestick::Module, runestick::ContextError> {
    let mut module = runestick::Module::with_crate("std");
    module.macro_(&["stringify"], stringify_macro)?;
    module.macro_(&["panic"], panic_macro)?;
    Ok(module)
}

/// Implementation for the `stringify!` macro.
pub(crate) fn stringify_macro(
    ctx: &mut MacroContext<'_>,
    stream: &TokenStream,
) -> runestick::Result<TokenStream> {
    let lit = ctx.stringify(stream).to_string();
    let lit = ast::Lit::new(ctx, lit);
    Ok(quote!(#lit).into_token_stream(ctx))
}

pub(crate) fn panic_macro(
    ctx: &mut MacroContext<'_>,
    stream: &TokenStream,
) -> runestick::Result<TokenStream> {
    let mut p = Parser::from_token_stream(stream, ctx.stream_span());
    let args = p.parse_all::<macros::FormatArgs>()?;
    let expanded = args.expand(ctx)?;
    Ok(quote!(::std::panic(#expanded)).into_token_stream(ctx))
}
