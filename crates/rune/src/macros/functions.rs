use crate::compiling::CompileError;
use crate::ir::{IrCompile, IrEval};
use crate::macros::{current_context, ToTokens, TokenStream};
use crate::parsing::{ParseError, ResolveOwned};
use crate::Spanned;

/// Evaluate the given target as a constant expression.
///
/// # Panics
///
/// This will panic if it's called outside of a macro context.
///
/// # Examples
///
/// ```rust
/// use rune::{macros, ast, IrValue, MacroContext};
///
/// // Note: should only be used for testing.
/// let ctx = MacroContext::empty();
///
/// macros::with_context(ctx, || {
///     let stream = rune::quote!(1 + 2).into_token_stream();
///
///     let mut p = rune::Parser::from_token_stream(&stream);
///     let expr = p.parse_all::<ast::Expr>().unwrap();
///     let value = macros::eval(&expr).unwrap();
///
///     assert_eq!(3, value.into_integer::<u32>().unwrap());
/// });
/// ```
pub fn eval<T>(target: &T) -> Result<<T::Output as IrEval>::Output, CompileError>
where
    T: Spanned + IrCompile,
    T::Output: IrEval,
{
    current_context(|ctx| ctx.eval(target))
}

/// Resolve the value of a token.
///
/// # Panics
///
/// This will panic if it's called outside of a macro context.
pub fn resolve<T>(item: T) -> Result<T::Owned, ParseError>
where
    T: ResolveOwned,
{
    current_context(|ctx| ctx.resolve_owned(item))
}

/// Convert the given argument into a tokens stream.
///
/// # Panics
///
/// This will panic if it's called outside of a macro context.
pub fn to_tokens<T>(tokens: &T, stream: &mut TokenStream)
where
    T: ToTokens,
{
    current_context(|ctx| tokens.to_tokens(ctx, stream))
}

/// Stringify the token stream.
///
/// # Panics
///
/// This will panic if it's called outside of a macro context.
pub fn stringify<T>(stream: &T) -> String
where
    T: ToTokens,
{
    current_context(|ctx| ctx.stringify(stream).to_string())
}
