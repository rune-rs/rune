use crate::alloc::VecDeque;

use crate::ast::{Kind, Span, Token};
use crate::compile::{Error, ErrorKind, Result, WithSpan};
use crate::parse::{Advance, Lexer, Peekable};
use crate::shared::rune_trace;
use crate::SourceId;

use super::Tree;

use Kind::*;

macro_rules! ws {
    () => {
        Whitespace | Comment | MultilineComment(..)
    };
}

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

pub(crate) struct Parser<'a> {
    lexer: Lexer<'a>,
    buf: VecDeque<Token>,
    source: &'a str,
    tree: syntree::Builder<Kind, u32, usize>,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(source: &'a str) -> Self {
        Self {
            lexer: Lexer::new(source, SourceId::new(0), true).without_processing(),
            buf: VecDeque::new(),
            source,
            tree: syntree::Builder::new(),
        }
    }

    /// Generate an error encompassing the current token.
    pub(super) fn unsupported(&mut self, at: usize, what: &'static str) -> Result<Error> {
        let tok = self.glued_token(at)?;

        Ok(Error::new(
            tok.span,
            ErrorKind::UnsupportedToken {
                actual: tok.kind,
                what,
            },
        ))
    }

    /// Generate an error encompassing the from span.
    #[tracing::instrument(skip_all)]
    pub(super) fn error(&mut self, from: Span, kind: ErrorKind) -> Result<Error> {
        let to = self.glued_token(0)?;
        let span = from.join(to.span);
        Ok(Error::new(span, kind))
    }

    /// Test if we are at EOF.
    #[tracing::instrument(skip_all)]
    pub(super) fn is_eof(&mut self) -> Result<bool> {
        self.ws()?;
        Ok(self.glued(0)? == Eof)
    }

    /// Construct the syntax tree.
    #[tracing::instrument(skip_all)]
    pub(crate) fn build(self) -> Result<Tree> {
        let tree = self
            .tree
            .build()
            .with_span(Span::new(0, self.source.len()))?;

        Ok(Tree::new(tree))
    }

    #[tracing::instrument(skip_all)]
    pub(super) fn checkpoint(&mut self) -> Result<Checkpoint> {
        let span = self.ws()?;

        Ok(Checkpoint {
            span,
            inner: self.tree.checkpoint().with_span(span)?,
        })
    }

    #[tracing::instrument(skip_all)]
    pub(super) fn bump(&mut self) -> Result<()> {
        let tok = self.next()?;
        self.tree
            .token(tok.kind, tok.span.range().len())
            .with_span(tok.span)?;
        Ok(())
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

    /// Bump and immediately close a token with the specified kind.
    #[tracing::instrument(skip_all)]
    pub(super) fn push(&mut self, kind: Kind) -> Result<()> {
        let tok = self.next()?;
        self.tree.open(kind).with_span(tok.span)?;
        self.tree
            .token(tok.kind, tok.span.range().len())
            .with_span(tok.span)?;
        self.tree.close().with_span(tok.span)?;
        Ok(())
    }

    /// Bump an empty node.
    #[tracing::instrument(skip_all)]
    pub(super) fn empty(&mut self, kind: Kind) -> Result<()> {
        let span = self.glued_token(0)?.span;
        self.tree.token(kind, 0).with_span(span)?;
        Ok(())
    }

    /// Close a node at the given checkpoint.
    #[tracing::instrument(skip_all)]
    pub(super) fn close_at(&mut self, c: &Checkpoint, kind: Kind) -> Result<()> {
        let span = self.glued_token(0)?.span;

        self.tree
            .close_at(&c.inner, kind)
            .with_span(c.span.join(span))?;

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
        Ok(self.glued_token(n)?.kind)
    }

    /// Eat heading whitespace and comments.
    #[tracing::instrument(skip_all)]
    fn ws(&mut self) -> Result<Span> {
        let mut span = Span::new(self.source.len(), self.source.len());

        loop {
            let tok = 'tok: {
                if let Some(tok) = self.buf.front() {
                    break 'tok *tok;
                }

                let Some(tok) = self.lexer.next()? else {
                    return Ok(span);
                };

                self.buf.try_push_back(tok).with_span(span)?;
                tok
            };

            span = tok.span;

            if !matches!(tok.kind, ws!()) {
                return Ok(tok.span);
            }

            self.tree
                .token(tok.kind, tok.span.range().len())
                .with_span(tok.span)?;

            self.buf.pop_front();
        }
    }

    #[tracing::instrument(skip_all)]
    fn next(&mut self) -> Result<Token> {
        if let Some(tok) = self.buf.pop_front() {
            return Ok(tok);
        }

        let Some(tok) = self.lexer.next()? else {
            return Ok(Token {
                span: Span::new(self.source.len(), self.source.len()),
                kind: Eof,
            });
        };

        rune_trace!("grammar.rs", tok);
        Ok(tok)
    }

    /// Peek the next token skipping over whitespace.
    #[tracing::instrument(skip_all)]
    fn peek_inner(&mut self) -> Result<Kind> {
        self.ws()?;
        self.glued(0)
    }

    fn glued_token(&mut self, n: usize) -> Result<Token> {
        let mut span = Span::new(self.source.len(), self.source.len());

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
                return Ok(Token {
                    span: Span::new(self.source.len(), self.source.len()),
                    kind: Eof,
                });
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

impl<'a> Advance for Parser<'a> {
    type Error = Error;

    #[inline]
    fn advance(&mut self, n: usize) -> Result<()> {
        Parser::advance(self, n)?;
        Ok(())
    }
}

impl<'a> Peekable for Parser<'a> {
    type Error = Error;

    #[inline]
    fn nth(&mut self, n: usize) -> Result<Token> {
        self.ws()?;
        self.nth_token(n)
    }
}
