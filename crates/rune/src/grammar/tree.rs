use std::fmt;
use std::io;

use crate::ast::{Kind, Span, Spanned, Token};
use crate::compile::{Error, ErrorKind, Result};
#[cfg(feature = "fmt")]
use crate::fmt::Output;
use crate::grammar::ws;
use crate::indexing::Indexer;
use crate::parse::{Expectation, IntoExpectation, ToAst};

pub(crate) trait Ignore<'a> {
    fn ignore(&mut self, node: Node<'a>) -> Result<()>;
}

#[derive(Default)]
pub(crate) struct Tree {
    inner: syntree::Tree<Kind, u32, usize>,
}

impl Tree {
    pub(super) fn new(inner: syntree::Tree<Kind, u32, usize>) -> Self {
        Self { inner }
    }

    /// Iterate over all the children of the tree.
    pub(crate) fn parse_all<'a, P>(&'a self, mut parser: P) -> Result<()>
    where
        P: FnMut(&mut Stream<'a>) -> Result<()>,
    {
        for node in self
            .inner
            .children()
            .filter(|n| !matches!(n.value(), ws!()))
        {
            let mut p = Stream {
                node,
                iter: node.children(),
                peek: None,
            };

            parser(&mut p)?;
            p.end()?;
        }

        Ok(())
    }

    /// Walk the tree.
    pub(crate) fn walk(&self) -> impl Iterator<Item = Node<'_>> {
        self.inner.walk().map(Node::new)
    }

    /// Print the tree to the given writer.
    pub(crate) fn print_with_source<O>(&self, o: &mut O, source: &str) -> Result<()>
    where
        O: io::Write,
    {
        syntree::print::print_with_source(o, &self.inner, source).map_err(|error| {
            Error::msg(
                Span::new(0, source.len()),
                format!("Failed to print tree: {error}"),
            )
        })?;

        Ok(())
    }
}

impl Spanned for Stream<'_> {
    fn span(&self) -> Span {
        self.span()
    }
}

/// Iterator over the children of a tree.
pub(crate) struct Stream<'a> {
    node: syntree::Node<'a, Kind, u32, usize>,
    iter: syntree::node::Children<'a, Kind, u32, usize>,
    peek: Option<syntree::Node<'a, Kind, u32, usize>>,
}

