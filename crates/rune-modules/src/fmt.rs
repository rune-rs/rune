//! `std::fmt` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = {version = "0.9.1", features = ["fmt"]}
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> runestick::Result<()> {
//! let mut context = runestick::Context::with_default_modules()?;
//! context.install(&rune_modules::fmt::module(true)?)?;
//! # Ok(())
//! # }
//! ```

use rune::{macros, MacroContext, Parser, TokenStream};

/// Construct the supplemental `std::io` module.
pub fn module(_stdio: bool) -> Result<runestick::Module, runestick::ContextError> {
    let mut module = runestick::Module::with_crate_item("std", &["fmt"]);
    module.macro_(&["format"], format_macro)?;
    Ok(module)
}

/// Implementation for the `format!` macro.
pub(crate) fn format_macro(ctx: &mut MacroContext<'_>, stream: &TokenStream) -> runestick::Result<TokenStream> {
    let mut p = Parser::from_token_stream(stream, ctx.stream_span());
    let args = p.parse::<macros::FormatArgs>()?;
    p.eof()?;
    let expanded = args.expand(ctx)?;
    Ok(expanded.into_token_stream(ctx))
}
