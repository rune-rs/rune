//! Lazy query system, used to compile and build items on demand and keep track
//! of what's being used and not.

mod query;

use core::fmt;
use core::num::NonZeroUsize;

use crate::no_std::prelude::*;

pub(crate) use self::query::{MissingId, Query, QueryInner};

use crate as rune;
use crate::ast;
use crate::ast::{Span, Spanned};
use crate::compile::ir;
use crate::compile::{ItemId, ItemMeta, ModId};
use crate::indexing;
use crate::runtime::format;

/// Indication whether a value is being evaluated because it's being used or not.
#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Spanned)]
pub(crate) struct BuiltInFile {
    /// Path value to use
    pub(crate) value: ast::Lit,
}

/// Macro data for `line!()`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Spanned)]
pub(crate) struct BuiltInLine {
    /// The line number
    pub(crate) value: ast::Lit,
}

/// An entry in the build queue.
#[derive(Debug, Clone)]
pub(crate) enum Build {
    Function(indexing::Function),
    InstanceFunction(indexing::InstanceFunction),
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
#[derive(Debug, Clone)]
pub(crate) struct BuildEntry {
    /// The item of the build entry.
    pub(crate) item_meta: ItemMeta,
    /// If the queued up entry was unused or not.
    pub(crate) used: Used,
    /// The build entry.
    pub(crate) build: Build,
}

/// Query information for a path.
#[derive(Debug, Clone, Copy)]
pub(crate) struct QueryPath {
    pub(crate) module: ModId,
    pub(crate) item: ItemId,
    pub(crate) impl_item: Option<ItemId>,
}

/// A compiled constant function.
#[derive(Debug)]
pub(crate) struct ConstFn {
    /// The item of the const fn.
    pub(crate) item_meta: ItemMeta,
    /// The compiled constant function.
    pub(crate) ir_fn: ir::IrFn,
}
