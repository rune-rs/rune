use std::fmt;
use std::io;
use std::rc::Rc;

use crate::alloc::prelude::*;
use crate::ast::{Kind, Span, Spanned, ToAst, Token};
use crate::compile::{Error, ErrorKind, Result, WithSpan};
#[cfg(feature = "fmt")]
use crate::fmt::Formatter;
use crate::grammar::ws;
use crate::parse::{Expectation, IntoExpectation};
use crate::shared::FixedVec;
use crate::{SourceId, Sources};

pub(crate) trait Ignore<'a> {
    fn ignore(&mut self, node: Node<'a>) -> Result<()>;
}

#[derive(Debug, Default)]
pub(crate) struct Tree {
    inner: syntree::Tree<Kind, u32, usize>,
}

impl Tree {
    pub(super) fn new(inner: syntree::Tree<Kind, u32, usize>) -> Self {
        Self { inner }
    }

    /// Get a reference to a node.
    pub(crate) fn get(&self, id: syntree::pointer::PointerUsize) -> Option<Node<'_>> {
        self.inner.get(id).map(Node::new)
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
            let mut p = Stream::new(node);
            parser(&mut p)?;
            p.end()?;
        }

        Ok(())
    }

    /// Walk the tree.
    #[cfg(feature = "fmt")]
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
    #[inline]
    fn span(&self) -> Span {
        self.span()
    }
}

/// Iterator over the children of a tree.
pub(crate) struct Stream<'a> {
    node: syntree::Node<'a, Kind, u32, usize>,
    iter: Iter<'a>,
    peek: Option<syntree::Node<'a, Kind, u32, usize>>,
}

impl<'a> Stream<'a> {
    pub(crate) fn new(node: syntree::Node<'a, Kind, u32, usize>) -> Self {
        Self {
            node,
            iter: Iter::new(node.first(), node.last()),
            peek: None,
        }
    }

    /// Get a clone of the raw current state of children.
    #[cfg(feature = "fmt")]
    pub(crate) fn children(&self) -> impl Iterator<Item = Node<'a>> + '_ {
        self.iter.clone().map(Node::new)
    }

    /// Get a clone of the raw current state of children.
    #[cfg(feature = "fmt")]
    pub(crate) fn write_remaining(&mut self, o: &mut Formatter<'a>) -> Result<()> {
        o.flush_whitespace(false)?;

        for node in self.peek.take().into_iter().chain(self.iter.by_ref()) {
            o.write_raw(Node::new(node))?;
        }

        Ok(())
    }