impl<'a> Stream<'a> {
    /// Construct an error message.
    pub(crate) fn msg<M>(&mut self, message: M) -> Error
    where
        M: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        Error::msg(self.next_span(), message)
    }

    /// Get a clone of the raw current state of children.
    pub(crate) fn children(&self) -> impl Iterator<Item = Node<'a>> + '_ {
        self.iter.clone().map(Node::new)
    }

    /// Get a clone of the raw current state of children.
    #[cfg(feature = "fmt")]
    pub(crate) fn write_remaining(&mut self, o: &mut Output<'a>) -> Result<()> {
        o.flush_whitespace(false)?;

        for node in self.peek.take().into_iter().chain(self.iter.by_ref()) {
            o.write_raw(Node::new(node))?;
        }

        Ok(())
    }

    /// Get a clone of the raw current state of children.
    #[cfg(feature = "fmt")]
    pub(crate) fn write_remaining_trimmed(&mut self, o: &mut Output<'a>) -> Result<()> {
        o.flush_whitespace(false)?;

        let mut buf = None;
        let mut first = true;
        let mut last_was_line = false;

        let iter = self.peek.take().into_iter().chain(self.iter.by_ref());

        for node in iter {
            if node.has_children() {
                continue;
            }

            if matches!(node.value(), Kind::Whitespace) {
                buf = Some(Node::new(node));
                continue;
            }

            if !first {
                if let Some(buf) = buf.take() {
                    o.write_raw(buf)?;
                }
            }

            last_was_line = matches!(node.value(), Kind::Comment);
            o.write_raw(Node::new(node))?;
            first = false;
        }

        // Since we've trimmed the last whitespace, we need to add the
        // corresponding number of lines here.
        if last_was_line {
            o.nl(1)?;
        }

        Ok(())
    }

    /// Get the kind of the current node.
    pub(crate) fn kind(&self) -> Kind {
        self.node.value()
    }

    /// Test if the parser is at the end of input.
    pub(crate) fn is_eof(&mut self) -> bool {
        matches!(self.peek(), Kind::Eof)
    }

    /// Peek the next node.
    pub(crate) fn peek(&mut self) -> Kind {
        if let Some(value) = self.peek_node() {
            return value.value();
        }

        Kind::Eof
    }

    fn peek_node(&mut self) -> Option<&syntree::Node<Kind, u32, usize>> {
        if self.peek.is_none() {
            if let Some(node) = self.next_node() {
                self.peek = Some(node);
            }
        }

        self.peek.as_ref()
    }

    /// Report an unsupported error for the current tree parser.
    pub(crate) fn unsupported(&mut self, what: &'static str) -> Error {
        Error::new(
            self.next_span(),
            ErrorKind::UnsupportedSyntax {
                what,
                actual: self.kind().into_expectation(),
            },
        )
    }

    /// Require that there is at least one more child node.
    pub(crate) fn expect(&mut self, expected: Kind) -> Result<Node<'a>> {
        let Some(node) = self.next_node() else {
            return Err(Error::new(
                self.next_span(),
                ErrorKind::UnexpectedEndOfSyntax {
                    inside: self.kind().into_expectation(),
                },
            ));
        };

        if node.value() != expected {
            return Err(Error::new(
                Span::new(node.span().start, node.span().end),
                ErrorKind::ExpectedSyntax {
                    inside: self.kind().into_expectation(),
                    expected: expected.into_expectation(),
                    actual: node.value().into_expectation(),
                },
            ));
        }

        Ok(Node::new(node))
    }

    /// Require that there is at least one more child node.
    pub(crate) fn pump(&mut self) -> Result<Node<'a>> {
        let Some(node) = self.next_node() else {
            return Err(Error::new(
                self.next_span(),
                ErrorKind::UnexpectedEndOfSyntax {
                    inside: self.kind().into_expectation(),
                },
            ));
        };

        Ok(Node::new(node))
    }

    /// Try to bump one node.
    pub(crate) fn try_pump(&mut self, expect: Kind) -> Result<Option<Node<'a>>> {
        if let Some(node) = self.next_node() {
            if node.value() == expect {
                return Ok(Some(Node::new(node)));
            }

            self.peek = Some(node);
        }

        Ok(None)
    }

    /// Read remaining nodes equal to the given kind.
    pub(crate) fn remaining(
        &mut self,
        o: &mut dyn Ignore<'a>,
        expected: Kind,
    ) -> Result<Remaining<'a>> {
        let mut first = None;
        let mut out = None;
        let mut found = 0;

        while let Some(node) = self.next_node() {
            if node.value() != expected {
                if let Some(node) = self.peek.replace(node) {
                    o.ignore(Node::new(node))?;
                }

                break;
            }

            if first.is_none() {
                first = Some(Span::new(node.span().start, node.span().end));
            }

            found += 1;

            if let Some(old) = out.replace(node) {
                o.ignore(Node::new(old))?;
            }
        }

        let node = out.map(Node::new);

        let span = match (first, &node) {
            (Some(first), Some(last)) => first.join(last.span()),
            _ => self.next_span(),
        };

        Ok(Remaining {
            inside: self.kind(),
            expected,
            span,
            found,
            node,
        })
    }

    /// Read one node equal to the given kind.
    pub(crate) fn one(&mut self, expected: Kind) -> Result<Remaining<'a>> {
        let node = self.try_pump(expected)?;

        let span = match &node {
            Some(node) => node.span(),
            None => self.next_span(),
        };

        Ok(Remaining {
            inside: self.kind(),
            expected,
            span,
            found: usize::from(node.is_some()),
            node,
        })
    }

    /// Require that the iterator is ended.
    pub(super) fn end(mut self) -> Result<()> {
        if let Some(node) = self.next_node() {
            let inside = self.kind();

            let span = match self.iter.last() {
                Some(last) => node.span().join(last.span()),
                None => *node.span(),
            };

            return Err(Error::new(
                Span::new(span.start, span.end),
                ErrorKind::ExpectedSyntaxEnd {
                    inside: inside.into_expectation(),
                    actual: node.value().into_expectation(),
                },
            ));
        }

        Ok(())
    }

    /// Get the current span of the parser.
    fn next_span(&mut self) -> Span {
        if let Some(node) = self.peek_node() {
            return Span::new(node.span().start, node.span().end);
        }

        Span::point(self.node.span().end)
    }

    pub(crate) fn span(&self) -> Span {
        Span::new(self.node.span().start, self.node.span().end)
    }

    /// Get the next raw node, including whitespace.
    pub(crate) fn next_with_ws(&mut self) -> Option<Node<'a>> {
        if let Some(node) = self.peek.take() {
            return Some(Node::new(node));
        }

        self.iter.next().map(Node::new)
    }

    fn next_node(&mut self) -> Option<syntree::Node<'a, Kind, u32, usize>> {
        if let Some(node) = self.peek.take() {
            return Some(node);
        }

        // We walk over comments and whitespace separately when writing
        // nodes to ensure that formatting functions do not need to worry
        // about it here.
        self.iter
            .by_ref()
            .find(|node| !matches!(node.value(), ws!()))
    }
}

