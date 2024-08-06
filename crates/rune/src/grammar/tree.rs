use std::fmt;
use std::io;

use crate::ast::{Kind, Span};
use crate::compile::{Error, ErrorKind, Result};
#[cfg(feature = "fmt")]
use crate::fmt::Output;
use crate::parse::{Expectation, IntoExpectation};

pub(crate) struct Tree {
    inner: syntree::Tree<Kind, u32, usize>,
}

impl Tree {
    pub(super) fn new(inner: syntree::Tree<Kind, u32, usize>) -> Self {
        Self { inner }
    }

    /// Iterate over all the children of the tree.
    pub(crate) fn parse<'a, P, O>(&'a self, parser: P) -> Result<O>
    where
        P: FnOnce(&mut Stream<'a>) -> Result<O>,
    {
        let mut p = Stream {
            node: None,
            iter: self.inner.children(),
            peek: None,
        };

        let out = parser(&mut p)?;
        p.end()?;
        Ok(out)
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

/// Iterator over the children of a tree.
pub(crate) struct Stream<'a> {
    node: Option<syntree::Node<'a, Kind, u32, usize>>,
    iter: syntree::node::Children<'a, Kind, u32, usize>,
    peek: Option<syntree::Node<'a, Kind, u32, usize>>,
}

impl<'a> Stream<'a> {
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
        let Some(node) = &self.node else {
            return Kind::Root;
        };

        node.value()
    }

    /// Test if the parser is at the end of input.
    pub(crate) fn is_eof(&mut self) -> bool {
        matches!(self.peek(), Kind::Eof)
    }

    /// Peek the next node.
    pub(crate) fn peek(&mut self) -> Kind {
        if let Some(node) = &self.peek {
            return node.value();
        }

        if let Some(node) = self.next_node() {
            self.peek = Some(node);
            return node.value();
        }

        Kind::Eof
    }

    /// Report an unsupported error for the current tree parser.
    pub(crate) fn unsupported(&self, what: &'static str) -> Error {
        Error::new(
            self.span(),
            ErrorKind::UnsupportedSyntax {
                actual: self.kind(),
                what,
            },
        )
    }

    /// Require that there is at least one more child node.
    pub(crate) fn expect(&mut self, expected: Kind) -> Result<Node<'a>> {
        let Some(node) = self.next_node() else {
            return Err(Error::new(
                self.span(),
                ErrorKind::UnexpectedEndOfSyntax {
                    inside: self.kind(),
                },
            ));
        };

        if node.value() != expected {
            return Err(Error::new(
                Span::new(node.span().start, node.span().end),
                ErrorKind::ExpectedSyntax {
                    inside: self.kind(),
                    expected,
                    actual: node.value(),
                },
            ));
        }

        Ok(Node::new(node))
    }

    /// Require that there is at least one more child node.
    pub(crate) fn pump(&mut self) -> Result<Node<'a>> {
        let Some(node) = self.next_node() else {
            return Err(Error::new(
                self.span(),
                ErrorKind::UnexpectedEndOfSyntax {
                    inside: self.kind(),
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
    #[cfg(feature = "fmt")]
    pub(crate) fn remaining(
        &mut self,
        o: &mut Output<'a>,
        expected: Kind,
    ) -> Result<Remaining<'a>> {
        let lit = self.kind_to_lit(expected)?;

        let mut out = None;

        while let Some(node) = self.next_node() {
            if node.value() != expected {
                if let Some(node) = self.peek.replace(node) {
                    o.ignore(Node::new(node))?;
                }

                break;
            }

            if let Some(old) = out.replace(node) {
                o.ignore(Node::new(old))?;
            }
        }

        Ok(Remaining {
            lit,
            node: out.map(Node::new),
        })
    }

    /// Read one node equal to the given kind.
    pub(crate) fn one(&mut self, expected: Kind) -> Result<Remaining<'a>> {
        let lit = self.kind_to_lit(expected)?;

        Ok(Remaining {
            lit,
            node: self.try_pump(expected)?,
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
                    inside,
                    actual: node.value(),
                },
            ));
        }

        Ok(())
    }

    /// Get the current span of the parser.
    pub(crate) fn span(&self) -> Span {
        let Some(node) = &self.node else {
            return Span::point(0);
        };

        Span::new(node.span().start, node.span().end)
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
        self.iter.by_ref().find(|node| {
            !matches!(
                node.value(),
                Kind::Whitespace | Kind::Comment | Kind::MultilineComment(..)
            )
        })
    }

    fn kind_to_lit(&mut self, expected: Kind) -> Result<&'static str> {
        let lit = match expected.into_expectation() {
            Expectation::Keyword(lit) => lit,
            Expectation::Delimiter(lit) => lit,
            Expectation::Punctuation(lit) => lit,
            expectation => {
                return Err(Error::new(
                    self.span(),
                    ErrorKind::UnsupportedDelimiter { expectation },
                ));
            }
        };

        Ok(lit)
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

    pub(crate) fn unsupported(&self, what: &'static str) -> Error {
        Error::new(
            self.span(),
            ErrorKind::UnsupportedSyntax {
                actual: self.kind(),
                what,
            },
        )
    }

    /// Construct a stream over the current node.
    pub(crate) fn into_stream(self) -> Stream<'a> {
        Stream {
            node: Some(self.inner),
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
}

impl fmt::Debug for Node<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

#[derive(Default)]
#[must_use = "Remaining nodes must be consumed to capture all whitespace and comments"]
pub(crate) struct Remaining<'a> {
    lit: &'static str,
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
    pub(crate) fn write_if(self, o: &mut Output<'a>, needed: bool) -> Result<()> {
        if let Some(node) = self.node {
            o.write(node)?;
        } else if needed {
            o.lit(self.lit)?;
        }

        Ok(())
    }

    /// Write the remaining token, or fallback to the given literal if
    /// unavailable and even then only if it's needed.
    #[cfg(feature = "fmt")]
    pub(crate) fn write_only_if(self, o: &mut Output<'a>, needed: bool) -> Result<()> {
        if let Some(node) = self.node {
            if needed {
                o.write(node)?;
            } else {
                o.ignore(node)?;
            }
        } else if needed {
            o.lit(self.lit)?;
        }

        Ok(())
    }

    /// Ignore the remaining token.
    #[cfg(feature = "fmt")]
    pub(crate) fn ignore(self, o: &mut Output<'a>) -> Result<()> {
        if let Some(node) = self.node {
            o.ignore(node)?;
        }

        Ok(())
    }
}