    /// Get a clone of the raw current state of children.
    #[cfg(feature = "fmt")]
    pub(crate) fn fmt_remaining_trimmed(&mut self, o: &mut Formatter<'a>) -> Result<()> {
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
    pub(crate) fn expected(&mut self, expected: impl IntoExpectation) -> Error {
        Error::new(
            self.next_span(),
            ErrorKind::ExpectedSyntax {
                expected: expected.into_expectation(),
                actual: self.kind().into_expectation(),
            },
        )
    }

    /// Expect and discard all the given kinds.
    pub(crate) fn all<const N: usize>(&mut self, expected: [Kind; N]) -> Result<()> {
        for kind in expected {
            self.expect(kind)?;
        }

        Ok(())
    }

    /// Require that there is at least one more child node.
    pub(crate) fn expect(&mut self, expected: Kind) -> Result<Node<'a>> {
        let Some(node) = self.next_node() else {
            return Err(Error::new(
                self.next_span(),
                ErrorKind::UnexpectedEndOfSyntaxWith {
                    inside: self.kind().into_expectation(),
                    expected: expected.into_expectation(),
                },
            ));
        };

        if node.value() != expected {
            return Err(Error::new(
                Span::new(node.span().start, node.span().end),
                ErrorKind::ExpectedSyntaxIn {
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

    /// Helper to coerce the next node into an ast element.
    pub(crate) fn ast<T>(&mut self) -> Result<T>
    where
        T: ToAst,
    {
        let Some(node) = self.next_node() else {
            return Err(Error::new(
                self.next_span(),
                ErrorKind::UnexpectedEndOfSyntaxWith {
                    inside: self.kind().into_expectation(),
                    expected: T::into_expectation(),
                },
            ));
        };

        Node::new(node).ast()
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
        let mut count = 0;

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

            if let Some(old) = out.replace(node) {
                o.ignore(Node::new(old))?;
            }

            count += 1;
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
            node,
            count: Some(count),
        })
    }

    /// Read one node equal to the given kind.
    pub(crate) fn one(&mut self, expected: Kind) -> Result<Remaining<'a>> {
        let node = self.try_pump(expected)?;

        let span = match &node {
            Some(node) => node.span(),
            None => self.next_span(),
        };

        let count = Some(usize::from(node.is_some()));

        Ok(Remaining {
            inside: self.kind(),
            expected,
            span,
            node,
            count,
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

    /// Peek the next token of the parser.
    pub(crate) fn token(&mut self) -> Token {
        if let Some(node) = self.peek_node() {
            Token {
                span: Span::new(node.span().start, node.span().end),
                kind: node.value(),
            }
        } else {
            Token {
                span: Span::point(self.node.span().end),
                kind: Kind::Eof,
            }
        }
    }

    /// Get the current parser as a node.
    pub(crate) fn node(&self) -> Node<'a> {
        Node::new(self.node)
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
    #[cfg(feature = "fmt")]
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
        self.iter.find(|node| !matches!(node.value(), ws!()))
    }

    fn next_back_node(&mut self) -> Option<syntree::Node<'a, Kind, u32, usize>> {
        // We walk over comments and whitespace separately when writing
        // nodes to ensure that formatting functions do not need to worry
        // about it here.
        if let Some(node) = self.iter.rfind(|node| !matches!(node.value(), ws!())) {
            return Some(node);
        }

        self.peek.take()
    }
}

impl<'a> Iterator for Stream<'a> {
    type Item = Node<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.next_node().map(Node::new)
    }
}

impl DoubleEndedIterator for Stream<'_> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.next_back_node().map(Node::new)
    }
}

/// A node associated with a tree.
#[derive(Debug, TryClone)]
#[try_clone(crate)]
pub(crate) struct NodeAt {
    tree: Rc<Tree>,
    source_id: SourceId,
    #[try_clone(copy)]
    id: syntree::pointer::PointerUsize,
}

impl NodeAt {
    /// Parse the node being referenced.
    pub(crate) fn parse<P, O>(&self, parser: P) -> Result<O>
    where
        P: FnOnce(&mut Stream<'_>) -> Result<O>,
    {
        let Some(node) = self.tree.get(self.id) else {
            return Err(Error::msg(
                Span::empty(),
                try_format!("missing node {}", self.id.get()),
            ));
        };

        node.parse(parser)
    }

    /// Print the tree to the given writer.
    pub(crate) fn print_with_sources(&self, sources: &Sources) -> Result<()> {
        use std::io::Write;

        let o = std::io::stdout();
        let mut o = o.lock();

        let Some(node) = self.tree.get(self.id) else {
            return Err(Error::msg(
                Span::empty(),
                try_format!("missing node {}", self.id.get()),
            ));
        };

        let source = sources.get(self.source_id);

        for (depth, node) in node.inner.walk().with_depths() {
            if depth < 0 {
                break;
            }

            let n = (depth * 2) as usize;
            let data = node.value();
            let span = Span::new(node.span().start, node.span().end);

            if node.has_children() {
                writeln!(o, "{:n$}{:?}@{}", "", data, span).with_span(span)?;
            } else if let Some(source) = source.and_then(|s| s.get(span.range())) {
                writeln!(o, "{:n$}{:?}@{} {:?}", "", data, span, source).with_span(span)?;
            } else {
                writeln!(o, "{:n$}{:?}@{} +", "", data, span).with_span(span)?;
            }
        }

        Ok(())
    }
}

impl Spanned for NodeAt {
    fn span(&self) -> Span {
        let Some(node) = self.tree.get(self.id) else {
            return Span::empty();
        };

        node.span()
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

    /// Coerce a node into a token.
    pub(crate) fn into_token(self) -> Token {
        Token {
            span: self.span(),
            kind: self.kind(),
        }
    }

    /// Capture the node at the given position associated with its tree..
    pub(crate) fn node_at(&self, source_id: SourceId, tree: Rc<Tree>) -> NodeAt {
        NodeAt {
            tree,
            source_id,
            id: self.inner.id(),
        }
    }

    /// Replace the kind of the node.
    pub(crate) fn replace(&self, kind: Kind) -> Kind {
        self.inner.replace(kind)
    }

    /// Get the last node.
    #[cfg(feature = "fmt")]
    pub(crate) fn last(&self) -> Option<Node<'a>> {
        self.inner.last().map(Node::new)
    }

