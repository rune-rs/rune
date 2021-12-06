//! `std::experiments` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! rune-modules = { version = "0.10.1", features = ["test"] }
//! ```
//!
//! Install it into your context:
//!
//! ```rust
//! # fn main() -> rune::Result<()> {
//! let mut context = rune::Context::with_default_modules()?;
//! context.install(&rune_modules::test::module(true)?)?;
//! # Ok(())
//! # }
//! ```

use rune::macros::{quote, MacroContext, TokenStream};
use rune::ast;
use rune::macros;
use rune::T;
use rune::parse::Parser;

/// Construct the `std::test` module.
pub fn module(_stdio: bool) -> Result<rune::Module, rune::ContextError> {
    let mut module = rune::Module::with_crate_item("std", &["test"]);
    module.macro_(&["assert"], assert_macro)?;
    module.macro_(&["assert_eq"], assert_eq_macro)?;
    Ok(module)
}

/// Implementation for the `assert!` macro.
pub(crate) fn assert_macro(ctx: &mut MacroContext<'_>, stream: &TokenStream) -> rune::Result<TokenStream> {
    let mut p = Parser::from_token_stream(stream, ctx.stream_span());
    let expr = p.parse::<ast::Expr>()?;

    let message = if p.parse::<Option<T![,]>>()?.is_some() {
        p.parse_all::<Option<macros::FormatArgs>>()?
    } else {
        None
    };

    let output = if let Some(message) = &message {
        let expanded = message.expand(ctx)?;

        quote!(if !(#expr) {
            panic("assertion failed: " + (#expanded));
        })
    } else {
        let message = format!("assertion failed: {}", ctx.stringify(&expr));
        let message = ctx.lit(&message);

        quote!(if !(#expr) {
            panic(#message);
        })
    };

    Ok(output.into_token_stream(ctx))
}

/// Implementation for the `assert!` macro.
pub(crate) fn assert_eq_macro(ctx: &mut MacroContext<'_>, stream: &TokenStream) -> rune::Result<TokenStream> {
    let mut p = Parser::from_token_stream(stream, ctx.stream_span());
    let left = p.parse::<ast::Expr>()?;
    p.parse::<T![,]>()?;
    let right = p.parse::<ast::Expr>()?;

    let message = if p.parse::<Option<T![,]>>()?.is_some() {
        p.parse_all::<Option<macros::FormatArgs>>()?
    } else {
        None
    };

    let output = if let Some(message) = &message {
        let message = message.expand(ctx)?;

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
        let message = ctx.lit("assertion failed (left == right):");

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

    Ok(output.into_token_stream(ctx))
}
