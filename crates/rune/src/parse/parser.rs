use core::fmt;
use core::ops;

use crate::alloc::VecDeque;
use crate::ast::{Kind, OptionSpanned, Span, Token};
use crate::compile::{self, ErrorKind};
use crate::macros::{TokenStream, TokenStreamIter};
use crate::parse::{Lexer, Parse, Peek};
use crate::SourceId;

/// Parser for the rune language.
///
/// # Examples
///
/// ```
/// use rune::ast;
/// use rune::SourceId;
/// use rune::parse::Parser;
///
/// let mut parser = Parser::new("fn foo() {}", SourceId::empty(), false);
/// let ast = parser.parse::<ast::ItemFn>()?;
/// # Ok::<_, rune::support::Error>(())
/// ```
#[derive(Debug)]
pub struct Parser<'a> {
    peeker: Peeker<'a>,
}

impl<'a> Parser<'a> {
    /// Construct a new parser around the given source.
    ///
    /// `shebang` indicates if the parser should try and parse a shebang or not.
    pub fn new(source: &'a str, source_id: SourceId, shebang: bool) -> Self {
        Self::with_source(
            Source {
                inner: SourceInner::Lexer(Lexer::new(source, source_id, shebang)),
            },
            Span::new(0u32, source.len()),
        )
    }

    /// Construct a parser from a token stream. The second argument `span` is
    /// the span to use if the stream is empty.
    pub fn from_token_stream(token_stream: &'a TokenStream, span: Span) -> Self {
        Self::with_source(
            Source {
                inner: SourceInner::TokenStream(token_stream.iter()),
            },
            span,
        )
    }

    /// Parse a specific item from the parser.
    pub fn parse<T>(&mut self) -> compile::Result<T>
    where
        T: Parse,
    {
        T::parse(self)
    }

    /// Parse a specific item from the parser and then expect end of input.
    pub fn parse_all<T>(&mut self) -> compile::Result<T>
    where
        T: Parse,
    {
        let item = self.parse::<T>()?;
        self.eof()?;
        Ok(item)
    }

    /// Peek for the given token.
    pub fn peek<T>(&mut self) -> compile::Result<bool>
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

    /// Assert that the parser has reached its end-of-file.
    pub fn eof(&mut self) -> compile::Result<()> {
        if let Some(token) = self.peeker.at(0)? {
            return Err(compile::Error::new(
                token,
                ErrorKind::ExpectedEof { actual: token.kind },
            ));
        }

        Ok(())
    }

    /// Test if the parser is at end-of-file, after which there is no more input
    /// to parse.
    pub fn is_eof(&mut self) -> compile::Result<bool> {
        Ok(self.peeker.at(0)?.is_none())
    }

    /// Construct a new parser with a source.
    fn with_source(source: Source<'a>, span: Span) -> Self {
        let default_span = source.span().unwrap_or(span);

        Self {
            peeker: Peeker {
                source,
                buf: VecDeque::new(),
                error: None,
                last: None,
                default_span,
            },
        }
    }

    /// Try to consume a single thing matching `T`, returns `true` if any tokens
    /// were consumed.
    pub fn try_consume<T>(&mut self) -> compile::Result<bool>
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
    pub fn try_consume_all<T>(&mut self) -> compile::Result<bool>
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

    /// Get the span for the given range offset of tokens.
    pub(crate) fn span(&mut self, range: ops::Range<usize>) -> Span {
        self.span_at(range.start).join(self.span_at(range.end))
    }

