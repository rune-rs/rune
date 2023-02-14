//! `std::io` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = { version = "0.12.1", features = ["io"] }
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> rune::Result<()> {
//! let mut context = rune::Context::with_default_modules()?;
//! context.install(&rune_modules::io::module(true)?)?;
//! # Ok(())
//! # }
//! ```

use rune::parse::Parser;
use rune::{Module, ContextError};
use rune::macros::{MacroContext, TokenStream, quote, FormatArgs};

/// Construct the supplemental `std::io` module.
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", &["io"]);
    module.macro_(&["println"], println_macro)?;
    Ok(module)
}

/// Implementation for the `println!` macro.
pub(crate) fn println_macro(ctx: &mut MacroContext<'_>, stream: &TokenStream) -> rune::Result<TokenStream> {
    let mut p = Parser::from_token_stream(stream, ctx.stream_span());
    let args = p.parse_all::<FormatArgs>()?;
    let expanded = args.expand(ctx)?;
    Ok(quote!(std::io::println(#expanded)).into_token_stream(ctx))
}