    /// Get the children as a fixed array ignoring whitespace.
    pub(crate) fn fixed_vec<const N: usize, T>(
        &self,
        factory: fn(Node<'a>) -> T,
    ) -> Option<FixedVec<T, N>> {
        let mut vec = FixedVec::new();

        for node in self
            .inner
            .children()
            .filter(|n| !matches!(n.value(), ws!()))
        {
            if vec.try_push(factory(Node::new(node))).is_err() {
                return None;
            }
        }

        Some(vec)
    }

    /// Get the children as an array ignoring whitespace.
    pub(crate) fn array<const N: usize>(&self) -> Option<[Node<'a>; N]> {
        self.fixed_vec(|n| n)?.try_into_inner()
    }

    /// Get the children as an array ignoring whitespace.
    pub(crate) fn as_array<const N: usize>(&self, expected: [Kind; N]) -> Option<[Node<'a>; N]> {
        let mut vec = FixedVec::new();

        for (node, expected) in self
            .inner
            .children()
            .filter(|n| !matches!(n.value(), ws!()))
            .zip(expected)
        {
            if node.value() != expected {
                return None;
            }

            if vec.try_push(Node::new(node)).is_err() {
                return None;
            }
        }

        vec.try_into_inner()
    }

    /// Iterate over the children of the node.
    pub(crate) fn children(&self) -> impl DoubleEndedIterator<Item = Node<'a>> + '_ {
        self.inner.children().map(Node::new)
    }

    /// Helper to coerce a node into an ast element.
    pub(crate) fn ast<T>(self) -> Result<T>
    where
        T: ToAst,
    {
        T::to_ast(self.span(), self.kind())
    }

    /// Construct an unsupported error.
    #[cfg(feature = "fmt")]
    pub(crate) fn unsupported(&self, expected: impl IntoExpectation) -> Error {
        Error::new(
            self.span(),
            ErrorKind::ExpectedSyntax {
                expected: expected.into_expectation(),
                actual: self.kind().into_expectation(),
            },
        )
    }

    /// Write the remaining token, or fallback to the given literal if unavailable.
    #[cfg(feature = "fmt")]
    pub(crate) fn fmt(self, o: &mut Formatter<'a>) -> Result<()> {
        o.write_owned(self)
    }

    /// Walk from the current node.
    #[cfg(feature = "fmt")]
    pub(crate) fn walk_from(&self) -> impl Iterator<Item = Node<'a>> + '_ {
        self.inner.walk_from().map(Node::new)
    }

    /// Run the given parser.
    pub(crate) fn parse<P, O>(self, parser: P) -> Result<O>
    where
        P: FnOnce(&mut Stream<'a>) -> Result<O>,
    {
        let mut p = Stream::new(self.inner);
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
    #[cfg(feature = "fmt")]
    pub(crate) fn is_whitespace(&self) -> bool {
        matches!(self.inner.value(), Kind::Whitespace)
    }

    /// Test if the node has children.
    #[cfg(feature = "fmt")]
    pub(crate) fn has_children(&self) -> bool {
        self.inner.has_children()
    }

    /// Find a node which matches the given kind.
    pub(crate) fn find(&self, kind: Kind) -> Option<Node<'a>> {
        self.inner
            .children()
            .find(|n| n.value() == kind)
            .map(Node::new)
    }

    /// Report an unsupported error for the current node.
    pub(crate) fn expected(self, expected: impl IntoExpectation) -> Error {
        Error::new(
            self.span(),
            ErrorKind::ExpectedSyntax {
                expected: expected.into_expectation(),
                actual: self.kind().into_expectation(),
            },
        )
    }
}

impl IntoExpectation for Node<'_> {
    #[inline]
    fn into_expectation(self) -> Expectation {
        self.inner.value().into_expectation()
    }
}

impl Spanned for Node<'_> {
    #[inline]
    fn span(&self) -> Span {
        Node::span(self)
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
    node: Option<Node<'a>>,
    count: Option<usize>,
}

impl<'a> Remaining<'a> {
    /// Ensure that there is exactly one node represented by this container.
    ///
    /// This only fires if the remaining set has been constructed from a stream.
    pub(crate) fn exactly_one(self, o: &mut dyn Ignore<'a>) -> Result<()> {
        if let Some(node) = self.node {
            o.ignore(node)?;
        }

        if let Some(0) = self.count {
            return Err(Error::new(
                self.span,
                ErrorKind::ExpectedOne {
                    inside: self.inside.into_expectation(),
                    expected: self.expected.into_expectation(),
                },
            ));
        }

        Ok(())
    }