impl<'a> Iterator for Stream<'a> {
    type Item = Node<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.next_node().map(Node::new)
    }
}

/// A node being parsed.
#[derive(Clone)]
pub(crate) struct Node<'a> {
    inner: syntree::Node<'a, Kind, u32, usize>,
}

impl<'a> Node<'a> {
    pub(super) fn new(inner: syntree::Node<'a, Kind, u32, usize>) -> Self {
        Self { inner }
    }

    /// Construct an error message.
    pub(crate) fn msg<M>(&self, message: M) -> Error
    where
        M: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        Error::msg(self.span(), message)
    }

    pub(crate) fn unsupported(&self, what: &'static str) -> Error {
        Error::new(
            self.span(),
            ErrorKind::UnsupportedSyntax {
                what,
                actual: self.kind().into_expectation(),
            },
        )
    }

    /// Construct a stream over the current node.
    pub(crate) fn into_stream(self) -> Stream<'a> {
        Stream {
            node: self.inner,
            iter: self.inner.children(),
            peek: None,
        }
    }

    /// Walk from the current node.
    pub(crate) fn walk_from(&self) -> impl Iterator<Item = Node<'a>> + '_ {
        self.inner.walk_from().map(Node::new)
    }

    /// Run the given parser.
    pub(crate) fn parse<P, O>(self, parser: P) -> Result<O>
    where
        P: FnOnce(&mut Stream<'a>) -> Result<O>,
    {
        let mut p = self.into_stream();
        let out = parser(&mut p)?;
        p.end()?;
        Ok(out)
    }

    /// Construct a span for the current node.
    pub(crate) fn span(&self) -> Span {
        let span = self.inner.span();
        Span::new(span.start, span.end)
    }

    /// Get the kind of the node.
    pub(crate) fn kind(&self) -> Kind {
        self.inner.value()
    }

    /// Test if the current node is whitespace.
    pub(crate) fn is_whitespace(&self) -> bool {
        matches!(self.inner.value(), Kind::Whitespace)
    }

    /// Test if the node has children.
    pub(crate) fn has_children(&self) -> bool {
        self.inner.has_children()
    }

    /// Coerce a node into an ast node.
    pub(crate) fn ast<T>(&self) -> Result<T>
    where
        T: ToAst,
    {
        T::to_ast(Token {
            span: self.span(),
            kind: self.kind(),
        })
    }
}

