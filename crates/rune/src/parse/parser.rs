use core::fmt;
use core::ops;

use crate::alloc::VecDeque;
use crate::ast::Spanned;
use crate::ast::{Kind, OptionSpanned, Span, Token};
use crate::compile::WithSpan;
use crate::compile::{self, ErrorKind, Options};
use crate::macros::{TokenStream, TokenStreamIter};
use crate::parse::{Advance, Lexer, Parse, Peek};
use crate::shared::FixedVec;
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
    /// How deep the tree built so far is along the path being parsed.
    nesting: usize,
    /// How deep a tree this parser is allowed to produce.
    max_depth: usize,
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

    /// Configure how deep a tree this parser is allowed to produce.
    ///
    /// Defaults to the `max-ast-depth` option's default. A macro which is
    /// handed the compiler's options through its [`MacroContext`] should
    /// configure the parser it builds from them, which
    /// [`MacroContext::parser`] does.
    ///
    /// [`MacroContext`]: crate::macros::MacroContext
    /// [`MacroContext::parser`]: crate::macros::MacroContext::parser
    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Parse one level deeper, ensuring that the tree being built does not get
    /// deeper than the parser allows.
    ///
    /// Every recursive path through the syntax tree descends through here, so
    /// that input which is too deep is reported as a diagnostic rather than
    /// overflowing the stack, either while it is being parsed or later while it
    /// is being walked.
    ///
    /// The level is *restored* rather than decremented on the way out, so that
    /// the links [`Parser::link`] accounts for are released along with the
    /// expression they belong to.
    pub(crate) fn nested<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> compile::Result<T>,
    ) -> compile::Result<T> {
        let nesting = self.nesting;
        self.deepen()?;
        let result = parse(self);
        self.nesting = nesting;
        result
    }

    /// Account for one more link of a chain.
    ///
    /// A chain is parsed over a loop rather than by recursing, so it costs no
    /// parser frames, but each link is another level of the tree which is left
    /// behind. The level is released by the enclosing [`Parser::nested`], which
    /// is the expression the chain belongs to.
    pub(crate) fn link(&mut self) -> compile::Result<()> {
        self.deepen()
    }

    fn deepen(&mut self) -> compile::Result<()> {
        if self.nesting >= self.max_depth {
            return Err(compile::Error::new(
                self.span_at(0),
                ErrorKind::MaxAstDepth {
                    max: self.max_depth,
                },
            ));
        }

        self.nesting += 1;
        Ok(())
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
            nesting: 0,
            max_depth: Options::DEFAULT.max_ast_depth,
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
    source: Source<'a>,
    /// The buffer of tokens seen.
    buf: VecDeque<Token>,
    // NB: parse errors encountered during peeking.
    error: Option<compile::Error>,
    /// The last span we encountered. Used to provide better EOF diagnostics.
    last: Option<Span>,
    /// The default span to use in case no better one is available.
    default_span: Span,
}

impl Peeker<'_> {
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

    /// Peek an array.
    pub(crate) fn array<const N: usize>(&mut self) -> FixedVec<Token, N> {
        let mut vec = FixedVec::new();

        if N == 0 {
            return vec;
        }

        if let Err(error) = self.fill(N) {
            self.error = Some(error);
        }

        let mut it = 0..N;

        for (&tok, _) in self.buf.iter().zip(it.by_ref()) {
            _ = vec.try_push(tok);
        }

        if let Some(error) = &self.error {
            for _ in it {
                _ = vec.try_push(Token {
                    kind: Kind::Error,
                    span: error.span(),
                });
            }
        } else {
            for _ in it {
                _ = vec.try_push(Token {
                    kind: Kind::Eof,
                    span: self.last_span(),
                });
            }
        }

        vec
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
            let Some(token) = self.source.next()? else {
                return Ok(None);
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
        self.fill(n)?;
        Ok(self.buf.get(n).copied())
    }

    fn fill(&mut self, n: usize) -> compile::Result<()> {
        if let Some(error) = self.error.take() {
            return Err(error);
        }

        while self.buf.len() <= n {
            let Some(tok) = self.next()? else {
                break;
            };

            self.last = Some(tok.span);
            self.buf.try_push_back(tok).with_span(tok.span)?;
        }

        Ok(())
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

impl Advance for Parser<'_> {
    type Error = compile::Error;

    #[inline]
    fn advance(&mut self, n: usize) -> Result<(), Self::Error> {
        for _ in 0..n {
            self.next()?;
        }

        Ok(())
    }
}
