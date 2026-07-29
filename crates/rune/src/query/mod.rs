//! Lazy query system, used to compile and build items on demand and keep track
//! of what's being used and not.

mod query;

use core::fmt;
use core::mem::take;

use rust_alloc::rc::Rc;

pub(crate) use self::query::{Query, QueryInner, QuerySource};

use crate::alloc::prelude::*;
use crate::ast::{self, OptionSpanned, Span};
use crate::compile::{Doc, Error, ItemId, ItemMeta, Location, ModId, Result};
use crate::grammar::{Ignore, Node, NodeAt, NodeId, Tree};
use crate::hash::Hash;
use crate::hir;
use crate::indexing;
use crate::parse::NonZeroId;
use crate::runtime::Call;
use crate::{self as rune, SourceId};

/// Indication whether a value is being evaluated because it's being used or not.
#[derive(Default, Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
pub(crate) enum Used {
    /// The value is not being used.
    Unused,
    /// The value is being used.
    #[default]
    Used,
}

impl Used {
    /// Test if this used indicates unuse.
    pub(crate) fn is_unused(self) -> bool {
        matches!(self, Self::Unused)
    }
}

pub(crate) enum Named2Kind {
    /// A full path.
    Full,
    /// An identifier.
    Ident(ast::Ident),
    /// Self value.
    SelfValue(#[allow(unused)] ast::SelfValue),
}

/// The result of calling [Query::convert_path2].
pub(crate) struct Named2<'a> {
    /// Module named item belongs to.
    pub(crate) module: ModId,
    /// The kind of named item.
    pub(crate) kind: Named2Kind,
    /// The path resolved to the given item.
    pub(crate) item: ItemId,
    /// Trailing parameters.
    pub(crate) trailing: usize,
    /// Type parameters if any.
    pub(crate) parameters: [Option<Node<'a>>; 2],
}

impl fmt::Display for Named2<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.item, f)
    }
}

pub(crate) enum BuiltInMacro2 {
    File(ast::LitStr),
    Line(usize),
    Template(Rc<Tree>, BuiltInLiteral),
    Format(Rc<Tree>),
}

#[derive(Debug, TryClone)]
pub(crate) struct Closure<'hir> {
    /// Ast for closure.
    pub(crate) hir: &'hir hir::ExprClosure<'hir>,
    /// Calling convention used for closure.
    pub(crate) call: Call,
}

#[derive(Debug, TryClone)]
pub(crate) struct AsyncBlock<'hir> {
    /// Ast for block.
    pub(crate) hir: &'hir hir::AsyncBlock<'hir>,
    /// Calling convention used for async block.
    pub(crate) call: Call,
}

/// An entry in the build queue.
#[derive(Debug, TryClone)]
pub(crate) enum SecondaryBuild<'hir> {
    Closure(Closure<'hir>),
    AsyncBlock(AsyncBlock<'hir>),
}

/// An entry in the build queue.
#[derive(Debug, TryClone)]
pub(crate) struct SecondaryBuildEntry<'hir> {
    /// The item of the build entry.
    pub(crate) item_meta: ItemMeta,
    /// The build entry.
    pub(crate) build: SecondaryBuild<'hir>,
}

/// An entry in the build queue.
#[derive(Debug, TryClone)]
pub(crate) enum Build {
    Function(indexing::Function),
    Unused,
    Import(indexing::Import),
    /// A public re-export.
    ReExport,
    /// A build which simply queries for the item.
    Query,
}

/// An entry in the build queue.
#[derive(Debug, TryClone)]
pub(crate) struct BuildEntry {
    /// The item of the build entry.
    pub(crate) item_meta: ItemMeta,
    /// The build entry.
    pub(crate) build: Build,
}

/// The kind of item being implemented.
pub(crate) enum ImplItemKind {
    Node {
        /// The path being implemented.
        path: NodeAt,
        /// Functions being added.
        functions: Vec<(NodeId, Attrs)>,
    },
}

#[must_use = "must be consumed"]
#[derive(Default, Debug)]
pub(crate) struct Attrs {
    pub(crate) test: Option<Span>,
    pub(crate) bench: Option<Span>,
    pub(crate) docs: Vec<Doc>,
    pub(crate) builtin: Option<(Span, BuiltInLiteral)>,
}

impl Attrs {
    pub(crate) fn deny_non_docs(self, cx: &mut dyn Ignore<'_>) -> Result<()> {
        if let Some(span) = self.test {
            cx.error(Error::msg(span, "unsupported #[test] attribute"))?;
        }

        if let Some(span) = self.bench {
            cx.error(Error::msg(span, "unsupported #[bench] attribute"))?;
        }

        if let Some((span, _)) = self.builtin {
            cx.error(Error::msg(span, "unsupported #[builtin] attribute"))?;
        }

        Ok(())
    }

