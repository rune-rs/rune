pub(crate) mod index;
pub(crate) mod items;
mod scopes;

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::Box;
use crate::ast::{self, Span, Spanned};
use crate::compile::meta;
use crate::compile::{ItemId, ItemMeta};
use crate::parse::NonZeroId;
use crate::runtime::Call;

pub(crate) use self::index::{IndexItem, Indexer};
pub(crate) use self::items::Items;
pub(crate) use self::scopes::{Layer, Scopes};

#[derive(Debug, TryClone)]
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
#[derive(Debug, TryClone)]
pub(crate) enum Indexed {
    /// An enum.
    Enum,
    /// A struct.
    Struct(Struct),
    /// A variant.
    Variant(Variant),
    /// A function.
    Function(Function),
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

/// The ast of a function.
#[derive(Debug, TryClone, Spanned)]
pub(crate) enum FunctionAst {
    /// An empty function body.
    Empty(Box<ast::EmptyBlock>, #[rune(span)] Span),
    /// A regular item function body.
    Item(Box<ast::ItemFn>),
}

impl FunctionAst {
    /// Get the number of arguments for the function ast.
    #[cfg(feature = "doc")]
    pub(crate) fn args(&self) -> usize {
        match self {
            FunctionAst::Empty(..) => 0,
            FunctionAst::Item(ast) => ast.args.len(),
        }
    }
}

#[derive(Debug, TryClone)]
pub(crate) struct Function {
    /// Ast for declaration.
    pub(crate) ast: FunctionAst,
    /// The calling convention of the function.
    pub(crate) call: Call,
    /// If this is an instance function that receives `self`.
    pub(crate) is_instance: bool,
    /// If this is a test function.
    pub(crate) is_test: bool,
    /// If this is a bench function.
    pub(crate) is_bench: bool,
    /// The impl item this function is registered in.
    #[allow(unused)]
    pub(crate) impl_item: Option<NonZeroId>,
}

#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
pub(crate) struct Import {
    /// The import entry.
    pub(crate) entry: meta::Import,
    /// Indicates if the import is a wildcard or not.
    ///
    /// Wildcard imports do not cause unused warnings.
    pub(crate) wildcard: bool,
}

#[derive(Debug, TryClone)]
pub(crate) struct Struct {
    /// The ast of the struct.
    pub(crate) ast: Box<ast::ItemStruct>,
}

#[derive(Debug, TryClone)]
pub(crate) struct Variant {
    /// Id of of the enum type.
    pub(crate) enum_id: NonZeroId,
    /// Ast for declaration.
    pub(crate) ast: ast::ItemVariant,
    /// The index of the variant in its source.
    pub(crate) index: usize,
}

#[derive(Debug, TryClone)]
pub(crate) struct Closure {
    /// Ast for closure.
    pub(crate) ast: Box<ast::ExprClosure>,
    /// Calling convention used for closure.
    pub(crate) call: Call,
}

#[derive(Debug, TryClone)]
pub(crate) struct AsyncBlock {
    /// Ast for block.
    pub(crate) ast: ast::Block,
    /// Calling convention used for async block.
    pub(crate) call: Call,
}

#[derive(Debug, TryClone)]
pub(crate) struct ConstExpr {
    pub(crate) ast: Box<ast::Expr>,
}

#[derive(Debug, TryClone)]
pub(crate) struct ConstBlock {
    pub(crate) ast: Box<ast::Block>,
}

#[derive(Debug, TryClone)]
pub(crate) struct ConstFn {
    /// The const fn ast.
    pub(crate) item_fn: Box<ast::ItemFn>,
}
