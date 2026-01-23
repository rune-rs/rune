use core::fmt;
use core::mem::{replace, take};

use rust_alloc::rc::Rc;

use crate::alloc;
use crate::alloc::prelude::*;
use crate::ast::{Kind, Span, Spanned, ToAst, Token};
use crate::compile::{Error, ErrorKind, Result, WithSpan};
#[cfg(feature = "fmt")]
use crate::fmt::Formatter;
use crate::grammar::ws;
use crate::parse::{Expectation, IntoExpectation};
use crate::shared::FixedVec;
use crate::SourceId;
#[cfg(feature = "std")]
use crate::Sources;

use super::Flavor;

pub(super) type InternalNode<'a> = syntree::Node<'a, Kind, Flavor>;
pub(super) type InternalChildren<'a> = syntree::node::Children<'a, Kind, Flavor>;

pub(crate) trait Ignore<'a> {
    /// Capture an error.
    fn error(&mut self, error: Error) -> alloc::Result<()>;

    /// Ignore the given node.
    fn ignore(&mut self, node: Node<'a>) -> Result<()>;
}

#[derive(Debug, Default)]
pub(crate) struct Tree {
    inner: syntree::Tree<Kind, Flavor>,
}

impl Tree {
    pub(super) fn new(inner: syntree::Tree<Kind, Flavor>) -> Self {
        Self { inner }
    }

    /// Construt a root for the tree.
    pub(crate) fn node_at(self: &Rc<Self>, source_id: SourceId, NodeId(id): NodeId) -> NodeAt {
        NodeAt {
            tree: self.clone(),
            source_id,
            id,
        }
    }

    /// Get a reference to a node.
    pub(crate) fn get(&self, id: syntree::pointer::PointerUsize) -> Option<Node<'_>> {
        self.inner.get(id).map(Node::new)
    }

    /// Get the children as an array ignoring whitespace.
    pub(crate) fn nodes<const N: usize>(&self) -> Option<[Node<'_>; N]> {
        self.fixed_vec(Node::new)?.try_into_inner()
    }

    /// Get the children as a fixed array ignoring whitespace.
    fn fixed_vec<'a, const N: usize, T>(
        &'a self,
        factory: fn(InternalNode<'a>) -> T,
    ) -> Option<FixedVec<T, N>> {
        let mut vec = FixedVec::new();

        for node in self
            .inner
            .children()
            .filter(|n| !matches!(n.value(), ws!()))
        {
            if vec.try_push(factory(node)).is_err() {
                return None;
            }
        }

        Some(vec)
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
    #[cfg(feature = "std")]
    pub(crate) fn print_with_source(
        &self,
        span: &dyn Spanned,
        title: impl fmt::Display,
        source: &str,
    ) -> Result<()> {
        use std::io::Write;

        let o = std::io::stdout();
        let mut o = o.lock();

        writeln!(o, "{title}:").with_span(span)?;

        for (depth, node) in self.inner.walk().with_depths() {
            let n = (depth * 2) as usize;
            let data = node.value();
            let span = node.span();

            if node.has_children() {
                writeln!(o, "{:n$}{:?}@{}", "", data, span).with_span(span)?;
            } else if let Some(source) = source.get(span.range()) {
                writeln!(o, "{:n$}{:?}@{} {:?}", "", data, span, source).with_span(span)?;
            } else {
                writeln!(o, "{:n$}{:?}@{} +", "", data, span).with_span(span)?;
            }
        }

        Ok(())
    }

    /// Print the tree to the given writer.
    #[cfg(feature = "std")]
    pub(crate) fn print(&self, span: &dyn Spanned, title: impl fmt::Display) -> Result<()> {
        use std::io::Write;

        let o = std::io::stdout();
        let mut o = o.lock();

        writeln!(o, "{title}:").with_span(span)?;

        for (depth, node) in self.inner.walk().with_depths() {
            let n = (depth * 2) as usize;
            let data = node.value();
            let span = node.span();

            if node.has_children() {
                writeln!(o, "{:n$}{:?}@{}", "", data, span).with_span(span)?;
            } else {
                writeln!(o, "{:n$}{:?}@{} +", "", data, span).with_span(span)?;
            }
        }

        Ok(())
    }
}