    pub(crate) fn deny_any(self, cx: &mut dyn Ignore<'_>) -> Result<()> {
        if let Some(span) = self.docs.option_span() {
            cx.error(Error::msg(span, "unsupported documentation"))?;
        }

        self.deny_non_docs(cx)?;
        Ok(())
    }
}

/// The implementation item.
pub(crate) struct ImplItem {
    /// The kind of item being implemented.
    pub(crate) kind: ImplItemKind,
    /// Location where the item impl is defined and is being expanded.
    pub(crate) location: Location,
    ///See [Indexer][crate::indexing::Indexer].
    pub(crate) root: Option<SourceId>,
    ///See [Indexer][crate::indexing::Indexer].
    pub(crate) nested_item: Option<Span>,
    /// See [Indexer][crate::indexing::Indexer].
    pub(crate) macro_depth: usize,
}

/// Expand the given macro.
#[must_use = "Must be used to report errors"]
pub(crate) struct ExpandMacroBuiltin {
    /// The identifier of the macro being expanded.
    pub(crate) id: NonZeroId,
    /// The macro being expanded.
    pub(crate) node: NodeAt,
    /// Location where the item impl is defined and is being expanded.
    pub(crate) location: Location,
    /// See [Indexer][crate::indexing::Indexer].
    pub(crate) root: Option<SourceId>,
    /// See [Indexer][crate::indexing::Indexer].
    pub(crate) macro_depth: usize,
    /// Indexing item at macro expansion position.
    pub(crate) item: indexing::IndexItem,
    /// The literal option.
    pub(crate) literal: BuiltInLiteral,
}

impl ExpandMacroBuiltin {
    /// Deny any unused options.
    pub(crate) fn finish(self) -> Result<NonZeroId> {
        if let BuiltInLiteral::Yes(span) = self.literal {
            return Err(Error::msg(
                span,
                "#[builtin(literal)] option is not allowed",
            ));
        }

        Ok(self.id)
    }
}

/// Whether the literal option is set.
#[derive(Default, Debug)]
pub(crate) enum BuiltInLiteral {
    Yes(Span),
    #[default]
    No,
}

impl BuiltInLiteral {
    /// Take the literal option.
    pub(crate) fn take(&mut self) -> Self {
        take(self)
    }

    /// Test if the literal option is set.
    pub(crate) fn is_yes(&self) -> bool {
        matches!(self, Self::Yes(_))
    }
}

/// Expand an item which has at least one attribute which might be an attribute
/// macro.
pub(crate) struct ExpandAttributeMacro {
    /// The item node being expanded, including its attributes.
    pub(crate) node: NodeAt,
    /// Location of the item being expanded.
    pub(crate) location: Location,
    /// See [Indexer][crate::indexing::Indexer].
    pub(crate) root: Option<SourceId>,
    /// See [Indexer][crate::indexing::Indexer].
    pub(crate) nested_item: Option<Span>,
    /// See [Indexer][crate::indexing::Indexer].
    pub(crate) macro_depth: usize,
    /// Indexing item at macro expansion position.
    pub(crate) item: indexing::IndexItem,
}

/// A deferred build entry.
pub(crate) enum DeferEntry {
    ImplItem(ImplItem),
    ExpandMacroBuiltin(ExpandMacroBuiltin),
    ExpandMacroCall(ExpandMacroBuiltin),
    ExpandAttributeMacro(ExpandAttributeMacro),
}

/// A compiled constant function.
pub(crate) struct ConstFn<'hir> {
    /// The item of the const fn.
    pub(crate) item_meta: ItemMeta,
    /// HIR function associated with this constant function.
    pub(crate) hir: hir::ItemFn<'hir>,
    /// Expressions referred to by `hir`.
    ///
    /// The HIR refers to its children by identifier, so it is only meaningful
    /// alongside the store they were lowered into.
    pub(crate) exprs: hir::Exprs<'hir>,
}

/// The data of a macro call.
pub(crate) enum ExpandedMacro {
    /// A built-in expanded macro.
    Builtin(BuiltInMacro2),
    /// The expanded body of a macro.
    Tree(Rc<Tree>),
}

/// Generic parameters.
#[derive(Default)]
pub(crate) struct GenericsParameters {
    pub(crate) trailing: usize,
    pub(crate) parameters: [Option<Hash>; 2],
}

impl GenericsParameters {
    pub(crate) fn is_empty(&self) -> bool {
        self.parameters.iter().all(|p| p.is_none())
    }
}

impl fmt::Debug for GenericsParameters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_list();

        for p in &self.parameters[2 - self.trailing..] {
            f.entry(p);
        }

        f.finish()
    }
}

impl AsRef<GenericsParameters> for GenericsParameters {
    #[inline]
    fn as_ref(&self) -> &GenericsParameters {
        self
    }
}
