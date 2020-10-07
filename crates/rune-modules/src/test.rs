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
//! context.install(&rune_modules::test::module()?)?;
//! # Ok(())
//! # }
//! ```

use rune::T;
use rune::ast;
use rune::macros::stringify;
use rune::{quote, Parser, TokenStream};

/// Construct the `std::test` module.
pub fn module() -> Result<runestick::Module, runestick::ContextError> {
    let mut module = runestick::Module::new(&["std", "test"]);
    module.macro_(&["assert"], assert_macro)?;
    module.macro_(&["assert_eq"], assert_eq_macro)?;
    Ok(module)
}

/// Implementation for the `assert!` macro.
pub(crate) fn assert_macro(
    stream: &TokenStream,
) -> runestick::Result<TokenStream> {
    let mut parser = Parser::from_token_stream(stream);
    let expr = parser.parse::<ast::Expr>()?;

    let comma = parser.parse::<Option<T![,]>>()?;

    let message = if comma.is_some() {
        parser.parse::<Option<ast::Expr>>()?
    } else {
        None
    };

    let output = if let Some(message) = message {
        quote!(if !(#expr) {
            panic("assertion failed: " + #message);
        })
    } else {
        let message = format!("assertion failed: {}", stringify(&expr));
        let message = ast::Lit::new(&message);

        quote!(if !(#expr) {
            panic(#message);
        })
    };

    parser.eof()?;
    Ok(output.into_token_stream())
}

/// Implementation for the `assert!` macro.
pub(crate) fn assert_eq_macro(
    stream: &TokenStream,
) -> runestick::Result<TokenStream> {
    let mut parser = Parser::from_token_stream(stream);
    let left = parser.parse::<ast::Expr>()?;
    parser.parse::<T![,]>()?;
    let right = parser.parse::<ast::Expr>()?;

    let comma = parser.parse::<Option<T![,]>>()?;

    let message = if comma.is_some() {
        parser.parse::<Option<ast::Expr>>()?
    } else {
        None
    };

    let expr = quote!(#left == #right);

    let output = if let Some(message) = message {
        quote!(if !(#expr) {
            panic("assertion failed (left == right): " + #message);
        })
    } else {
        let message = format!("assertion failed (left == right): {}", stringify(&expr));
        let message = ast::Lit::new(&message);

        quote!(if !(#expr) {
            panic(#message);
        })
    };

    parser.eof()?;
    Ok(output.into_token_stream())
}
