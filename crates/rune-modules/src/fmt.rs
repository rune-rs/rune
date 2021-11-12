//! `std::fmt` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = { version = "0.10.0", features = ["fmt"] }
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> rune::Result<()> {
//! let mut context = rune::Context::with_default_modules()?;
//! context.install(&rune_modules::fmt::module(true)?)?;
//! # Ok(())
//! # }
//! ```

use rune::parsing::Parser;
use rune::{Module, ContextError};
use rune::macros::{MacroContext, TokenStream, FormatArgs};

/// Construct the supplemental `std::io` module.
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["fmt"]);
    module.macro_(&["format"], format_macro)?;
    Ok(module)
}

/// Implementation for the `format!` macro.
pub(crate) fn format_macro(ctx: &mut MacroContext<'_, '_>, stream: &TokenStream) -> rune::Result<TokenStream> {
    let mut p = Parser::from_token_stream(stream, ctx.stream_span());
    let args = p.parse::<FormatArgs>()?;
    p.eof()?;
    let expanded = args.expand(ctx)?;
    Ok(expanded.into_token_stream(ctx))
}
