//! `std::experiments` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = {version = "0.6.16", features = ["io"]}
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> runestick::Result<()> {
//! let mut context = runestick::Context::with_default_modules()?;
//! context.install(&rune_modules::io::module(true)?)?;
//! # Ok(())
//! # }
//! ```

use rune::macros;
use rune::{quote, Parser, TokenStream};

/// Construct the supplemental `std::io` module.
pub fn module(_stdio: bool) -> Result<runestick::Module, runestick::ContextError> {
    let mut module = runestick::Module::new(&["std", "io"]);
    module.macro_(&["println"], println_macro)?;
    Ok(module)
}

/// Implementation for the `println!` macro.
pub(crate) fn println_macro(stream: &TokenStream) -> runestick::Result<TokenStream> {
    let mut p = Parser::from_token_stream(stream);
    let args = p.parse::<macros::FormatArgs>()?;
    p.eof()?;
    let expanded = args.expand()?;
    Ok(quote!(std::io::println(#expanded)).into_token_stream())
}
