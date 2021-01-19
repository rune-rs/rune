//! `std::experiments` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = {version = "0.8.0", features = ["experiments"]}
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> runestick::Result<()> {
//! let mut context = runestick::Context::with_default_modules()?;
//! context.install(&rune_modules::experiments::module(true)?)?;
//! # Ok(())
//! # }
//! ```

use rune::ast;
use rune::T;
use rune::{Parser, TokenStream};

mod stringy_math_macro;

/// Implementation for the `passthrough!` macro.
fn passthrough_impl(stream: &TokenStream) -> runestick::Result<TokenStream> {
    Ok(stream.clone())
}

/// Implementation for the `make_function!` macro.
fn make_function(stream: &TokenStream) -> runestick::Result<TokenStream> {
    let mut parser = Parser::from_token_stream(stream);

    let ident = parser.parse::<ast::Ident>()?;
    let _ = parser.parse::<T![=>]>()?;
    let output = parser.parse::<ast::ExprBlock>()?;
    parser.eof()?;

    Ok(rune::quote!(fn #ident() { #output }).into_token_stream())
}

/// Construct the `std::experiments` module, which contains experiments.
pub fn module(_stdio: bool) -> Result<runestick::Module, runestick::ContextError> {
    let mut module = runestick::Module::with_crate_item("std", &["experiments"]);
    module.macro_(&["passthrough"], passthrough_impl)?;
    module.macro_(&["stringy_math"], stringy_math_macro::stringy_math)?;
    module.macro_(&["make_function"], make_function)?;
    Ok(module)
}
