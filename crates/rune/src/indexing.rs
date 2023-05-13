pub(crate) mod index;
mod index_scopes;
mod locals;

use crate::no_std::prelude::*;
use crate::no_std::sync::Arc;

use crate::ast::{self, Span};
use crate::compile::ir;
use crate::compile::meta;
use crate::compile::{ItemId, ItemMeta, Location, ModId};
use crate::parse::Id;
use crate::runtime::Call;

pub(crate) use self::index::Indexer;
pub(crate) use self::index_scopes::{IndexFnKind, IndexScopes};

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
    /// A closure.
    Closure(Closure),
    /// An async block.
    AsyncBlock(AsyncBlock),
    /// A constant value.
    Const(Const),
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
    /// Captures.
    pub(crate) captures: Arc<[String]>,
    /// Calling convention used for closure.
    pub(crate) call: Call,
    /// If the closure moves its captures.
    pub(crate) do_move: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct AsyncBlock {
    /// Ast for block.
    pub(crate) ast: ast::Block,
    /// Captures.
    pub(crate) captures: Arc<[String]>,
    /// Calling convention used for async block.
    pub(crate) call: Call,
    /// If the block moves its captures.
    pub(crate) do_move: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct Const {
    /// The module item the constant is defined in.
    pub(crate) module: ModId,
    /// The intermediate representation of the constant expression.
    pub(crate) ir: ir::Ir,
}

#[derive(Debug, Clone)]
pub(crate) struct ConstFn {
    /// The source of the constant function.
    pub(crate) location: Location,
    /// The const fn ast.
    pub(crate) item_fn: Box<ast::ItemFn>,
}
