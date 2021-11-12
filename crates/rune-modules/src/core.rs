//! `std::experiments` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = { version = "0.10.0", features = ["test"] }
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> rune::Result<()> {
//! let mut context = rune::Context::with_default_modules()?;
//! context.install(&rune_modules::core::module(true)?)?;
//! # Ok(())
//! # }
//! ```

use rune::ast;
use rune::parse::Parser;
use rune::{Module, ContextError};
use rune::macros::{quote, FormatArgs, MacroContext, TokenStream};

/// Construct the `std::core` module.
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate("std");
    module.macro_(&["stringify"], stringify_macro)?;
    module.macro_(&["panic"], panic_macro)?;
    Ok(module)
}

/// Implementation for the `stringify!` macro.
pub(crate) fn stringify_macro(
    ctx: &mut MacroContext<'_, '_>,
    stream: &TokenStream,
) -> rune::Result<TokenStream> {
    let lit = ctx.stringify(stream).to_string();
    let lit = ast::Lit::new(ctx, lit);
    Ok(quote!(#lit).into_token_stream(ctx))
}

pub(crate) fn panic_macro(
    ctx: &mut MacroContext<'_, '_>,
    stream: &TokenStream,
) -> rune::Result<TokenStream> {
    let mut p = Parser::from_token_stream(stream, ctx.stream_span());
    let args = p.parse_all::<FormatArgs>()?;
    let expanded = args.expand(ctx)?;
    Ok(quote!(::std::panic(#expanded)).into_token_stream(ctx))
}
