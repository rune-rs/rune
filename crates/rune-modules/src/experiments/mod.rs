//! `std::experiments` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = { version = "0.12.1", features = ["experiments"] }
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> rune::Result<()> {
//! let mut context = rune::Context::with_default_modules()?;
//! context.install(&rune_modules::experiments::module(true)?)?;
//! # Ok(())
//! # }
//! ```

use rune::ast;
use rune::macros::{quote, MacroContext, TokenStream};
use rune::parse::Parser;
use rune::T;
use rune::{ContextError, Module};

mod stringy_math_macro;

/// Construct the `std::experiments` module, which contains experiments.
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["experiments"]);
    module.macro_(["passthrough"], passthrough_impl)?;
    module.macro_(["stringy_math"], stringy_math_macro::stringy_math)?;
    module.macro_(["make_function"], make_function)?;
    Ok(module)
}

/// Implementation for the `passthrough!` macro.
fn passthrough_impl(_: &mut MacroContext<'_>, stream: &TokenStream) -> rune::Result<TokenStream> {
    Ok(stream.clone())
}

/// Implementation for the `make_function!` macro.
fn make_function(ctx: &mut MacroContext<'_>, stream: &TokenStream) -> rune::Result<TokenStream> {
    let mut parser = Parser::from_token_stream(stream, ctx.stream_span());

    let ident = parser.parse::<ast::Ident>()?;
    let _ = parser.parse::<T![=>]>()?;
    let output = parser.parse::<ast::ExprBlock>()?;
    parser.eof()?;

    Ok(quote!(fn #ident() { #output }).into_token_stream(ctx))
}
