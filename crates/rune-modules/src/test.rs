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
//! context.install(&rune_modules::test::module(true)?)?;
//! # Ok(())
//! # }
//! ```

use rune::T;
use rune::ast;
use rune::macros;
use rune::{quote, Parser, TokenStream};

/// Construct the `std::test` module.
pub fn module(_stdio: bool) -> Result<runestick::Module, runestick::ContextError> {
    let mut module = runestick::Module::new(&["std", "test"]);
    module.macro_(&["assert"], assert_macro)?;
    module.macro_(&["assert_eq"], assert_eq_macro)?;
    Ok(module)
}

/// Implementation for the `assert!` macro.
pub(crate) fn assert_macro(
    stream: &TokenStream,
) -> runestick::Result<TokenStream> {
    let mut p = Parser::from_token_stream(stream);
    let expr = p.parse::<ast::Expr>()?;

    let message = if p.parse::<Option<T![,]>>()?.is_some() {
        p.parse::<Option<macros::FormatArgs>>()?
    } else {
        None
    };

    let output = if let Some(message) = &message {
        let expanded = message.expand()?;

        quote!(if !(#expr) {
            panic("assertion failed: " + (#expanded));
        })
    } else {
        let message = format!("assertion failed: {}", macros::stringify(&expr));
        let message = ast::Lit::new(&message);

        quote!(if !(#expr) {
            panic(#message);
        })
    };

    p.eof()?;
    Ok(output.into_token_stream())
}

/// Implementation for the `assert!` macro.
pub(crate) fn assert_eq_macro(
    stream: &TokenStream,
) -> runestick::Result<TokenStream> {
    let mut p = Parser::from_token_stream(stream);
    let left = p.parse::<ast::Expr>()?;
    p.parse::<T![,]>()?;
    let right = p.parse::<ast::Expr>()?;

    let message = if p.parse::<Option<T![,]>>()?.is_some() {
        p.parse::<Option<macros::FormatArgs>>()?
    } else {
        None
    };

    let expr = quote!(#left == #right);

    let output = if let Some(message) = &message {
        let expanded = message.expand()?;

        quote!(if !(#expr) {
            panic("assertion failed (left == right): " + (#expanded));
        })
    } else {
        let message = format!("assertion failed (left == right): {}", macros::stringify(&expr));
        let message = ast::Lit::new(&message);

        quote!(if !(#expr) {
            panic(#message);
        })
    };

    p.eof()?;
    Ok(output.into_token_stream())
}
