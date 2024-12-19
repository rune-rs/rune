use core::fmt;
use core::mem::take;

use crate::alloc::VecDeque;

use crate::ast::{Kind, OptionSpanned, Span, Token};
use crate::compile::{Error, ErrorKind, Result, WithSpan};
use crate::grammar::ws;
use crate::macros::TokenStreamIter;
use crate::parse::{Advance, IntoExpectation, Lexer};
use crate::shared::{rune_trace, FixedVec};

use super::{inner_token, Flavor, InternalChildren, Node, Tree};

/// A checkpoint during tree construction.
#[derive(Clone)]
pub(super) struct Checkpoint {
    span: Span,
    inner: syntree::Checkpoint<syntree::pointer::PointerUsize>,
}

impl Checkpoint {
    /// Get the span of the checkpoint.
    pub(super) fn span(&self) -> Span {
        self.span
    }
}

pub(super) struct Parser<'a> {
    lexer: Source<'a>,
    ws: VecDeque<Token>,
    buf: VecDeque<(Token, usize)>,
    tree: syntree::Builder<Kind, Flavor>,
    eof: Token,
    include_whitespace: bool,
}

impl<'a> Parser<'a> {
    pub(super) fn new(source: Source<'a>) -> Self {
        let span = source.span().unwrap_or_else(Span::empty).tail();

        let eof = Token {
            span,
            kind: Kind::Eof,
        };

        Self {
            lexer: source,
            ws: VecDeque::new(),
            buf: VecDeque::new(),
            tree: syntree::Builder::new_with(),
            eof,
            include_whitespace: false,
        }
    }

    /// Configure whether whitespace should be ignored.
    pub(super) fn include_whitespace(&mut self, include_whitespace: bool) {
        self.include_whitespace = include_whitespace;
    }

    /// Generate an error encompassing the current token.
    pub(super) fn expected_at(
        &mut self,
        at: usize,
        expected: impl IntoExpectation,
    ) -> Result<Error> {
        let tok = self.nth_token(at)?;

        Ok(Error::new(
            tok.span,
            ErrorKind::ExpectedSyntax {
                expected: expected.into_expectation(),
                actual: tok.kind.into_expectation(),
            },
        ))
    }

    /// Generate an error encompassing the from span.
    #[tracing::instrument(skip_all)]
    pub(super) fn error(&mut self, from: Span, kind: ErrorKind) -> Result<Error> {
        let to = self.nth_token(0)?;
        let span = from.join(to.span);
        Ok(Error::new(span, kind))
    }

    /// Test if we are at EOF.
    #[tracing::instrument(skip_all)]
    pub(super) fn is_eof(&mut self) -> Result<bool> {
        Ok(self.peek()? == Kind::Eof)
    }

    /// Construct the syntax tree.
    #[tracing::instrument(skip_all)]
    pub(crate) fn build(self) -> Result<Tree> {
        let tree = self.tree.build().with_span(self.eof.span)?;
        Ok(Tree::new(tree))
    }

    #[tracing::instrument(skip_all)]
    pub(super) fn checkpoint(&mut self) -> Result<Checkpoint> {
        let span = self.flush_ws()?;

        Ok(Checkpoint {
            span,
            inner: self.tree.checkpoint().with_span(span)?,
        })
    }

    #[tracing::instrument(skip_all)]
    pub(super) fn bump(&mut self) -> Result<Token> {
        let tok = self.next()?;
        emit(&mut self.tree, tok)?;
        Ok(tok)
    }

    /// Bump while the given token matches.
    #[tracing::instrument(skip_all)]
    pub(super) fn bump_while(&mut self, kind: Kind) -> Result<bool> {
        let mut any = false;

        while self.peek()? == kind {
            self.bump()?;
            any = true;
        }

        Ok(any)
    }

    /// Bump if the given token matches.
    #[tracing::instrument(skip_all)]
    pub(super) fn bump_if_matches(&mut self, m: fn(Kind) -> bool) -> Result<bool> {
        self._bump_if_matches(m)
    }

    #[tracing::instrument(skip_all)]
    pub(super) fn bump_if(&mut self, kind: Kind) -> Result<bool> {
        self._bump_if_matches(|k| k == kind)
    }

