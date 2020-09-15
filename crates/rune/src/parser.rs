use crate::ast::Token;
use crate::lexer::Lexer;
use crate::token_stream::{TokenStream, TokenStreamIter};
use crate::traits::{Parse, Peek};
use crate::{ParseError, ParseErrorKind};
use runestick::Span;
use std::fmt;

/// Parser for the rune language.
///
/// # Examples
///
/// ```rust
/// use rune::{ast, Parser};
///
/// let mut parser = Parser::new("fn foo() {}");
/// parser.parse::<ast::ItemFn>().unwrap();
/// ```
#[derive(Debug)]
pub struct Parser<'a> {
    pub(crate) source: Source<'a>,
    p1: Result<Option<Token>, ParseError>,
    p2: Result<Option<Token>, ParseError>,
    p3: Result<Option<Token>, ParseError>,
}

impl<'a> Parser<'a> {
    /// Construct a new parser around the given source.
    pub fn new(source: &'a str) -> Self {
        Self::new_with_start(source, 0)
    }

    /// Construct a parser from a token stream.
    pub fn from_token_stream(token_stream: &'a TokenStream) -> Self {
        Self::with_source(Source {
            inner: SourceInner::TokenStream(token_stream.iter()),
        })
    }

    /// Construct a new parser around the given source.
    pub(crate) fn new_with_start(source: &'a str, start: usize) -> Self {
        Self::with_source(Source {
            inner: SourceInner::Lexer(Lexer::new_with_start(source, start)),
        })
    }

    /// Construct a new parser with a source.
    fn with_source(mut source: Source<'a>) -> Self {
        let p1 = source.next();
        let p2 = source.next();
        let p3 = source.next();

        Self { source, p1, p2, p3 }
    }

    /// Parse a specific item from the parser.
    pub fn parse<T>(&mut self) -> Result<T, ParseError>
    where
        T: Parse,
    {
        T::parse(self)
    }

    /// Peek for the given token.
    pub fn peek<T>(&self) -> Result<bool, ParseError>
    where
        T: Peek,
    {
        Ok(T::peek(self.p1?, self.p2?))
    }

    /// Peek for the given token.
    pub fn peek2<T>(&self) -> Result<bool, ParseError>
    where
        T: Peek,
    {
        Ok(T::peek(self.p2?, self.p3?))
    }

    /// Peek the current token.
    pub fn token_peek(&mut self) -> Result<Option<Token>, ParseError> {
        self.p1
    }

    /// Peek the next two tokens.
    pub fn token_peek_pair(&mut self) -> Result<Option<(Token, Option<Token>)>, ParseError> {
        Ok(match self.p1? {
            Some(p1) => Some((p1, self.p2?)),
            None => None,
        })
    }

    /// Consume the next token from the lexer.
    pub fn token_next(&mut self) -> Result<Token, ParseError> {
        let token = std::mem::replace(&mut self.p3, self.source.next());
        let token = std::mem::replace(&mut self.p2, token);
        let token = std::mem::replace(&mut self.p1, token);

        match token? {
            Some(token) => Ok(token),
            None => Err(ParseError::new(
                self.source.end(),
                ParseErrorKind::UnexpectedEof,
            )),
        }
    }

    /// Peek the current token from the lexer but treat a missing token as an
    /// unexpected end-of-file.
    pub fn token_peek_eof(&mut self) -> Result<Token, ParseError> {
        match self.p1? {
            Some(token) => Ok(token),
            None => Err(ParseError::new(
                self.source.end(),
                ParseErrorKind::UnexpectedEof,
            )),
        }
    }

    /// Test if the parser is at end-of-file, after which there is no more input
    /// to parse.
    pub fn is_eof(&self) -> Result<bool, ParseError> {
        Ok(self.p1?.is_none())
    }

    /// Assert that the parser has reached its end-of-file.
    pub fn parse_eof(&mut self) -> Result<(), ParseError> {
        if let Some(token) = self.source.next()? {
            return Err(ParseError::new(
                token,
                ParseErrorKind::ExpectedEof { actual: token.kind },
            ));
        }

        Ok(())
    }
}

/// A source adapter.
pub(crate) struct Source<'a> {
    inner: SourceInner<'a>,
}

impl Source<'_> {
    /// Get the end span of the source.
    pub(crate) fn end(&self) -> Span {
        match &self.inner {
            SourceInner::Lexer(lexer) => lexer.end(),
            SourceInner::TokenStream(token_stream) => token_stream.end(),
        }
    }

    /// Get the next token in the stream.
    pub(crate) fn next(&mut self) -> Result<Option<Token>, ParseError> {
        match &mut self.inner {
            SourceInner::Lexer(lexer) => lexer.next(),
            SourceInner::TokenStream(token_stream) => Ok(token_stream.next()),
        }
    }
}

impl fmt::Debug for Source<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

#[derive(Debug)]
enum SourceInner<'a> {
    Lexer(Lexer<'a>),
    TokenStream(TokenStreamIter<'a>),
}
