use crate::quote::inner::ToTokens;
use proc_macro2::{Span, TokenStream};

#[derive(Debug, Clone)]
pub(crate) struct Builder {
    stream: TokenStream,
    span: Option<Span>,
}

impl Builder {
    pub(crate) fn new() -> Self {
        Self {
            stream: TokenStream::new(),
            span: None,
        }
    }

    pub(crate) fn into_stream(self) -> TokenStream {
        self.stream
    }

    pub(crate) fn push<T>(&mut self, tokens: T)
    where
        T: ToTokens,
    {
        let span = self.span.unwrap_or_else(Span::call_site);
        tokens.to_tokens(&mut self.stream, span);
    }

    pub(crate) fn push_spanned<T>(&mut self, span: Span, tokens: T)
    where
        T: ToTokens,
    {
        tokens.to_tokens(&mut self.stream, span);
    }
}

impl ToTokens for Builder {
    fn to_tokens(self, stream: &mut TokenStream, _: Span) {
        stream.extend(self.stream);
    }
}

impl From<TokenStream> for Builder {
    fn from(stream: TokenStream) -> Self {
        Self { stream, span: None }
    }
}
