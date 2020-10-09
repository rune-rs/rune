use crate::ast::{Kind, Token};
use crate::macros::{TokenStream, TokenStreamIter};
use crate::parsing::{Lexer, Parse, ParseError, ParseErrorKind, Peek};
use crate::OptionSpanned as _;
use runestick::Span;
use std::collections::VecDeque;
use std::fmt;
use std::ops;

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
    /// The default span to use in case no better one is available.
    span: Option<Span>,
}

impl<'a> Parser<'a> {
    /// Construct a new parser around the given source.
    pub fn new(source: &'a str) -> Self {
        Self::with_source(Source {
            inner: SourceInner::Lexer(Lexer::new(source)),
        })
    }

    /// Try to consume a single thing matching `T`, returns `true` if any tokens
    /// were consumed.
    pub fn try_consume<T>(&mut self) -> Result<bool, ParseError>
    where
        T: Parse + Peek,
    {
        Ok(if self.peek::<T>()? {
            self.parse::<T>()?;
            true
        } else {
            false
        })
    }

    /// Try to consume all things matching `T`, returns `true` if any tokens
    /// were consumed.
    pub fn try_consume_all<T>(&mut self) -> Result<bool, ParseError>
    where
        T: Parse + Peek,
    {
        let mut consumed = false;

        while self.peek::<T>()? {
            self.parse::<T>()?;
            consumed = true;
        }

        Ok(consumed)
    }

    /// Construct a parser from a token stream.
    pub fn from_token_stream(token_stream: &'a TokenStream) -> Self {
        Self::with_source(Source {
            inner: SourceInner::TokenStream(token_stream.iter()),
        })
    }

    /// Construct a new parser with a source.
    fn with_source(source: Source<'a>) -> Self {
        let span = source.span().or_else(crate::macros::current_stream_span);

        Self {
            peeker: Peeker {
                source,
                buf: VecDeque::new(),
                error: None,
                last: None,
            },
            span,
        }
    }

    /// Get the span for the given range offset of tokens.
    pub fn span(&mut self, range: ops::Range<usize>) -> Span {
        self.span_at(range.start).join(self.span_at(range.end))
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
    #[allow(clippy::should_implement_trait)]
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
                self.last_span().end(),
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

    /// Get the span for the given offset.
    pub fn span_at(&mut self, n: usize) -> Span {
        if let Ok(Some(t)) = self.peeker.at(n) {
            t.span
        } else {
            self.last_span().end()
        }
    }

    /// Get the span at the given position.
    pub fn tok_at(&mut self, n: usize) -> Result<Token, ParseError> {
        Ok(if let Some(t) = self.peeker.at(n)? {
            t
        } else {
            Token {
                kind: Kind::Eof,
                span: self.last_span().end(),
            }
        })
    }

    /// The last known span in this parser.
    pub fn last_span(&self) -> Span {
        self.peeker.last.or(self.span).unwrap_or_default()
    }
}

/// Construct used to peek a parser.
#[derive(Debug)]
pub struct Peeker<'a> {
    pub(crate) source: Source<'a>,
    buf: VecDeque<Token>,
    // NB: parse errors encountered during peeking.
    error: Option<ParseError>,
    /// The last span we encountered. Used to provide better EOF diagnostics.
    last: Option<Span>,
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

            self.last = Some(token.span);
            self.buf.push_back(token);
        }

        Ok(self.buf.get(n).copied())
    }

    /// Test if we are at end of file.
    pub fn is_eof(&mut self) -> bool {
        match self.at(0) {
            Ok(t) => t.is_none(),
            Err(error) => {
                self.error = Some(error);
                false
            }
        }
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
