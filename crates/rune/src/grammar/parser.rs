use std::collections::VecDeque;
use std::mem::take;

use crate::ast::{Delimiter, Kind, Span, Token};
use crate::parse::{Lexer, ParseError};
use crate::SourceId;
use syntree::{Checkpoint, Tree, TreeBuilder, TreeError};

pub(crate) struct Parser<'a> {
    lexer: Lexer<'a>,
    tree: TreeBuilder<Kind>,
    buf: VecDeque<Token>,
    // The number of whitespace or skippable tokens we've seen.
    ws: usize,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(source: &'a str, source_id: SourceId, shebang: bool) -> Self {
        Self {
            lexer: Lexer::new(source, source_id, shebang),
            tree: TreeBuilder::new(),
            buf: VecDeque::new(),
            ws: 0,
        }
    }

    /// Get a span for the remainder of input.
    fn remainder(&self) -> Span {
        if let Some(front) = self.buf.front() {
            self.lexer.from_span(front.span.start.into_usize())
        } else {
            self.lexer.remainder()
        }
    }

    /// Build the tree that has been parsed.
    pub(crate) fn build(self) -> Result<Tree<Kind>, TreeError> {
        self.tree.build()
    }

    /// Add a checkpoint to the tree.
    pub(crate) fn checkpoint(&mut self) -> Result<Checkpoint, ParseError> {
        match self.tree.checkpoint() {
            Ok(checkpoint) => Ok(checkpoint),
            Err(e) => {
                return Err(ParseError::new(self.remainder(), e));
            }
        }
    }

    /// Push the given kind onto the tree.
    pub(crate) fn open(&mut self, kind: Kind) -> Result<(), ParseError> {
        if let Err(e) = self.tree.open(kind) {
            return Err(ParseError::new(self.remainder(), e));
        }

        Ok(())
    }

    /// Close the current node.
    pub(crate) fn close(&mut self) -> Result<(), ParseError> {
        if let Err(e) = self.tree.close() {
            return Err(ParseError::new(self.lexer.full_span(), e));
        }

        Ok(())
    }

    /// Close the current node at the given checkpoint.
    pub(crate) fn close_at(&mut self, c: Checkpoint, kind: Kind) -> Result<(), ParseError> {
        if let Err(e) = self.tree.close_at(c, kind) {
            return Err(ParseError::new(self.lexer.full_span(), e));
        }

        Ok(())
    }

    /// Skip any leading whitespace.
    pub(crate) fn skip(&mut self) -> Result<(), ParseError> {
        self.count_whitespace()?;

        for _ in 0..take(&mut self.ws) {
            self.bump_inner()?;
        }

        Ok(())
    }

    /// Bump the next token.
    pub(crate) fn bump(&mut self) -> Result<(), ParseError> {
        let n = take(&mut self.ws) + 1;

        for _ in 0..n {
            self.bump_inner()?;
        }

        Ok(())
    }

    fn bump_inner(&mut self) -> Result<(), ParseError> {
        let t = if let Some(t) = self.next()? {
            t
        } else {
            return Ok(());
        };

        self.tree
            .token(t.kind, t.span.len().into_usize())
            .map_err(|error| ParseError::new(t.span, error))?;
        Ok(())
    }

    /// Skip whitespace and eat the next token if it matches the given
    /// predicate.
    pub(crate) fn eat_matching<T>(&mut self, predicate: T) -> Result<bool, ParseError>
    where
        T: Fn(Kind) -> bool,
    {
        if predicate(self.nth(0)?) {
            self.bump()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Skip whitespace and eat specifically one kind of token.
    pub(crate) fn eat(&mut self, kind: Kind) -> Result<bool, ParseError> {
        if self.nth(0)? == kind {
            self.bump()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Bump until we find the given kind.
    pub fn bump_until(&mut self, expected: Kind) -> Result<(), ParseError> {
        loop {
            let kind = self.nth(0)?;

            if kind == Kind::Eof || kind == expected {
                break;
            }

            self.bump()?;
        }

        Ok(())
    }

    /// Bump until we find the given expected kind *and* opening and closing
    /// braces / brackets / parens are balanced.
    pub fn bump_until_closed(&mut self, delimiter: Delimiter) -> Result<(), ParseError> {
        let mut balance = 0u32;

        loop {
            match self.nth(0)? {
                Kind::Eof => {
                    break;
                }
                Kind::Open(d) if d == delimiter => {
                    balance = balance.checked_add(1).ok_or_else(|| {
                        ParseError::msg(self.remainder(), "balance out of bounds")
                    })?;
                }
                Kind::Close(d) if d == delimiter => {
                    if balance == 0 {
                        break;
                    }

                    balance = balance.saturating_sub(1);
                }
                _ => {}
            }

            self.bump()?;
        }

        Ok(())
    }

    /// Peek the nth token.
    pub fn nth(&mut self, n: usize) -> Result<Kind, ParseError> {
        self.count_whitespace()?;
        let n = self.ws.saturating_add(n);
        self.nth_with_whitespace(n)
    }

    /// Inner nth implementation which ignores whitespace.
    fn nth_with_whitespace(&mut self, n: usize) -> Result<Kind, ParseError> {
        self.fill(n)?;

        let t = match self.buf.get(n) {
            Some(t) => t,
            None => return Ok(Kind::Eof),
        };

        Ok(t.kind)
    }

    /// Ensure that the given number of elements are filled.
    fn fill(&mut self, n: usize) -> Result<(), ParseError> {
        while self.buf.len() <= n {
            if let Some(t) = self.lexer.next()? {
                self.buf.push_back(t);
            } else {
                break;
            }
        }

        Ok(())
    }

    /// Consume and return the next token.
    fn next(&mut self) -> Result<Option<Token>, ParseError> {
        loop {
            let t = if let Some(t) = self.buf.pop_front() {
                t
            } else if let Some(t) = self.lexer.next()? {
                t
            } else {
                return Ok(None);
            };

            return Ok(Some(t));
        }
    }

    /// Skip over whitespace or other whitespace-like tokens.
    fn count_whitespace(&mut self) -> Result<(), ParseError> {
        use Kind::*;

        if self.ws == 0 {
            while let Whitespace | Comment | MultilineComment(..) =
                self.nth_with_whitespace(self.ws)?
            {
                self.ws += 1;
            }
        }

        Ok(())
    }
}