impl fmt::Debug for Node<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

#[must_use = "Remaining nodes must be consumed to capture all whitespace and comments"]
pub(crate) struct Remaining<'a> {
    inside: Kind,
    expected: Kind,
    span: Span,
    found: usize,
    node: Option<Node<'a>>,
}

impl<'a> Remaining<'a> {
    /// Test if there is a remaining node present.
    #[inline]
    pub(crate) fn is_present(&self) -> bool {
        self.node.is_some()
    }

    /// Write the remaining token, or fallback to the given literal if unavailable.
    #[cfg(feature = "fmt")]
    pub(crate) fn write(self, o: &mut Output<'a>) -> Result<()> {
        self.write_if(o, true)
    }

    /// Write the remaining token, or fallback to the given literal if
    /// unavailable and needed.
    #[cfg(feature = "fmt")]
    pub(crate) fn write_if(mut self, o: &mut Output<'a>, needed: bool) -> Result<()> {
        if let Some(node) = self.node.take() {
            o.write(node)?;
        } else if needed {
            o.lit(self.lit()?)?;
        }

        Ok(())
    }

    /// Write the remaining token, or fallback to the given literal if
    /// unavailable and even then only if it's needed.
    #[cfg(feature = "fmt")]
    pub(crate) fn write_only_if(mut self, o: &mut Output<'a>, needed: bool) -> Result<()> {
        if let Some(node) = self.node.take() {
            if needed {
                o.write(node)?;
            } else {
                o.ignore(node)?;
            }
        } else if needed {
            o.lit(self.lit()?)?;
        }

        Ok(())
    }

    /// Ignore the remaining token.
    pub(crate) fn ignore(self, o: &mut dyn Ignore<'a>) -> Result<()> {
        if let Some(node) = self.node {
            o.ignore(node)?;
        }

        Ok(())
    }

    /// Parse the current token or report an error if it's not available.
    pub(crate) fn one(self, cx: &mut Indexer<'_, '_>) -> Result<()> {
        if self.node.is_none() {
            cx.error(Error::new(
                self.span,
                ErrorKind::ExpectedSyntax {
                    inside: self.inside.into_expectation(),
                    expected: self.expected.into_expectation(),
                    actual: self.node.map_or(Kind::Eof, |n| n.kind()).into_expectation(),
                },
            ))?;
        }

        if self.found > 1 {
            cx.error(Error::new(
                self.span,
                ErrorKind::MoreThanOneElement {
                    expected: self.expected.into_expectation(),
                    actual: self.found,
                },
            ))?;
        }

        Ok(())
    }

    /// Parse the current token or report an error if it's not available.
    pub(crate) fn at_most_one(self, cx: &mut Indexer<'_, '_>) -> Result<()> {
        if self.found > 1 {
            cx.error(Error::new(
                self.span,
                ErrorKind::MoreThanOneElement {
                    expected: self.expected.into_expectation(),
                    actual: self.found,
                },
            ))?;
        }

        Ok(())
    }

    fn lit(&self) -> Result<&'static str> {
        let lit = match self.expected.into_expectation() {
            Expectation::Keyword(lit) => lit,
            Expectation::Delimiter(lit) => lit,
            Expectation::Punctuation(lit) => lit,
            expectation => {
                return Err(Error::new(
                    self.span,
                    ErrorKind::UnsupportedDelimiter { expectation },
                ));
            }
        };

        Ok(lit)
    }
}

impl<'a> Default for Remaining<'a> {
    fn default() -> Self {
        Self {
            inside: Kind::Root,
            expected: Kind::Eof,
            span: Span::empty(),
            found: 0,
            node: None,
        }
    }
}
