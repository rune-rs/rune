pub(crate) mod index;
mod locals;
mod scopes;

use crate::no_std::prelude::*;

use crate::ast::{self, Span};
use crate::compile::meta;
use crate::compile::{ItemId, ItemMeta};
use crate::hash::Hash;
use crate::parse::Id;
use crate::runtime::Call;

pub(crate) use self::index::Indexer;
pub(crate) use self::scopes::{Layer, Scopes};

#[derive(Debug, Clone)]
pub(crate) struct Entry {
    /// The query item this indexed entry belongs to.
    pub(crate) item_meta: ItemMeta,
    /// The entry data.
    pub(crate) indexed: Indexed,
}

impl Entry {
    /// The item that best describes this indexed entry.
    pub(crate) fn item(&self) -> ItemId {
        match &self.indexed {
            Indexed::Import(Import { entry, .. }) => entry.target,
            _ => self.item_meta.item,
        }
    }
}

/// An entry that has been indexed.
#[derive(Debug, Clone)]
pub(crate) enum Indexed {
    /// An enum.
    Enum,
    /// A struct.
    Struct(Struct),
    /// A variant.
    Variant(Variant),
    /// A function.
    Function(Function),
    /// An instance function.
    InstanceFunction(InstanceFunction),
    /// A constant expression.
    ConstExpr(ConstExpr),
    /// A constant block.
    ConstBlock(ConstBlock),
    /// A constant function.
    ConstFn(ConstFn),
    /// An import.
    Import(Import),
    /// An indexed module.
    Module,
}

#[derive(Debug, Clone)]
pub(crate) struct Function {
    /// Ast for declaration.
    pub(crate) ast: Box<ast::ItemFn>,
    /// The calling convention of the function.
    pub(crate) call: Call,
    /// If this is a test function.
    pub(crate) is_test: bool,
    /// If this is a bench function.
    pub(crate) is_bench: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct InstanceFunction {
    /// Ast for declaration.
    pub(crate) ast: Box<ast::ItemFn>,
    /// The calling convention of the function.
    pub(crate) call: Call,
    /// The item of the instance function.
    pub(crate) impl_item: ItemId,
    /// The span of the instance function.
    pub(crate) instance_span: Span,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Import {
    /// The import entry.
    pub(crate) entry: meta::Import,
    /// Indicates if the import is a wildcard or not.
    ///
    /// Wildcard imports do not cause unused warnings.
    pub(crate) wildcard: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct Struct {
    /// The ast of the struct.
    pub(crate) ast: Box<ast::ItemStruct>,
}

#[derive(Debug, Clone)]
pub(crate) struct Variant {
    /// Id of of the enum type.
    pub(crate) enum_id: Id,
    /// Ast for declaration.
    pub(crate) ast: ast::ItemVariant,
    /// The index of the variant in its source.
    pub(crate) index: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct Closure {
    /// Ast for closure.
    pub(crate) ast: Box<ast::ExprClosure>,
    /// Calling convention used for closure.
    pub(crate) call: Call,
    /// Captures.
    pub(crate) captures: Hash,
}

#[derive(Debug, Clone)]
pub(crate) struct AsyncBlock {
    /// Ast for block.
    pub(crate) ast: ast::Block,
    /// Calling convention used for async block.
    pub(crate) call: Call,
    /// Captured variables.
    pub(crate) captures: Hash,
}

#[derive(Debug, Clone)]
pub(crate) struct ConstExpr {
    pub(crate) ast: Box<ast::Expr>,
}

#[derive(Debug, Clone)]
pub(crate) struct ConstBlock {
    pub(crate) ast: Box<ast::Block>,
}

#[derive(Debug, Clone)]
pub(crate) struct ConstFn {
    /// The const fn ast.
    pub(crate) item_fn: Box<ast::ItemFn>,
}
