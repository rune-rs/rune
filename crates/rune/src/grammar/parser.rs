use core::fmt;
use core::mem::take;

use crate::alloc::VecDeque;

use crate::ast::{Kind, OptionSpanned, Span, Token};
use crate::compile::{Error, ErrorKind, Result, WithSpan};
use crate::grammar::ws;
use crate::macros::TokenStreamIter;
use crate::parse::{Advance, IntoExpectation, Lexer};
use crate::shared::{rune_trace, FixedVec};

use super::Tree;

use Kind::*;

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
    buf: VecDeque<Token>,
    tree: syntree::Builder<Kind, u32, usize>,
    /// The current whitespace offset in use.
    ws: usize,
}

impl<'a> Parser<'a> {
    pub(super) fn new(source: Source<'a>) -> Self {
        Self {
            lexer: source,
            buf: VecDeque::new(),
            tree: syntree::Builder::new(),
            ws: 0,
        }
    }

    /// Generate an error encompassing the current token.
    pub(super) fn expected_at(
        &mut self,
        at: usize,
        expected: impl IntoExpectation,
    ) -> Result<Error> {
        self.ws()?;
        let tok = self.glued_token(at)?;

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
        self.ws()?;
        let to = self.glued_token(0)?;
        let span = from.join(to.span);
        Ok(Error::new(span, kind))
    }

    /// Test if we are at EOF.
    #[tracing::instrument(skip_all)]
    pub(super) fn is_eof(&mut self) -> Result<bool> {
        Ok(self.glued(0)? == Eof)
    }

    /// Construct the syntax tree.
    #[tracing::instrument(skip_all)]
    pub(crate) fn build(self) -> Result<Tree> {
        let tree = self
            .tree
            .build()
            .with_span(self.lexer.span().unwrap_or_else(Span::empty))?;

        Ok(Tree::new(tree))
    }

    #[tracing::instrument(skip_all)]
    pub(super) fn checkpoint(&mut self) -> Result<Checkpoint> {
        let span = self.ws()?;
        self.flush_ws()?;

        Ok(Checkpoint {
            span: span.tail(),
            inner: self.tree.checkpoint().with_span(span)?,
        })
    }

