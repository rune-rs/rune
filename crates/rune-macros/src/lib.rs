//! <div align="center">
//! <a href="https://rune-rs.github.io/rune/">
//!     <b>Read the Book 📖</b>
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
//!     <img alt="Book Status" src="https://github.com/rune-rs/rune/workflows/Book/badge.svg">
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

use rune::{MacroContext, TokenStream};

/// Implementation for the `passthrough!` macro.
fn passthrough_impl(_: &mut MacroContext, stream: &TokenStream) -> runestick::Result<TokenStream> {
    Ok(stream.clone())
}

/// Implementation for the `test_add!` macro.
fn test_add(context: &mut MacroContext, stream: &TokenStream) -> runestick::Result<TokenStream> {
    use rune::ast;
    use rune::Resolve as _;

    let mut parser = rune::Parser::from_token_stream(stream);

    let ident = parser.parse::<ast::Ident>()?;
    let var = parser.parse::<ast::Ident>()?;
    parser.parse_eof()?;

    let ident = ident.resolve(context.source())?;

    if ident != "please" {
        return Err(runestick::Error::msg("you didn't ask nicely..."));
    }

    let mut output = TokenStream::default();

    output.extend(var);
    output.extend(ast::Kind::Plus);
    output.extend(var);

    Ok(output)
}

/// Construct the `http` module.
pub fn module() -> Result<runestick::Module, runestick::ContextError> {
    let mut module = runestick::Module::new(&["std", "experiments"]);
    module.macro_(&["passthrough"], passthrough_impl)?;
    module.macro_(&["test_add"], test_add)?;
    Ok(module)
}
