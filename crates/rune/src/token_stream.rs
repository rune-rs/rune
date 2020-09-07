use crate::ast::Token;
use runestick::Span;
use std::slice;

/// A token stream.
#[derive(Default, Debug, Clone)]
pub struct TokenStream {
    stream: Vec<Token>,
    default_span: Span,
    end: Span,
}

impl TokenStream {
    /// Construct a new token stream with the specified end span.
    pub fn new(stream: Vec<Token>, default_span: Span, end: Span) -> Self {
        Self {
            stream,
            default_span,
            end,
        }
    }

    /// Get the default span for the stream.
    pub fn default_span(&self) -> Span {
        self.default_span
    }

    /// Push the current token to the stream.
    pub fn push(&mut self, token: Token) {
        self.stream.push(token);
    }

    /// Push something that can be turned into a token stream.
    pub fn extend<T>(&mut self, item: T)
    where
        T: IntoTokens,
    {
        item.into_tokens(self);
    }

    /// Get the end span of the token stream.
    pub fn end(&self) -> Span {
        self.end
    }

    /// Create an iterator over the token stream.
    pub(crate) fn iter(&self) -> TokenStreamIter<'_> {
        TokenStreamIter {
            iter: self.stream.iter(),
            end: self.end,
        }
    }
}

/// A token stream iterator.
#[derive(Debug)]
pub struct TokenStreamIter<'a> {
    iter: slice::Iter<'a, Token>,
    end: Span,
}

impl TokenStreamIter<'_> {
    /// Get the end point of the token stream iterator.
    pub(crate) fn end(&self) -> Span {
        self.end
    }
}

impl Iterator for TokenStreamIter<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().copied()
    }
}

/// Trait for things that can be turned into tokens.
pub trait IntoTokens {
    /// Turn the current item into tokens.
    fn into_tokens(&self, stream: &mut TokenStream);
}
