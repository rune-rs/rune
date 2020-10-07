use crate::ast::{Kind, Token};
use crate::macros::{TokenStream, TokenStreamIter};
use crate::parsing::{Lexer, Parse, ParseError, ParseErrorKind, Peek};
use crate::OptionSpanned as _;
use runestick::Span;
use std::collections::VecDeque;
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
    peeker: Peeker<'a>,
    span: Option<Span>,
}

impl<'a> Parser<'a> {
    /// Construct a new parser around the given source.
    pub fn new(source: &'a str) -> Self {
        Self::with_source(Source {
            inner: SourceInner::Lexer(Lexer::new(source)),
        })
    }

    /// Construct a parser from a token stream.
    pub fn from_token_stream(token_stream: &'a TokenStream) -> Self {
        Self::with_source(Source {
            inner: SourceInner::TokenStream(token_stream.iter()),
        })
    }

    /// Construct a new parser with a source.
    fn with_source(source: Source<'a>) -> Self {
        let span = source.span();

        Self {
            peeker: Peeker {
                source,
                buf: VecDeque::new(),
                error: None,
            },
            span,
        }
    }

    /// Parse a specific item from the parser.
    pub fn parse<T>(&mut self) -> Result<T, ParseError>
    where
        T: Parse,
    {
        T::parse(self)
    }

    /// Peek for the given token.
    pub fn peek<T>(&mut self) -> Result<bool, ParseError>
    where
        T: Peek,
    {
        if let Some(error) = self.peeker.error.take() {
            return Err(error);
        }

        let result = T::peek(&mut self.peeker);

        if let Some(error) = self.peeker.error.take() {
            return Err(error);
        }

        Ok(result)
    }

    /// Access the interior peeker of the parser.
    pub fn peeker(&mut self) -> &mut Peeker<'a> {
        &mut self.peeker
    }

    /// Consume the next token from the parser.
    pub fn next(&mut self) -> Result<Token, ParseError> {
        if let Some(error) = self.peeker.error.take() {
            return Err(error);
        }

        if let Some(t) = self.peeker.buf.pop_front() {
            return Ok(t);
        }

        match self.peeker.source.next()? {
            Some(t) => Ok(t),
            None => Err(ParseError::new(
                self.span.unwrap_or_default().end(),
                ParseErrorKind::UnexpectedEof,
            )),
        }
    }

    /// Test if the parser is at end-of-file, after which there is no more input
    /// to parse.
    pub fn is_eof(&mut self) -> Result<bool, ParseError> {
        Ok(self.peeker.at(0)?.is_none())
    }

    /// Assert that the parser has reached its end-of-file.
    pub fn eof(&mut self) -> Result<(), ParseError> {
        if let Some(token) = self.peeker.at(0)? {
            return Err(ParseError::new(
                token,
                ParseErrorKind::ExpectedEof { actual: token.kind },
            ));
        }

        Ok(())
    }

    /// Peek the token kind at the given position.
    pub fn nth(&mut self, n: usize) -> Result<Kind, ParseError> {
        if let Some(t) = self.peeker.at(n)? {
            Ok(t.kind)
        } else {
            Ok(Kind::Eof)
        }
    }

    /// Get the span at the given position.
    pub fn token(&mut self, n: usize) -> Result<Token, ParseError> {
        if let Some(t) = self.peeker.at(n)? {
            Ok(t)
        } else {
            Ok(Token {
                kind: Kind::Eof,
                span: self.span.unwrap_or_default(),
            })
        }
    }
}

/// Construct used to peek a parser.
#[derive(Debug)]
pub struct Peeker<'a> {
    pub(crate) source: Source<'a>,
    buf: VecDeque<Token>,
    // NB: parse errors encountered during peeking.
    error: Option<ParseError>,
}

impl<'a> Peeker<'a> {
    /// Peek the token kind at the given position.
    pub fn nth(&mut self, n: usize) -> Kind {
        // Error tripped already, this peeker returns nothing but errors from
        // here on out.
        if self.error.is_some() {
            return Kind::Error;
        }

        match self.at(n) {
            Ok(t) => match t {
                Some(t) => t.kind,
                None => Kind::Eof,
            },
            Err(error) => {
                self.error = Some(error);
                Kind::Error
            }
        }
    }

    /// Make sure there are at least `n` items in the buffer, and return the
    /// item at that point.
    fn at(&mut self, n: usize) -> Result<Option<Token>, ParseError> {
        if let Some(error) = self.error.take() {
            return Err(error);
        }

        while self.buf.len() <= n {
            let token = match self.source.next()? {
                Some(token) => token,
                None => break,
            };

            self.buf.push_back(token);
        }

        Ok(self.buf.get(n).copied())
    }
}

/// A source adapter.
pub(crate) struct Source<'a> {
    inner: SourceInner<'a>,
}

impl Source<'_> {
    /// Get the span of the source.
    pub(crate) fn span(&self) -> Option<Span> {
        match &self.inner {
            SourceInner::Lexer(lexer) => Some(lexer.span()),
            SourceInner::TokenStream(token_stream) => token_stream.option_span(),
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
