//! `std::test` module for the [Rune Language].
//!
//! [Rune Language]: https://rune-rs.github.io

use crate::no_std::vec::Vec;

use crate as rune;
use crate::ast;
use crate::compile;
use crate::macros::{quote, FormatArgs, MacroContext, TokenStream};
use crate::parse::Parser;
use crate::runtime::Function;
use crate::{Any, ContextError, Module, T};

/// A helper type to capture benchmarks.
#[derive(Default, Any)]
#[rune(module = crate, item = ::std::test)]
pub struct Bencher {
    fns: Vec<Function>,
}

impl Bencher {
    /// Coerce bencher into its underlying functions.
    pub fn into_functions(self) -> Vec<Function> {
        self.fns
    }

    /// Run a benchmark using the given closure.
    #[rune::function]
    fn iter(&mut self, f: Function) {
        self.fns.push(f);
    }
}

/// Construct the `std::test` module.
pub fn module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("std", ["test"]).with_unique("std::test");
    module.macro_meta(assert)?;
    module.macro_meta(assert_eq)?;
    module.ty::<Bencher>()?;
    module.function_meta(Bencher::iter)?;
    Ok(module)
}

/// Assert that the expression provided as an argument is true, or cause a vm
/// panic.
///
/// The second argument can optionally be used to format a panic message.
///
/// This is useful when writing test cases.
///
/// # Examples
///
/// ```rune
/// let value = 42;
///
/// assert!(value == 42, "Value was not what was expected, instead it was {}", value);
/// ```
#[rune::macro_]
pub(crate) fn assert(
    ctx: &mut MacroContext<'_, '_>,
    stream: &TokenStream,
) -> compile::Result<TokenStream> {
    use crate as rune;

    let mut p = Parser::from_token_stream(stream, ctx.input_span());
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

/// Assert that the two arguments provided are equal, or cause a vm panic.
///
/// The third argument can optionally be used to format a panic message.
///
/// # Examples
///
/// ```rune
/// let value = 42;
///
/// assert_eq!(value, 42, "Value was not 42, instead it was {}", value);
/// ```
#[rune::macro_]
pub(crate) fn assert_eq(
    ctx: &mut MacroContext<'_, '_>,
    stream: &TokenStream,
) -> compile::Result<TokenStream> {
    use crate as rune;

    let mut p = Parser::from_token_stream(stream, ctx.input_span());
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