    /// Access the interior peeker of the parser.
    pub(crate) fn peeker(&mut self) -> &mut Peeker<'a> {
        &mut self.peeker
    }

    /// Consume the next token from the parser.
    #[allow(clippy::should_implement_trait)]
    pub(crate) fn next(&mut self) -> compile::Result<Token> {
        if let Some(error) = self.peeker.error.take() {
            return Err(error);
        }

        if let Some(t) = self.peeker.buf.pop_front() {
            return Ok(t);
        }

        match self.peeker.next()? {
            Some(t) => Ok(t),
            None => Err(compile::Error::new(
                self.last_span().tail(),
                ErrorKind::UnexpectedEof,
            )),
        }
    }

    /// Peek the token kind at the given position.
    pub(crate) fn nth(&mut self, n: usize) -> compile::Result<Kind> {
        if let Some(t) = self.peeker.at(n)? {
            Ok(t.kind)
        } else {
            Ok(Kind::Eof)
        }
    }

    /// Get the span for the given offset.
    pub(crate) fn span_at(&mut self, n: usize) -> Span {
        if let Ok(Some(t)) = self.peeker.at(n) {
            t.span
        } else {
            self.last_span().tail()
        }
    }

    /// Get the token at the given offset.
    pub(crate) fn tok_at(&mut self, n: usize) -> compile::Result<Token> {
        Ok(if let Some(t) = self.peeker.at(n)? {
            t
        } else {
            Token {
                kind: Kind::Eof,
                span: self.last_span().tail(),
            }
        })
    }

    /// The last known span in this parser.
    pub(crate) fn last_span(&self) -> Span {
        self.peeker.last_span()
    }
}

/// Construct used to peek a parser.
#[derive(Debug)]
pub struct Peeker<'a> {
    /// The source being processed.
    pub(crate) source: Source<'a>,
    /// The buffer of tokens seen.
    buf: VecDeque<Token>,
    // NB: parse errors encountered during peeking.
    error: Option<compile::Error>,
    /// The last span we encountered. Used to provide better EOF diagnostics.
    last: Option<Span>,
    /// The default span to use in case no better one is available.
    default_span: Span,
}

impl<'a> Peeker<'a> {
    /// Peek the token kind at the given position.
    pub(crate) fn nth(&mut self, n: usize) -> Kind {
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

    /// Get the span at the given position.
    pub(crate) fn tok_at(&mut self, n: usize) -> Token {
        let kind = match self.at(n) {
            Ok(t) => {
                if let Some(t) = t {
                    return t;
                } else {
                    Kind::Eof
                }
            }
            Err(e) => {
                self.error = Some(e);
                Kind::Error
            }
        };

        Token {
            kind,
            span: self.last_span().tail(),
        }
    }

    /// Test if we are at end of file.
    pub(crate) fn is_eof(&mut self) -> bool {
        match self.at(0) {
            Ok(t) => t.is_none(),
            Err(error) => {
                self.error = Some(error);
                false
            }
        }
    }

    /// Advance the internals of the peeker and return the next token (without
    /// buffering).
    fn next(&mut self) -> compile::Result<Option<Token>> {
        loop {
            let token = match self.source.next()? {
                Some(token) => token,
                None => return Ok(None),
            };

            match token.kind {
                Kind::Comment | Kind::Whitespace => {
                    continue;
                }
                Kind::MultilineComment(term) => {
                    if !term {
                        return Err(compile::Error::new(
                            token.span,
                            ErrorKind::ExpectedMultilineCommentTerm,
                        ));
                    }

                    continue;
                }
                _ => (),
            }

            return Ok(Some(token));
        }
    }

    /// Make sure there are at least `n` items in the buffer, and return the
    /// item at that point.
    fn at(&mut self, n: usize) -> compile::Result<Option<Token>> {
        if let Some(error) = self.error.take() {
            return Err(error);
        }

        while self.buf.len() <= n {
            let token = match self.next()? {
                Some(token) => token,
                None => break,
            };

            self.last = Some(token.span);
            self.buf.try_push_back(token)?;
        }

        Ok(self.buf.get(n).copied())
    }

    /// The last known span in this parser.
    fn last_span(&self) -> Span {
        self.last.unwrap_or(self.default_span)
    }
}

/// A source adapter.
pub(crate) struct Source<'a> {
    inner: SourceInner<'a>,
}

impl Source<'_> {
    /// Get the span of the source.
    fn span(&self) -> Option<Span> {
        match &self.inner {
            SourceInner::Lexer(lexer) => Some(lexer.span()),
            SourceInner::TokenStream(token_stream) => token_stream.option_span(),
        }
    }

    /// Get the next token in the stream.
    fn next(&mut self) -> compile::Result<Option<Token>> {
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
