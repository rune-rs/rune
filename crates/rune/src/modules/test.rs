//! `std::test` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io

use crate::ast;
use crate::macros::{quote, FormatArgs, MacroContext, TokenStream};
use crate::parse::Parser;
use crate::{ContextError, Module, T};

/// Construct the `std::test` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["test"]).with_unique("std::test");
    module.macro_(["assert"], assert_macro)?;
    module.macro_(["assert_eq"], assert_eq_macro)?;
    Ok(module)
}

/// Implementation for the `assert!` macro.
pub(crate) fn assert_macro(
    ctx: &mut MacroContext<'_>,
    stream: &TokenStream,
) -> crate::Result<TokenStream> {
    use crate as rune;

    let mut p = Parser::from_token_stream(stream, ctx.stream_span());
    let expr = p.parse::<ast::Expr>()?;

    let message = if p.parse::<Option<T![,]>>()?.is_some() {
        p.parse_all::<Option<FormatArgs>>()?
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
pub(crate) fn assert_eq_macro(
    ctx: &mut MacroContext<'_>,
    stream: &TokenStream,
) -> crate::Result<TokenStream> {
    use crate as rune;

    let mut p = Parser::from_token_stream(stream, ctx.stream_span());
    let left = p.parse::<ast::Expr>()?;
    p.parse::<T![,]>()?;
    let right = p.parse::<ast::Expr>()?;

    let message = if p.parse::<Option<T![,]>>()?.is_some() {
        p.parse_all::<Option<FormatArgs>>()?
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
