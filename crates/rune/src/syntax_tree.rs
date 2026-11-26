//! The syntax tree the compiler parses a source into, flattened into the
//! order it is walked in, for tools which want to show it.
//!
//! This is not a stable interface. It exposes the parser's own node kinds,
//! which change whenever the grammar does.

use crate::alloc::Vec;
use crate::ast::{Kind, Span};
use crate::compile::{self, Options};
use crate::SourceId;

/// A node or token in a [`SyntaxTree`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Node {
    /// How deeply the node is nested, where the root is zero.
    pub depth: usize,
    /// What the node is.
    pub kind: Kind,
    /// The byte span of source the node covers.
    pub span: Span,
    /// Whether this is a token rather than a node with children.
    pub is_token: bool,
}

/// A parsed syntax tree.
#[derive(Debug)]
#[non_exhaustive]
pub struct SyntaxTree {
    /// Every node in the tree, parents before their children.
    pub nodes: Vec<Node>,
    /// The error parsing stopped at, if any, in which case `nodes` is the
    /// tree as far as it got.
    pub error: Option<compile::Error>,
}

/// Parse `source` the way the compiler does and flatten the result.
///
/// A source which does not parse still produces the tree up to where it
/// stopped, alongside the error.
///
/// # Examples
///
/// ```
/// let tree = rune::syntax_tree::parse("fn main() {}")?;
/// assert!(tree.error.is_none());
/// assert_eq!(tree.nodes[0].depth, 0);
///
/// let tree = rune::syntax_tree::parse("fn main() { a < b < c }")?;
/// assert!(tree.error.is_some());
/// assert!(!tree.nodes.is_empty());
/// # Ok::<_, rune::compile::Error>(())
/// ```
pub fn parse(source: &str) -> compile::Result<SyntaxTree> {
    let (tree, error) = crate::grammar::text(SourceId::EMPTY, source)
        .max_nesting(Options::DEFAULT.max_depth)
        .root_partial()?;

    let mut nodes = Vec::new();

    for (depth, kind, span, is_token) in tree.walk_with_depths() {
        nodes.try_push(Node {
            depth,
            kind,
            span,
            is_token,
        })?;
    }

    Ok(SyntaxTree { nodes, error })
}
