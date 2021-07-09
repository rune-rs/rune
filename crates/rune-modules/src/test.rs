//! `std::experiments` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = {version = "0.9.0", features = ["test"]}
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

use rune::ast;
use rune::macros;
use rune::T;
use rune::{quote, Parser, TokenStream};

/// Construct the `std::test` module.
pub fn module(_stdio: bool) -> Result<runestick::Module, runestick::ContextError> {
    let mut module = runestick::Module::with_crate_item("std", &["test"]);
    module.macro_(&["assert"], assert_macro)?;
    module.macro_(&["assert_eq"], assert_eq_macro)?;
    Ok(module)
}

/// Implementation for the `assert!` macro.
pub(crate) fn assert_macro(stream: &TokenStream) -> runestick::Result<TokenStream> {
    let mut p = Parser::from_token_stream(stream);
    let expr = p.parse::<ast::Expr>()?;

    let message = if p.parse::<Option<T![,]>>()?.is_some() {
        p.parse_all::<Option<macros::FormatArgs>>()?
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

    Ok(output.into_token_stream())
}

/// Implementation for the `assert!` macro.
pub(crate) fn assert_eq_macro(stream: &TokenStream) -> runestick::Result<TokenStream> {
    let mut p = Parser::from_token_stream(stream);
    let left = p.parse::<ast::Expr>()?;
    p.parse::<T![,]>()?;
    let right = p.parse::<ast::Expr>()?;

    let message = if p.parse::<Option<T![,]>>()?.is_some() {
        p.parse_all::<Option<macros::FormatArgs>>()?
    } else {
        None
    };

    let output = if let Some(message) = &message {
        let message = message.expand()?;

        quote! {{
            let left = #left;
            let right = #right;

            if !(left == right) {
                let message = #message;
                message += format!("\nleft: {:?}", left);
                message += format!("\nright: {:?}", right);
                panic("assertion failed (left == right): " + message);
            }
        }}
    } else {
        let message = "assertion failed (left == right):".to_string();
        let message = ast::Lit::new(&message);

        quote! {{
            let left = #left;
            let right = #right;

            if !(left == right) {
                let message = String::from_str(#message);
                message += format!("\nleft: {:?}", left);
                message += format!("\nright: {:?}", right);
                panic(message);
            }
        }}
    };

    Ok(output.into_token_stream())
}
