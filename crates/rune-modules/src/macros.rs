//! `std::macros` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = {version = "0.9.1", features = ["macros"]}
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> runestick::Result<()> {
//! let mut context = runestick::Context::with_default_modules()?;
//! context.install(&rune_modules::macros::module(true)?)?;
//! # Ok(())
//! # }
//! ```
//!
//! Use it in Rune:
//!
//! ```rust,ignore
//! fn main() {
//!     println(`Hello from ${file!()}:${line!()});
//! }
//! ```

use rune::{Parser, TokenStream};

/// Construct the supplemental `std::macros` module.
pub fn module(_unused: bool) -> Result<runestick::Module, runestick::ContextError> {
    let mut builtins = runestick::Module::with_crate_item("std", &["macros", "builtin"]);
    builtins.macro_(&["file"], emit_file)?;
    builtins.macro_(&["line"], emit_line)?;
    Ok(builtins)
}

/// Implementation for the `line!()` macro
pub(crate) fn emit_line(stream: &TokenStream) -> runestick::Result<TokenStream> {
    let mut parser = Parser::from_token_stream(stream);

    parser.eof()?;

    Ok(rune::quote!(
        #[builtin]
        line!()
    )
    .into_token_stream())
}

/// Implementation for the `file!()` macro
pub(crate) fn emit_file(stream: &TokenStream) -> runestick::Result<TokenStream> {
    let mut parser = Parser::from_token_stream(stream);

    parser.eof()?;

    Ok(rune::quote!(
        #[builtin]
        file!()
    )
    .into_token_stream())
}
