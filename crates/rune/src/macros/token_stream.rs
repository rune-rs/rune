use crate::ast::{Kind, Token};
use crate::{MacroContext, OptionSpanned, Span};
use std::fmt;
use std::slice;

/// A token stream.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TokenStream {
    stream: Vec<Token>,
}

impl TokenStream {
    /// Construct an empty token stream for testing.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push the current token to the stream.
    pub fn push(&mut self, token: Token) {
        self.stream.push(token);
    }

    /// Extend the token stream with another iterator.
    pub fn extend<I>(&mut self, tokens: I)
    where
        I: IntoIterator,
        Token: From<I::Item>,
    {
        self.stream.extend(tokens.into_iter().map(Token::from));
    }

    /// Create an iterator over the token stream.
    pub(crate) fn iter(&self) -> TokenStreamIter<'_> {
        TokenStreamIter {
            iter: self.stream.iter(),
        }
    }

    /// Return something that once formatted will produce a stream of kinds.
    pub fn kinds(&self) -> Kinds<'_> {
        Kinds {
            stream: &self.stream,
        }
    }
}

impl From<Vec<Token>> for TokenStream {
    fn from(stream: Vec<Token>) -> Self {
        Self { stream }
    }
}

impl OptionSpanned for TokenStream {
    fn option_span(&self) -> Option<Span> {
        self.stream.option_span()
    }
}

/// A token stream iterator.
#[derive(Debug, Clone)]
pub struct TokenStreamIter<'a> {
    iter: slice::Iter<'a, Token>,
}

impl OptionSpanned for TokenStreamIter<'_> {
    fn option_span(&self) -> Option<Span> {
        self.iter.option_span()
    }
}

impl Iterator for TokenStreamIter<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().copied()
    }
}

impl DoubleEndedIterator for TokenStreamIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().copied()
    }
}

impl<'a> IntoIterator for &'a TokenStream {
    type Item = &'a Token;
    type IntoIter = std::slice::Iter<'a, Token>;

    fn into_iter(self) -> Self::IntoIter {
        self.stream.iter()
    }
}

impl IntoIterator for TokenStream {
    type Item = Token;
    type IntoIter = std::vec::IntoIter<Token>;

    fn into_iter(self) -> Self::IntoIter {
        self.stream.into_iter()
    }
}

/// Trait for things that can be turned into tokens.
pub trait ToTokens: Sized {
    /// Turn the current item into tokens.
    fn to_tokens(&self, context: &mut MacroContext<'_>, stream: &mut TokenStream);
}

impl<T> ToTokens for Box<T>
where
    T: ToTokens,
{
    fn to_tokens(&self, context: &mut MacroContext<'_>, stream: &mut TokenStream) {
        (**self).to_tokens(context, stream);
    }
}

impl<T> ToTokens for &T
where
    T: ToTokens,
{
    fn to_tokens(&self, context: &mut MacroContext<'_>, stream: &mut TokenStream) {
        ToTokens::to_tokens(*self, context, stream)
    }
}

impl<T> ToTokens for Option<T>
where
    T: ToTokens,
{
    fn to_tokens(&self, context: &mut MacroContext<'_>, stream: &mut TokenStream) {
        if let Some(this) = self {
            this.to_tokens(context, stream);
        }
    }
}

impl<T> ToTokens for Vec<T>
where
    T: ToTokens,
{
    fn to_tokens(&self, context: &mut MacroContext<'_>, stream: &mut TokenStream) {
        for item in self {
            item.to_tokens(context, stream);
        }
    }
}

impl<A, B> ToTokens for (A, B)
where
    A: ToTokens,
    B: ToTokens,
{
    fn to_tokens(&self, context: &mut MacroContext<'_>, stream: &mut TokenStream) {
        self.0.to_tokens(context, stream);
        self.1.to_tokens(context, stream);
    }
}

impl<A, B, C> ToTokens for (A, B, C)
where
    A: ToTokens,
    B: ToTokens,
    C: ToTokens,
{
    fn to_tokens(&self, context: &mut MacroContext<'_>, stream: &mut TokenStream) {
        self.0.to_tokens(context, stream);
        self.1.to_tokens(context, stream);
        self.2.to_tokens(context, stream);
    }
}

impl ToTokens for TokenStream {
    fn to_tokens(&self, context: &mut MacroContext<'_>, stream: &mut TokenStream) {
        self.stream.to_tokens(context, stream);
    }
}

impl PartialEq<Vec<Token>> for TokenStream {
    fn eq(&self, other: &Vec<Token>) -> bool {
        self.stream == *other
    }
}

impl PartialEq<TokenStream> for Vec<Token> {
    fn eq(&self, other: &TokenStream) -> bool {
        *self == other.stream
    }
}

pub struct Kinds<'a> {
    stream: &'a [Token],
}

impl Iterator for Kinds<'_> {
    type Item = Kind;

    fn next(&mut self) -> Option<Self::Item> {
        let (first, rest) = self.stream.split_first()?;
        self.stream = rest;
        Some(first.kind)
    }
}

impl fmt::Debug for Kinds<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut it = self.stream.iter();
        let last = it.next_back();

        for t in it {
            write!(f, "{} ", t.kind)?;
        }

        if let Some(t) = last {
            write!(f, "{}", t.kind)?;
        }

        Ok(())
    }
}
