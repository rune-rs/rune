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
//! context.install(&rune_modules::core::module()?)?;
//! # Ok(())
//! # }
//! ```

use rune::ast;
use rune::macros;
use rune::{quote, Parser, TokenStream};

/// Construct the `std::core` module.
pub fn module() -> Result<runestick::Module, runestick::ContextError> {
    let mut module = runestick::Module::new(&["std", "core"]);
    module.macro_(&["stringify"], stringify_macro)?;
    module.macro_(&["println"], println_macro)?;
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

/// Implementation for the `println!` macro.
pub(crate) fn println_macro(
    stream: &TokenStream,
) -> runestick::Result<TokenStream> {
    let mut parser = Parser::from_token_stream(stream);
    let expr = parser.parse::<ast::Expr>()?;
    let _ = macros::eval(&expr)?;
    parser.eof()?;
    Ok(quote!().into_token_stream())
}
