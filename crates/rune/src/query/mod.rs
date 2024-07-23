//! Lazy query system, used to compile and build items on demand and keep track
//! of what's being used and not.

mod query;

use core::fmt;
use core::num::NonZeroUsize;

pub(crate) use self::query::{MissingId, Query, QueryInner};

use crate as rune;
use crate::alloc::path::PathBuf;
use crate::alloc::prelude::*;
use crate::alloc::{self, Box, Vec};
use crate::ast;
use crate::ast::{Span, Spanned};
use crate::compile::ir;
use crate::compile::{ItemId, ItemMeta, Location, ModId};
use crate::hash::Hash;
use crate::hir;
use crate::indexing;
use crate::parse::NonZeroId;
use crate::runtime::format;

/// Indication whether a value is being evaluated because it's being used or not.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
pub(crate) enum Used {
    /// The value is not being used.
    Unused,
    /// The value is being used.
    Used,
}

impl Used {
    /// Test if this used indicates unuse.
    pub(crate) fn is_unused(self) -> bool {
        matches!(self, Self::Unused)
    }
}

impl Default for Used {
    fn default() -> Self {
        Self::Used
    }
}

/// The result of calling [Query::convert_path].
pub(crate) struct Named<'ast> {
    /// Module named item belongs to.
    pub(crate) module: ModId,
    /// The path resolved to the given item.
    pub(crate) item: ItemId,
    /// Trailing parameters.
    pub(crate) trailing: usize,
    /// Type parameters if any.
    pub(crate) parameters: [Option<(
        &'ast dyn Spanned,
        &'ast ast::AngleBracketed<ast::PathSegmentExpr, T![,]>,
    )>; 2],
}

impl fmt::Display for Named<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.item, f)
    }
}

/// An internally resolved macro.
#[allow(clippy::large_enum_variant)]
pub(crate) enum BuiltInMacro {
    Template(BuiltInTemplate),
    Format(BuiltInFormat),
    File(BuiltInFile),
    Line(BuiltInLine),
}

/// An internally resolved template.
#[derive(Spanned)]
pub(crate) struct BuiltInTemplate {
    /// The span of the built-in template.
    #[rune(span)]
    pub(crate) span: Span,
    /// Indicate if template originated from literal.
    pub(crate) from_literal: bool,
    /// Expressions being concatenated as a template.
    pub(crate) exprs: Vec<ast::Expr>,
}

/// An internal format specification.
#[derive(Spanned)]
pub(crate) struct BuiltInFormat {
    #[rune(span)]
    pub(crate) span: Span,
    /// The fill character to use.
    pub(crate) fill: Option<char>,
    /// Alignment specification.
    pub(crate) align: Option<format::Alignment>,
    /// Width to fill.
    pub(crate) width: Option<NonZeroUsize>,
    /// Precision to fill.
    pub(crate) precision: Option<NonZeroUsize>,
    /// A specification of flags.
    pub(crate) flags: Option<format::Flags>,
    /// The format specification type.
    pub(crate) format_type: Option<format::Type>,
    /// The value being formatted.
    pub(crate) value: ast::Expr,
}

/// Macro data for `file!()`
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, Spanned)]
#[try_clone(copy)]
pub(crate) struct BuiltInFile {
    /// Path value to use
    pub(crate) value: ast::Lit,
}

/// Macro data for `line!()`
#[derive(Debug, TryClone, Clone, Copy, PartialEq, Eq, Spanned)]
#[try_clone(copy)]
pub(crate) struct BuiltInLine {
    /// The line number
    pub(crate) value: ast::Lit,
}

/// An entry in the build queue.
#[derive(Debug, TryClone)]
pub(crate) enum Build {
    Function(indexing::Function),
    Closure(indexing::Closure),
    AsyncBlock(indexing::AsyncBlock),
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

/// An implementation function.
pub(crate) struct QueryImplFn {
    /// Ast for declaration.
    pub(crate) ast: Box<ast::ItemFn>,
}

pub(crate) struct ItemImplEntry {
    /// Non-expanded ast of the path.
    pub(crate) path: Box<ast::Path>,
    /// Location where the item impl is defined and is being expanded.
    pub(crate) location: Location,
    /// The item impl being expanded.
    pub(crate) id: NonZeroId,
    ///See [Indexer][crate::indexing::Indexer].
    pub(crate) root: Option<PathBuf>,
    ///See [Indexer][crate::indexing::Indexer].
    pub(crate) nested_item: Option<Span>,
    /// See [Indexer][crate::indexing::Indexer].
    pub(crate) macro_depth: usize,
}

/// Query information for a path.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
pub(crate) struct QueryPath {
    pub(crate) module: ModId,
    pub(crate) impl_item: Option<NonZeroId>,
    pub(crate) item: ItemId,
}

/// A compiled constant function.
pub(crate) struct ConstFn<'hir> {
    /// The item of the const fn.
    pub(crate) item_meta: ItemMeta,
    /// The soon-to-be deprecated IR function.
    pub(crate) ir_fn: ir::IrFn,
    /// HIR function associated with this constant function.
    #[allow(unused)]
    pub(crate) hir: hir::ItemFn<'hir>,
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

    pub(crate) fn as_boxed(&self) -> alloc::Result<Box<[Option<Hash>]>> {
        self.parameters.iter().copied().try_collect()
    }
}

impl AsRef<GenericsParameters> for GenericsParameters {
    #[inline]
    fn as_ref(&self) -> &GenericsParameters {
        self
    }
}
