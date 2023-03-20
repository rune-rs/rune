use crate::macros::{MacroContext, ToTokens, TokenStream};
use std::fmt;

type EncodeFn<'a> = dyn Fn(&mut MacroContext<'_>, &mut TokenStream) + Send + Sync + 'a;

/// Construct a token stream from a function.
pub fn quote_fn<'a, T>(f: T) -> Quote<'a>
where
    T: 'a + Fn(&mut MacroContext<'_>, &mut TokenStream) + Send + Sync,
{
    Quote(Box::new(f))
}

/// [ToTokens] implementation generated by [quote_fn].
pub struct Quote<'a>(Box<EncodeFn<'a>>);

impl<'a> Quote<'a> {
    /// Convert into token stream.
    ///
    /// # Panics
    ///
    /// This panics if called outside of a macro context.
    pub fn into_token_stream(self, ctx: &mut MacroContext<'_>) -> TokenStream {
        let mut stream = TokenStream::new();
        self.to_tokens(ctx, &mut stream);
        stream
    }
}

impl<'a> ToTokens for Quote<'a> {
    fn to_tokens(&self, context: &mut MacroContext<'_>, stream: &mut TokenStream) {
        (self.0)(context, stream)
    }
}

impl<'a> fmt::Debug for Quote<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Quote").finish()
    }
}