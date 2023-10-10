//! `std::experiments` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = { version = "0.13.1", features = ["experiments"] }
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! let mut context = rune::Context::with_default_modules()?;
//! context.install(rune_modules::experiments::module(true)?)?;
//! # Ok::<_, rune::support::Error>(())
//! ```

use rune::alloc::prelude::*;
use rune::ast;
use rune::compile;
use rune::macros::{quote, MacroContext, TokenStream};
use rune::parse::Parser;
use rune::T;
use rune::{ContextError, Module};

mod stringy_math_macro;

/// Construct the `std::experiments` module, which contains experiments.
pub fn module(_stdio: bool) -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["experiments"])?;
    module.macro_meta(passthrough)?;
    module.macro_meta(stringy_math_macro::stringy_math)?;
    module.macro_meta(make_function)?;
    Ok(module)
}

/// Implementation for the `passthrough!` macro.
#[rune::macro_]
fn passthrough(
    _: &mut MacroContext<'_, '_, '_>,
    stream: &TokenStream,
) -> compile::Result<TokenStream> {
    Ok(stream.try_clone()?)
}

/// Implementation for the `make_function!` macro.
#[rune::macro_]
fn make_function(
    cx: &mut MacroContext<'_, '_, '_>,
    stream: &TokenStream,
) -> compile::Result<TokenStream> {
    let mut parser = Parser::from_token_stream(stream, cx.input_span());

    let ident = parser.parse::<ast::Ident>()?;
    let _ = parser.parse::<T![=>]>()?;
    let output = parser.parse::<ast::ExprBlock>()?;
    parser.eof()?;

    Ok(quote!(fn #ident() { #output }).into_token_stream(cx)?)
}