    /// Ensure that there are at most one node represented by this container.
    ///
    /// This only fires if the remaining set has been constructed from a stream.
    pub(crate) fn at_most_one(self, o: &mut dyn Ignore<'a>) -> Result<()> {
        if let Some(node) = self.node {
            o.ignore(node)?;
        }

        if let Some(count) = self.count {
            if count > 1 {
                return Err(Error::new(
                    self.span,
                    ErrorKind::ExpectedAtMostOne {
                        inside: self.inside.into_expectation(),
                        expected: self.expected.into_expectation(),
                        count,
                    },
                ));
            }
        }

        Ok(())
    }

    /// Test if there is a remaining node present.
    #[inline]
    #[cfg(feature = "fmt")]
    pub(crate) fn is_present(&self) -> bool {
        self.node.is_some()
    }

    /// Write the remaining token, or fallback to the given literal if unavailable.
    #[cfg(feature = "fmt")]
    pub(crate) fn fmt(self, o: &mut Formatter<'a>) -> Result<bool> {
        self.write_if(o, true)
    }

    /// Write the remaining token, or fallback to the given literal if
    /// unavailable and needed.
    #[cfg(feature = "fmt")]
    pub(crate) fn write_if(mut self, o: &mut Formatter<'a>, needed: bool) -> Result<bool> {
        if self.count.is_none() {
            return Ok(false);
        }

        if let Some(node) = self.node.take() {
            o.write_owned(node)?;
        } else if needed {
            o.lit(self.lit()?)?;
        }

        Ok(true)
    }

    /// Write the remaining token, or fallback to the given literal if
    /// unavailable and even then only if it's needed.
    #[cfg(feature = "fmt")]
    pub(crate) fn write_only_if(mut self, o: &mut Formatter<'a>, needed: bool) -> Result<()> {
        if let Some(node) = self.node.take() {
            if needed {
                o.write_owned(node)?;
            } else {
                o.ignore(node)?;
            }
        } else if needed {
            o.lit(self.lit()?)?;
        }

        Ok(())
    }

    /// Ignore the remaining token.
    pub(crate) fn ignore(self, o: &mut dyn Ignore<'a>) -> Result<bool> {
        if self.count.is_none() {
            return Ok(false);
        }

        if let Some(node) = self.node {
            o.ignore(node)?;
        }

        Ok(true)
    }

    #[cfg(feature = "fmt")]
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
            node: None,
            count: None,
        }
    }
}

#[derive(Clone)]
struct Iter<'a> {
    first: Option<syntree::Node<'a, Kind, u32, usize>>,
    last: Option<syntree::Node<'a, Kind, u32, usize>>,
}

impl<'a> Iter<'a> {
    fn new(
        first: Option<syntree::Node<'a, Kind, u32, usize>>,
        last: Option<syntree::Node<'a, Kind, u32, usize>>,
    ) -> Self {
        Self { first, last }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = syntree::Node<'a, Kind, u32, usize>;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.first.take()?;

        if Some(node.id()) == self.last.map(|n| n.id()) {
            self.last = None;
        } else {
            self.first = node.next();
        }

        Some(node)
    }
}

impl<'a> DoubleEndedIterator for Iter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let node = self.last.take()?;

        if Some(node.id()) == self.first.map(|n| n.id()) {
            self.first = None;
        } else {
            self.last = node.prev();
        }

        Some(node)
    }
}
