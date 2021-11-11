use crate::ir::{IrCompile, IrError, IrEval};
use crate::macros::{current_context, current_context_mut, ToTokens, TokenStream};
use crate::parsing::{Parse, ParseError, ResolveError, ResolveOwned};
use crate::Spanned;
use runestick::Source;

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
pub fn eval<T>(target: &T) -> Result<<T::Output as IrEval>::Output, IrError>
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
pub fn resolve<T>(item: T) -> Result<T::Owned, ResolveError>
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

/// Parse the given input as the given type that implements
/// [Parse][crate::parsing::Parse].
///
/// # Panics
///
/// This will panic if it's called outside of a macro context.
pub fn parse_all<T>(source: &str) -> Result<T, ParseError>
where
    T: Parse,
{
    current_context_mut(|ctx| {
        let sources = ctx.sources_mut();
        let source_id = sources.insert(Source::new("macro", source));
        crate::parse_all(source, source_id)
    })
}
