//! `std::experiments` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = {version = "0.6.16", features = ["test"]}
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
use rune::{quote, TokenStream};

/// Construct the `std::core` module.
pub fn module(_stdio: bool) -> Result<runestick::Module, runestick::ContextError> {
    let mut module = runestick::Module::new(&["std", "core"]);
    module.macro_(&["stringify"], stringify_macro)?;
    Ok(module)
}

/// Implementation for the `stringify!` macro.
pub(crate) fn stringify_macro(
    stream: &TokenStream,
) -> runestick::Result<TokenStream> {
    let lit = macros::stringify(stream);
    let lit = ast::Lit::new(lit);
    Ok(quote!(#lit).into_token_stream())
}
