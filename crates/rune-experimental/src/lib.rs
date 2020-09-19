//! <div align="center">
//!     <img alt="Rune Logo" src="https://raw.githubusercontent.com/rune-rs/rune/master/assets/icon.png" />
//! </div>
//!
//! <br>
//!
//! <div align="center">
//! <a href="https://rune-rs.github.io">
//!     <b>Visit the site 🌐</b>
//! </a>
//! -
//! <a href="https://rune-rs.github.io/bool/">
//!     <b>Read the book 📖</b>
//! </a>
//! </div>
//!
//! <br>
//!
//! <div align="center">
//! <a href="https://github.com/rune-rs/rune/actions">
//!     <img alt="Build Status" src="https://github.com/rune-rs/rune/workflows/Build/badge.svg">
//! </a>
//!
//! <a href="https://github.com/rune-rs/rune/actions">
//!     <img alt="Site Status" src="https://github.com/rune-rs/rune/workflows/Site/badge.svg">
//! </a>
//!
//! <a href="https://crates.io/crates/rune">
//!     <img alt="crates.io" src="https://img.shields.io/crates/v/rune.svg">
//! </a>
//!
//! <a href="https://docs.rs/rune">
//!     <img alt="docs.rs" src="https://docs.rs/rune/badge.svg">
//! </a>
//!
//! <a href="https://discord.gg/v5AeNkT">
//!     <img alt="Chat on Discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square">
//! </a>
//! </div>
//!
//! Native macros for Rune.

use rune::ast;
use rune::{MacroContext, Parser, TokenStream};

mod stringy_math_macro;

/// Implementation for the `passthrough!` macro.
fn passthrough_impl(_: &mut MacroContext, stream: &TokenStream) -> runestick::Result<TokenStream> {
    Ok(stream.clone())
}

/// Implementation for the `make_function!` macro.
fn make_function(ctx: &mut MacroContext, stream: &TokenStream) -> runestick::Result<TokenStream> {
    let mut parser = Parser::from_token_stream(stream);

    let ident = parser.parse::<ast::Ident>()?;
    let _ = parser.parse::<ast::Rocket>()?;
    let output = parser.parse::<ast::ExprBlock>()?;
    parser.parse_eof()?;

    Ok(rune::quote!(ctx => fn #ident() { #output }))
}

/// Construct the `std::experimental` module, which contains experimental
/// macros.
pub fn module() -> Result<runestick::Module, runestick::ContextError> {
    let mut module = runestick::Module::new(&["std", "experiments"]);
    module.macro_(&["passthrough"], passthrough_impl)?;
    module.macro_(&["stringy_math"], stringy_math_macro::stringy_math)?;
    module.macro_(&["make_function"], make_function)?;
    Ok(module)
}
