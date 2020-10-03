use crate::macros::{current_context, ToTokens, TokenStream};
use crate::parsing::{ParseError, ResolveOwned};

/// Resolve the value of a token.
///
/// # Panics
///
/// This will panic if it's called outside of a macro context.
pub fn resolve<T>(value: T) -> Result<T::Owned, ParseError>
where
    T: ResolveOwned,
{
    current_context(|ctx| {
        let value = value.resolve_owned(ctx.storage(), ctx.source())?;
        Ok(value)
    })
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