    #[inline]
    fn _bump_if_matches(&mut self, m: impl FnOnce(Kind) -> bool) -> Result<bool> {
        if m(self.peek()?) {
            self.bump()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Open a new node.
    pub(super) fn open(&mut self, kind: Kind) -> Result<()> {
        self.tree.open(kind).with_span(Span::point(0))?;
        Ok(())
    }

    /// Close the last opened node.
    pub(super) fn close(&mut self) -> Result<()> {
        self.tree.close().with_span(Span::point(0))?;
        Ok(())
    }

    /// Bump and immediately close a token with the specified kind.
    #[tracing::instrument(skip_all)]
    pub(super) fn push(&mut self, kind: Kind) -> Result<()> {
        let tok = self.next()?;
        self.tree.open(kind).with_span(tok.span)?;
        let span = syntree::Span::new(tok.span.start.0, tok.span.end.0);
        self.tree.token_with(tok.kind, span).with_span(tok.span)?;
        self.tree.close().with_span(tok.span)?;
        Ok(())
    }

    /// Bump an empty node.
    #[tracing::instrument(skip_all)]
    pub(super) fn empty(&mut self, kind: Kind) -> Result<()> {
        self.flush_ws()?;
        let span = self.nth_token(0)?.span;
        let s = span.head();
        let s = syntree::Span::new(s.start.0, s.end.0);
        self.tree.token_with(kind, s).with_span(span)?;
        Ok(())
    }

    /// Close a node at the given checkpoint.
    #[tracing::instrument(skip_all)]
    pub(super) fn close_at(&mut self, c: &Checkpoint, kind: Kind) -> Result<()> {
        self.tree.close_at(&c.inner, kind).with_span(c.span)?;
        Ok(())
    }

    /// Peek the next token skipping over whitespace.
    #[tracing::instrument(skip_all)]
    pub(super) fn peek(&mut self) -> Result<Kind> {
        self.nth(0)
    }

    #[tracing::instrument(skip(self))]
    pub(super) fn glued(&mut self, n: usize) -> Result<Kind> {
        self.fill(n)?;

        let Some((tok, 0)) = self.buf.get(n) else {
            return Ok(Kind::Eof);
        };

        Ok(tok.kind)
    }

    /// Flush whitespace.
    pub(super) fn flush_ws(&mut self) -> Result<Span> {
        self.fill(0)?;

        let Some((tok, ws)) = self.buf.front_mut() else {
            for tok in self.ws.drain(..) {
                emit(&mut self.tree, tok)?;
            }

            return Ok(self.eof.span);
        };

        let span = tok.span.head();

        for tok in self.ws.drain(..take(ws)) {
            emit(&mut self.tree, tok)?;
        }

        Ok(span)
    }

    #[inline]
    pub(super) fn nth(&mut self, n: usize) -> Result<Kind> {
        self.fill(n)?;

        let Some((tok, _)) = self.buf.get(n) else {
            return Ok(Kind::Eof);
        };

        Ok(tok.kind)
    }

    #[inline]
    pub(super) fn nth_token(&mut self, n: usize) -> Result<Token> {
        self.fill(n)?;

        let Some((tok, _)) = self.buf.get(n) else {
            return Ok(self.eof);
        };

        Ok(*tok)
    }

    /// Access an array.
    pub(super) fn array<const N: usize>(&mut self) -> Result<FixedVec<Token, N>> {
        if N == 0 {
            return Ok(FixedVec::new());
        }

        self.fill(N - 1)?;

        let mut vec = FixedVec::new();

        for index in 0.. {
            if vec.len() == N {
                break;
            }

            if let Some((tok, _)) = self.buf.get(index) {
                vec.try_push(*tok).with_span(tok.span)?;
            } else {
                vec.try_push(self.eof).with_span(self.eof.span)?;
            }
        }

        Ok(vec)
    }

    #[tracing::instrument(skip_all)]
    fn next(&mut self) -> Result<Token> {
        self.flush_ws()?;

        let Some((tok, _)) = self.buf.pop_front() else {
            return Ok(self.eof);
        };

        rune_trace!("grammar.rs", tok);
        Ok(tok)
    }

    fn advance(&mut self, n: usize) -> Result<()> {
        for (tok, ws) in self.buf.drain(..n) {
            for tok in self.ws.drain(..ws) {
                emit(&mut self.tree, tok)?;
            }

            emit(&mut self.tree, tok)?;
        }

        Ok(())
    }

    fn fill(&mut self, n: usize) -> Result<()> {
        let mut ws = 0;

        while self.buf.len() <= n {
            let Some(tok) = self.lexer.next()? else {
                break;
            };

            if !matches!(tok.kind, ws!()) {
                self.buf
                    .try_push_back((tok, take(&mut ws)))
                    .with_span(tok.span)?;
            } else if self.include_whitespace {
                ws += 1;
                self.ws.try_push_back(tok).with_span(tok.span)?;
            }
        }

        Ok(())
    }
}

#[inline]
fn emit(tree: &mut syntree::Builder<Kind, Flavor>, tok: Token) -> Result<()> {
    let span = syntree::Span::new(tok.span.start.0, tok.span.end.0);
    tree.token_with(tok.kind, span).with_span(tok.span)?;
    Ok(())
}

/// A source adapter.
pub(super) struct Source<'a> {
    inner: SourceInner<'a>,
}

impl<'a> Source<'a> {
    /// Construct a source based on a lexer.
    pub(super) fn lexer(lexer: Lexer<'a>) -> Self {
        Self {
            inner: SourceInner::Lexer(lexer),
        }
    }

    /// Construct a source based on a token stream.
    pub(super) fn token_stream(iter: TokenStreamIter<'a>) -> Self {
        Self {
            inner: SourceInner::TokenStream(iter),
        }
    }

    /// Construct a source based on a token stream.
    pub(super) fn node(node: Node<'a>) -> Self {
        Self {
            inner: SourceInner::Node(NodeSource {
                span: node.span(),
                children: node.internal_children(),
            }),
        }
    }

    /// Get the span of the source.
    fn span(&self) -> Option<Span> {
        match &self.inner {
            SourceInner::Lexer(lexer) => Some(lexer.span()),
            SourceInner::TokenStream(token_stream) => token_stream.option_span(),
            SourceInner::Node(source) => Some(source.span),
        }
    }

    /// Get the next token in the stream.
    fn next(&mut self) -> Result<Option<Token>> {
        match &mut self.inner {
            SourceInner::Lexer(lexer) => lexer.next(),
            SourceInner::TokenStream(token_stream) => Ok(token_stream.next()),
            SourceInner::Node(source) => Ok(source.children.next().map(inner_token)),
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
    Node(NodeSource<'a>),
}

impl Advance for Parser<'_> {
    type Error = Error;

    #[inline]
    fn advance(&mut self, n: usize) -> Result<()> {
        Parser::advance(self, n)?;
        Ok(())
    }
}

struct NodeSource<'a> {
    span: Span,
    children: InternalChildren<'a>,
}

impl fmt::Debug for NodeSource<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeSource")
            .field("span", &self.span)
            .finish_non_exhaustive()
    }
}
