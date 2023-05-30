//! Lazy query system, used to compile and build items on demand and keep track
//! of what's being used and not.

mod query;

use core::fmt;
use core::num::NonZeroUsize;

use crate::no_std::prelude::*;

pub(crate) use self::query::Query;
use crate as rune;
use crate::ast;
use crate::ast::{Span, Spanned};
use crate::compile::ir;
use crate::compile::{self, CompileErrorKind, ItemId, ItemMeta, ModId};
use crate::hir;
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
#[derive(Debug)]
pub(crate) struct Named<'hir> {
    /// If the resolved value is local.
    pub(crate) local: Option<Box<str>>,
    /// The path resolved to the given item.
    pub(crate) item: ItemId,
    /// Trailing parameters.
    pub(crate) trailing: usize,
    /// Type parameters if any.
    pub(crate) parameters: [Option<(Span, &'hir [hir::Expr<'hir>])>; 2],
}

impl Named<'_> {
    /// Get the local identifier of this named.
    pub(crate) fn as_local(&self) -> Option<&str> {
        if self.parameters.iter().all(|v| v.is_none()) {
            self.local.as_deref()
        } else {
            None
        }
    }

    /// Assert that this named type is not generic.
    pub(crate) fn assert_not_generic(&self) -> compile::Result<()> {
        if let Some((span, _)) = self.parameters.iter().flatten().next() {
            return Err(compile::Error::new(
                span,
                CompileErrorKind::UnsupportedGenerics,
            ));
        }

        Ok(())
    }
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
    pub(crate) fill: Option<(ast::LitChar, char)>,
    /// Alignment specification.
    pub(crate) align: Option<(ast::Ident, format::Alignment)>,
    /// Width to fill.
    pub(crate) width: Option<(ast::LitNumber, Option<NonZeroUsize>)>,
    /// Precision to fill.
    pub(crate) precision: Option<(ast::LitNumber, Option<NonZeroUsize>)>,
    /// A specification of flags.
    pub(crate) flags: Option<(ast::LitNumber, format::Flags)>,
    /// The format specification type.
    pub(crate) format_type: Option<(ast::Ident, format::Type)>,
    /// The value being formatted.
    pub(crate) value: ast::Expr,
}

/// Macro data for `file!()`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Spanned)]
pub(crate) struct BuiltInFile {
    /// The span of the built-in-file
    #[rune(span)]
    pub(crate) span: Span,
    /// Path value to use
    pub(crate) value: ast::LitStr,
}

/// Macro data for `line!()`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Spanned)]
pub(crate) struct BuiltInLine {
    /// The span of the built-in-file
    #[rune(span)]
    pub(crate) span: Span,
    /// The line number
    pub(crate) value: ast::LitNumber,
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
    module: ModId,
    impl_item: Option<ItemId>,
    item: ItemId,
}

/// A compiled constant function.
#[derive(Debug)]
pub(crate) struct ConstFn {
    /// The item of the const fn.
    pub(crate) item_meta: ItemMeta,
    /// The compiled constant function.
    pub(crate) ir_fn: ir::IrFn,
}