/// Iterator over the children of a tree.
pub(crate) struct StreamBuf<'a> {
    stream: Stream<'a>,
}

impl<'a> StreamBuf<'a> {
    /// Test if the stream is end-of-file.
    pub(crate) fn is_eof(&mut self) -> bool {
        self.stream.is_eof()
    }

    /// Parse the stream being referenced.
    pub(crate) fn parse<P, O>(mut self, parser: P) -> Result<O>
    where
        P: FnOnce(&mut Stream<'a>) -> Result<O>,
    {
        let out = parser(&mut self.stream)?;
        self.stream.end()?;
        Ok(out)
    }
}

impl Spanned for StreamBuf<'_> {
    #[inline]
    fn span(&self) -> Span {
        self.stream.span()
    }
}

impl<'a> Iterator for StreamBuf<'a> {
    type Item = Node<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.stream.next()
    }
}

impl fmt::Debug for StreamBuf<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.stream.fmt(f)
    }
}

/// Iterator over the children of a tree.
pub(crate) struct Stream<'a> {
    node: InternalNode<'a>,
    iter: Iter<'a>,
    peek: Option<InternalNode<'a>>,
}

impl<'a> Stream<'a> {
    pub(crate) fn new(node: InternalNode<'a>) -> Self {
        Self {
            node,
            iter: Iter::new(node.first(), node.last()),
            peek: None,
        }
    }

    /// Ignore the remainder of the stream.
    pub(crate) fn ignore(&mut self) {
        self.iter = Iter::default();
        self.peek = None;
    }

    /// Coerce into remaining inner parser.
    pub(crate) fn take_remaining(&mut self) -> StreamBuf<'a> {
        StreamBuf {
            stream: Stream {
                node: self.node,
                iter: take(&mut self.iter),
                peek: self.peek.take(),
            },
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

    /// Get kinds of all children excluding whitespace.
    pub(crate) fn kinds<const N: usize>(&self) -> Option<[Kind; N]> {
        self.node().kinds()
    }

    /// Get all children excluding whitespace.
    pub(crate) fn nodes<const N: usize>(&self) -> Option<[Node<'a>; N]> {
        self.node().nodes()
    }

    /// Test if the parser is at the end of input.
    pub(crate) fn is_eof(&mut self) -> bool {
        matches!(self.peek(), Kind::Eof)
    }

    /// Get the identifier of the node.
    pub(crate) fn id(&self) -> NodeId {
        NodeId(self.node.id())
    }

    /// Get the current parser as a node.
    #[inline]
    pub(crate) fn node(&self) -> Node<'a> {
        Node::new(self.node)
    }

    /// Peek the next node.
    pub(crate) fn peek(&mut self) -> Kind {
        if let Some(value) = self.peek_node() {
            return value.value();
        }

        Kind::Eof
    }

    /// Get the current span of the parser.
    pub(crate) fn peek_span(&mut self) -> Span {
        if let Some(node) = self.peek_node() {
            return inner_span(node.span());
        }

        Span::point(self.node.span().end)
    }

    fn peek_node(&mut self) -> Option<&InternalNode<'a>> {
        if self.peek.is_none() {
            if let Some(node) = self.next_node() {
                self.peek = Some(node);
            }
        }