    #[tracing::instrument(skip_all)]
    pub(super) fn bump(&mut self) -> Result<Token> {
        let tok = self.next()?;
        let span = syntree::Span::new(tok.span.start.0, tok.span.end.0);
        self.tree.token_with(tok.kind, span).with_span(tok.span)?;
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
    pub(super) fn bump_if(&mut self, kind: Kind) -> Result<bool> {
        if self.peek()? == kind {
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
        let span = self.glued_token(0)?.span;
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
        self.ws()?;
        let tok = self.glued_token(0)?;
        Ok(tok.kind)
    }

    #[tracing::instrument(skip(self))]
    pub(super) fn glued(&mut self, n: usize) -> Result<Kind> {
        self.ws()?;
        Ok(self.glued_token(n)?.kind)
    }

    /// Eat heading whitespace and comments.
    #[tracing::instrument(skip_all)]
    fn ws(&mut self) -> Result<Span> {
        let mut span = self.lexer.span().unwrap_or_else(Span::empty);

        loop {
            let tok = 'tok: {
                if self.buf.len() <= self.ws {
                    let Some(tok) = self.lexer.next()? else {
                        return Ok(span.tail());
                    };

                    self.buf.try_push_back(tok).with_span(span)?;
                    break 'tok tok;
                }

                let Some(tok) = self.buf.get(self.ws) else {
                    return Ok(span.tail());
                };

                *tok
            };

            span = tok.span;

            if !matches!(tok.kind, ws!()) {
                return Ok(tok.span);
            }

            self.ws += 1;
        }
    }

    /// Flush whitespace.
    pub(crate) fn flush_ws(&mut self) -> Result<()> {
        self.ws()?;

        for tok in self.buf.drain(..take(&mut self.ws)) {
            let span = syntree::Span::new(tok.span.start.0, tok.span.end.0);
            self.tree.token_with(tok.kind, span).with_span(tok.span)?;
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn next(&mut self) -> Result<Token> {
        self.flush_ws()?;

        if let Some(tok) = self.buf.pop_front() {
            return Ok(tok);
        }

        let Some(tok) = self.lexer.next()? else {
            let span = self.lexer.span().unwrap_or_else(Span::empty).tail();
            return Ok(Token { span, kind: Eof });
        };

        rune_trace!("grammar.rs", tok);
        Ok(tok)
    }

    fn glued_token(&mut self, n: usize) -> Result<Token> {
        let n = self.ws + n;
        let mut span = self.lexer.span().unwrap_or_else(Span::empty);

        while self.buf.len() <= n {
            let Some(tok) = self.lexer.next()? else {
                break;
            };

            self.buf.try_push_back(tok).with_span(tok.span)?;
            span = tok.span;
        }

        let Some(tok) = self.buf.get(n).copied() else {
            return Ok(Token { span, kind: Eof });
        };

        rune_trace!("grammar.rs", tok);
        Ok(tok)
    }

    fn advance(&mut self, mut n: usize) -> Result<()> {
        while n > 0 {
            let tok = 'tok: {
                if let Some(tok) = self.buf.pop_front() {
                    break 'tok tok;
                };

                if let Some(tok) = self.lexer.next()? {
                    break 'tok tok;
                };

                return Ok(());
            };

            self.tree
                .token(tok.kind, tok.span.range().len())
                .with_span(tok.span)?;

            n -= usize::from(!matches!(tok.kind, ws!()));
        }

        Ok(())
    }

    #[inline]
    pub(super) fn nth(&mut self, n: usize) -> Result<Kind> {
        self.nth_token(n).map(|tok| tok.kind)
    }

    /// Access an array.
    pub(super) fn array<const N: usize>(&mut self) -> Result<FixedVec<Token, N>> {
        let mut vec = FixedVec::new();

        for index in 0.. {
            if vec.len() == N {
                break;
            }

            while self.buf.len() <= index {
                let Some(tok) = self.lexer.next()? else {
                    break;
                };

                self.buf.try_push_back(tok).with_span(tok.span)?;
            }

            if let Some(tok) = self.buf.get(index) {
                if !matches!(tok.kind, ws!()) {
                    vec.try_push(*tok).with_span(tok.span)?;
                }

                continue;
            }

            let span = self.lexer.span().unwrap_or_else(Span::empty);
            vec.try_push(Token { span, kind: Eof }).with_span(span)?;
        }

        Ok(vec)
    }

    fn nth_token(&mut self, n: usize) -> Result<Token> {
        let mut index = 0;
        let mut remaining = n;

        loop {
            while self.buf.len() <= index {
                let Some(tok) = self.lexer.next()? else {
                    break;
                };

                self.buf.try_push_back(tok).with_span(tok.span)?;
            }

            let Some(tok) = self.buf.get(index) else {
                let span = self.lexer.span().unwrap_or_else(Span::empty);
                return Ok(Token { span, kind: Eof });
            };

            if !matches!(tok.kind, ws!()) {
                if remaining == 0 {
                    return Ok(*tok);
                }

                remaining -= 1;
            }

            index += 1;
        }
    }
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

    /// Get the span of the source.
    fn span(&self) -> Option<Span> {
        match &self.inner {
            SourceInner::Lexer(lexer) => Some(lexer.span()),
            SourceInner::TokenStream(token_stream) => token_stream.option_span(),
        }
    }

    /// Get the next token in the stream.
    fn next(&mut self) -> Result<Option<Token>> {
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

impl<'a> Advance for Parser<'a> {
    type Error = Error;

    #[inline]
    fn advance(&mut self, n: usize) -> Result<()> {
        Parser::advance(self, n)?;
        Ok(())
    }
}
