//! `std::fmt` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = {version = "0.7.0", features = ["fmt"]}
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

use rune::macros;
use rune::{Parser, TokenStream};

/// Construct the supplemental `std::io` module.
pub fn module(_stdio: bool) -> Result<runestick::Module, runestick::ContextError> {
    let mut module = runestick::Module::new(&["std", "fmt"]);
    module.macro_(&["format"], format_macro)?;
    Ok(module)
}

/// Implementation for the `format!` macro.
pub(crate) fn format_macro(stream: &TokenStream) -> runestick::Result<TokenStream> {
    let mut p = Parser::from_token_stream(stream);
    let args = p.parse::<macros::FormatArgs>()?;
    p.eof()?;
    let expanded = args.expand()?;
    Ok(expanded.into_token_stream())
}