        self.peek.as_ref()
    }

    /// Report an unsupported error for the next item being peeked.
    pub(crate) fn expected_peek(&mut self, expected: impl IntoExpectation) -> Error {
        Error::new(
            self.peek_span(),
            ErrorKind::ExpectedSyntax {
                expected: expected.into_expectation(),
                actual: self.kind().into_expectation(),
            },
        )
    }

    /// Report an unsupported error for the current tree parser.
    pub(crate) fn expected(&mut self, expected: impl IntoExpectation) -> Error {
        Error::new(
            self.span(),
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
                self.peek_span(),
                ErrorKind::UnexpectedEndOfSyntaxWith {
                    inside: self.kind().into_expectation(),
                    expected: expected.into_expectation(),
                },
            ));
        };

        if node.value() != expected {
            return Err(Error::new(
                inner_span(node.span()),
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
                self.peek_span(),
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
                self.peek_span(),
                ErrorKind::UnexpectedEndOfSyntaxWith {
                    inside: self.kind().into_expectation(),
                    expected: T::into_expectation(),
                },
            ));
        };

        Node::new(node).ast()
    }

    /// Try to eat and return one node.
    pub(crate) fn eat(&mut self, expect: Kind) -> MaybeNode<'a> {
        self.eat_matching(|kind| kind == expect)
    }

    /// Try to eat and return one node.
    pub(crate) fn try_ast<T>(&mut self) -> Result<Option<T>>
    where
        T: ToAst,
    {
        match self.eat_matching(|kind| T::matches(&kind)) {
            MaybeNode::Some(node) => Ok(Some(node.ast()?)),
            MaybeNode::None => Ok(None),
        }
    }

    /// Try to eat and return one node.
    pub(crate) fn eat_matching<F>(&mut self, mut filter: F) -> MaybeNode<'a>
    where
        F: FnMut(Kind) -> bool,
    {
        if let Some(node) = self.next_node() {
            if filter(node.value()) {
                return MaybeNode::Some(Node::new(node));
            }

            self.peek = Some(node);
        }

        MaybeNode::None
    }

    /// Read remaining nodes equal to the given kind.
    pub(crate) fn remaining(
        &mut self,
        o: &mut dyn Ignore<'a>,
        expected: Kind,
    ) -> Result<Remaining<'a>> {
        let mut first = None;
        let mut trailing = None::<Span>;
        let mut out = MaybeNode::None;
        let mut count = 0;

        while let Some(node) = self.next_node() {
            if node.value() != expected {
                if let Some(node) = self.peek.replace(node) {
                    o.ignore(Node::new(node))?;
                }

                break;
            }

            let span = inner_span(node.span());

            if first.is_none() {
                first = Some(span);
            } else {
                trailing = Some(trailing.map(|head| head.join(span)).unwrap_or(span));
            }

            if let MaybeNode::Some(old) = out.replace(Node::new(node)) {
                o.ignore(old)?;
            }

            count += 1;
        }

        let span = match (first, &out) {
            (Some(first), MaybeNode::Some(last)) => first.join(last.span()),
            _ => self.peek_span(),
        };

        Ok(Remaining {
            inside: self.kind(),
            expected,
            span,
            trailing,
            node: out,
            count: Some(count),
        })
    }

    /// Read one node equal to the given kind.
    pub(crate) fn one(&mut self, expected: Kind) -> Remaining<'a> {
        let node = self.eat(expected);
        let span = node.span().unwrap_or_else(|| self.peek_span());
        let count = Some(usize::from(node.is_some()));

        Remaining {
            inside: self.kind(),
            expected,
            span,
            trailing: None,
            node,
            count,
        }
    }

    /// Require that the iterator is ended.
    pub(super) fn end(mut self) -> Result<()> {
        if let Some(node) = self.next_node() {
            let inside = self.kind();

            let span = match self.iter.next_back() {
                Some(last) => node.span().join(last.span()),
                None => *node.span(),
            };

            return Err(Error::new(
                inner_span(&span),
                ErrorKind::ExpectedSyntaxEnd {
                    inside: inside.into_expectation(),
                    actual: node.value().into_expectation(),
                },
            ));
        }

        Ok(())
    }

    /// Get the current span of the parser.
    pub(crate) fn remaining_span(&mut self) -> Option<Span> {
        let head = *self.peek_node()?.span();

        if let Some(last) = self.iter.peek_back() {
            Some(Span::new(head.start, last.span().end))
        } else {
            Some(Span::new(head.start, head.end))
        }
    }

    pub(crate) fn span(&self) -> Span {
        inner_span(self.node.span())
    }

    /// Get the next raw node, including whitespace.
    #[cfg(feature = "fmt")]
    pub(crate) fn next_with_ws(&mut self) -> Option<Node<'a>> {
        if let Some(node) = self.peek.take() {
            return Some(Node::new(node));
        }

        self.iter.next().map(Node::new)
    }

    fn next_node(&mut self) -> Option<InternalNode<'a>> {
        if let Some(node) = self.peek.take() {
            return Some(node);
        }

        // We walk over comments and whitespace separately when writing
        // nodes to ensure that formatting functions do not need to worry
        // about it here.
        self.iter.find(|node| !matches!(node.value(), ws!()))
    }

    fn next_back_node(&mut self) -> Option<InternalNode<'a>> {
        // We walk over comments and whitespace separately when writing
        // nodes to ensure that formatting functions do not need to worry
        // about it here.
        if let Some(node) = self.iter.rfind(|node| !matches!(node.value(), ws!())) {
            return Some(node);
        }

        self.peek.take()
    }
}

impl fmt::Debug for Stream<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stream").finish_non_exhaustive()
    }
}

impl Spanned for Stream<'_> {
    #[inline]
    fn span(&self) -> Span {
        Stream::span(self)
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

/// The identifier of a node.
#[derive(Debug)]
pub(crate) struct NodeId(syntree::pointer::PointerUsize);

/// A node associated with a tree.
#[derive(Debug, TryClone, Clone)]
#[try_clone(crate)]
pub(crate) struct NodeAt {
    tree: Rc<Tree>,
    #[try_clone(copy)]
    source_id: SourceId,
    #[try_clone(copy)]
    id: syntree::pointer::PointerUsize,
}

impl NodeAt {
    /// The tree associated with the node..
    pub(crate) fn tree(&self) -> &Rc<Tree> {
        &self.tree
    }

    /// Parse the node being referenced.
    pub(crate) fn parse<'a, P, O>(&'a self, parser: P) -> Result<O>
    where
        P: FnOnce(&mut Stream<'a>) -> Result<O>,
    {
        let Some(node) = self.tree.get(self.id) else {
            return Err(Error::msg(
                Span::empty(),
                try_format!("missing node {}", self.id.get()),
            ));
        };

        node.parse(parser)
    }

    /// Parse a custom id.
    pub(crate) fn parse_id<'a, P, O>(&'a self, NodeId(id): NodeId, parser: P) -> Result<O>
    where
        P: FnOnce(&mut Stream<'a>) -> Result<O>,
    {
        let Some(node) = self.tree.get(id) else {
            return Err(Error::msg(
                Span::empty(),
                try_format!("missing node {}", self.id.get()),
            ));
        };

        node.parse(parser)
    }

    /// Print the tree to the given writer.
    #[cfg(feature = "std")]
    pub(crate) fn print_with_sources(
        &self,
        title: impl fmt::Display,
        sources: &Sources,
    ) -> Result<()> {
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

        writeln!(o, "{title}:").with_span(Span::empty())?;

        for (depth, node) in node.inner.walk().with_depths() {
            if depth < 0 {
                break;
            }

            let n = (depth * 2) as usize;
            let data = node.value();
            let span = inner_span(node.span());

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
    inner: InternalNode<'a>,
}

impl<'a> Node<'a> {
    #[inline]
    pub(super) fn new(inner: InternalNode<'a>) -> Self {
        Self { inner }
    }

    /// Check if the current token is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Convert a node into a token.
    pub(crate) fn token(&self) -> Token {
        inner_token(self.inner)
    }

    /// Get the kind of the node.
    pub(crate) fn kind(&self) -> Kind {
        self.inner.value()
    }

    /// Walk the subtree.
    pub(crate) fn walk(&self) -> impl Iterator<Item = Node<'a>> {
        self.inner.walk().inside().map(Node::new)
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

    /// Iterate over child nodes.
    pub(crate) fn children(&self) -> impl DoubleEndedIterator<Item = Node<'a>> + '_ {
        self.inner.children().map(Node::new)
    }

    /// Iterate over child nodes using the internal iterator.
    pub(crate) fn internal_children(&self) -> InternalChildren<'a> {
        self.inner.children()
    }

    /// Get the children as a fixed array ignoring whitespace.
    pub(crate) fn fixed_vec<const N: usize, T>(
        &self,
        factory: fn(InternalNode<'a>) -> T,
    ) -> Option<FixedVec<T, N>> {
        let mut vec = FixedVec::new();

        for node in self
            .inner
            .children()
            .filter(|n| !matches!(n.value(), ws!()))
        {
            if vec.try_push(factory(node)).is_err() {
                return None;
            }
        }

        Some(vec)
    }

    /// Get the kinds of all children excluding whitespace.
    pub(crate) fn kinds<const N: usize>(&self) -> Option<[Kind; N]> {
        self.fixed_vec(|n| n.value())?.try_into_inner()
    }

    /// Get the tokens of all children excluding whitespace.
    pub(crate) fn tokens<const N: usize>(&self) -> Option<FixedVec<Token, N>> {
        self.fixed_vec(inner_token)
    }

    /// Get the children as an array ignoring whitespace.
    pub(crate) fn nodes<const N: usize>(&self) -> Option<[Node<'a>; N]> {
        self.fixed_vec(Node::new)?.try_into_inner()
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

    /// Ignore the node.
    #[cfg(feature = "fmt")]
    pub(crate) fn ignore(self, o: &mut Formatter<'a>) -> Result<()> {
        o.ignore(self)
    }

    /// Walk from the current node.
    #[cfg(feature = "fmt")]
    pub(crate) fn walk_from(&self) -> impl Iterator<Item = Node<'a>> + '_ {
        self.inner.walk_from().map(Node::new)
    }

    /// Run the given parser.
    #[inline]
    pub(crate) fn parse<P, O>(self, parser: P) -> Result<O>
    where
        P: FnOnce(&mut Stream<'a>) -> Result<O>,
    {
        self.into_stream().parse(parser)
    }

    /// Convert into a stream.
    pub(crate) fn into_stream(self) -> StreamBuf<'a> {
        StreamBuf {
            stream: Stream::new(self.inner),
        }
    }

    /// Construct a span for the current node.
    pub(crate) fn span(&self) -> Span {
        inner_span(self.inner.span())
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
    pub(crate) fn expected(&self, expected: impl IntoExpectation) -> Error {
        Error::new(
            self.span(),
            ErrorKind::ExpectedSyntax {
                expected: expected.into_expectation(),
                actual: self.kind().into_expectation(),
            },
        )
    }
}

#[inline]
pub(super) fn inner_token(node: InternalNode<'_>) -> Token {
    Token {
        span: inner_span(node.span()),
        kind: node.value(),
    }
}

#[inline]
fn inner_span(span: &syntree::Span<u32>) -> Span {
    Span::new(span.start, span.end)
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
        f.debug_struct("Node")
            .field("kind", &self.kind())
            .field("span", &self.span())
            .finish()
    }
}

#[must_use = "Remaining nodes must be consumed to capture all whitespace and comments"]
pub(crate) struct Remaining<'a> {
    inside: Kind,
    expected: Kind,
    span: Span,
    /// If there is more than one element, this contains the trailing span.
    trailing: Option<Span>,
    node: MaybeNode<'a>,
    count: Option<usize>,
}

impl<'a> Remaining<'a> {
    /// Get trailing span.
    pub(crate) fn trailing(&self) -> Option<Span> {
        self.trailing
    }

    /// Ensure that there is exactly one node represented by this container.
    ///
    /// This only fires if the remaining set has been constructed from a stream.
    pub(crate) fn exactly_one(self, o: &mut dyn Ignore<'a>) -> Result<()> {
        if let MaybeNode::Some(node) = self.node {
            o.ignore(node)?;
        }

        let Some(count) = self.count else {
            return Ok(());
        };

        if count == 0 {
            let result = o.error(Error::new(
                self.span,
                ErrorKind::ExpectedOne {
                    inside: self.inside.into_expectation(),
                    expected: self.expected.into_expectation(),
                },
            ));

            result.with_span(self.span)?;
        }

        if let Some(span) = self.trailing {
            let result = o.error(Error::new(
                span,
                ErrorKind::ExpectedAtMostOne {
                    inside: self.inside.into_expectation(),
                    expected: self.expected.into_expectation(),
                    count,
                },
            ));

            result.with_span(self.span)?;
        }

        Ok(())
    }

    /// Ensure that there are at most one node represented by this container.
    ///
    /// This only fires if the remaining set has been constructed from a stream.
    pub(crate) fn at_most_one(self, o: &mut dyn Ignore<'a>) -> Result<()> {
        if let MaybeNode::Some(node) = self.node {
            o.ignore(node)?;
        }

        let Some(count) = self.count else {
            return Ok(());
        };

        if let Some(span) = self.trailing {
            let result = o.error(Error::new(
                span,
                ErrorKind::ExpectedAtMostOne {
                    inside: self.inside.into_expectation(),
                    expected: self.expected.into_expectation(),
                    count,
                },
            ));

            result.with_span(self.span)?;
        }

        Ok(())
    }

    /// Ensure that there are at least one node represented by this container.
    ///
    /// This only fires if the remaining set has been constructed from a stream.
    pub(crate) fn at_least_one(self, o: &mut dyn Ignore<'a>) -> Result<()> {
        if let MaybeNode::Some(node) = self.node {
            o.ignore(node)?;
        }

        if matches!(self.count, Some(0)) {
            let result = o.error(Error::new(
                self.span,
                ErrorKind::ExpectedAtLeastOne {
                    inside: self.inside.into_expectation(),
                    expected: self.expected.into_expectation(),
                },
            ));

            result.with_span(self.span)?;
        }

        Ok(())
    }

    /// Test if there is a remaining node present.
    #[inline]
    pub(crate) fn is_present(&self) -> bool {
        self.node.is_some()
    }

    #[inline]
    pub(crate) fn is_absent(&self) -> bool {
        self.node.is_none()
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

        if let MaybeNode::Some(node) = self.node.take() {
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
        if let MaybeNode::Some(node) = self.node.take() {
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

        if let MaybeNode::Some(node) = self.node {
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

impl Default for Remaining<'_> {
    #[inline]
    fn default() -> Self {
        Self {
            inside: Kind::Root,
            expected: Kind::Eof,
            span: Span::empty(),
            trailing: None,
            node: MaybeNode::None,
            count: None,
        }
    }
}

#[derive(Default, Clone)]
struct Iter<'a> {
    first: Option<InternalNode<'a>>,
    last: Option<InternalNode<'a>>,
}

impl<'a> Iter<'a> {
    fn new(first: Option<InternalNode<'a>>, last: Option<InternalNode<'a>>) -> Self {
        Self { first, last }
    }

    /// Peek the next back node.
    fn peek_back(&self) -> Option<InternalNode<'a>> {
        self.last
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = InternalNode<'a>;

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

impl DoubleEndedIterator for Iter<'_> {
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

/// Maybe a node.
#[derive(Default)]
pub(crate) enum MaybeNode<'a> {
    Some(Node<'a>),
    #[default]
    None,
}

impl<'a> MaybeNode<'a> {
    /// Test if the node is set.
    pub(crate) fn is_some(&self) -> bool {
        matches!(self, MaybeNode::Some(..))
    }

    /// Test if the node is not set.
    pub(crate) fn is_none(&self) -> bool {
        matches!(self, MaybeNode::None)
    }

    /// Format the node if present.
    #[cfg(feature = "fmt")]
    pub(crate) fn fmt(self, o: &mut Formatter<'a>) -> Result<()> {
        match self {
            MaybeNode::Some(node) => node.fmt(o),
            MaybeNode::None => Ok(()),
        }
    }

    /// Map the result.
    #[cfg(feature = "fmt")]
    pub(crate) fn and_then<O>(self, f: impl FnOnce(Node<'a>) -> Result<O>) -> Result<Option<O>> {
        match self {
            MaybeNode::Some(node) => Ok(Some(f(node)?)),
            MaybeNode::None => Ok(None),
        }
    }

    /// Take the interior value of present.
    #[cfg(feature = "fmt")]
    pub(crate) fn take(&mut self) -> Self {
        take(self)
    }

    /// Replace the interior value if present.
    pub(crate) fn replace(&mut self, node: Node<'a>) -> Self {
        replace(self, MaybeNode::Some(node))
    }

    /// Get the span of the underlying node.
    pub(crate) fn span(&self) -> Option<Span> {
        match self {
            MaybeNode::Some(node) => Some(node.span()),
            MaybeNode::None => None,
        }
    }

    /// Helper to coerce a node into an ast element.
    pub(crate) fn ast<T>(self) -> Result<Option<T>>
    where
        T: ToAst,
    {
        match self {
            MaybeNode::Some(node) => node.ast().map(Some),
            MaybeNode::None => Ok(None),
        }
    }

    /// Parse the node being referenced.
    pub(crate) fn parse<P, O>(self, parser: P) -> Result<Option<O>>
    where
        P: FnOnce(&mut Stream<'a>) -> Result<O>,
    {
        match self {
            MaybeNode::Some(node) => node.parse(parser).map(Some),
            MaybeNode::None => Ok(None),
        }
    }
}
