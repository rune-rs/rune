//! Lazy query system, used to compile and build items on demand.

use crate::ast;
use crate::collections::{HashMap, HashSet};
use crate::error::CompileError;
use crate::source::Source;
use crate::traits::Resolve as _;
use runestick::{
    Call, Hash, Item, Meta, MetaClosureCapture, MetaStruct, MetaTuple, Span, Unit, ValueType,
};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Arc;

pub(crate) enum Indexed {
    Enum,
    Struct(Struct),
    Variant(Variant),
    Function(Function),
    Closure(Closure),
    AsyncBlock(AsyncBlock),
}

pub struct Struct {
    ast: ast::DeclStruct,
}

impl Struct {
    /// Construct a new struct entry.
    pub fn new(ast: ast::DeclStruct) -> Self {
        Self { ast }
    }
}

pub struct Variant {
    /// Item of the enum type.
    enum_item: Item,
    /// Ast for declaration.
    ast: ast::DeclStructBody,
}

impl Variant {
    /// Construct a new variant.
    pub fn new(enum_item: Item, ast: ast::DeclStructBody) -> Self {
        Self { enum_item, ast }
    }
}

pub(crate) struct Function {
    /// Ast for declaration.
    pub(crate) ast: ast::DeclFn,
    pub(crate) call: Call,
}

pub(crate) struct InstanceFunction {
    /// Ast for the instance function.
    pub(crate) ast: ast::DeclFn,
    /// The item of the instance function.
    pub(crate) impl_item: Item,
    /// The span of the instance function.
    pub(crate) instance_span: Span,
    pub(crate) call: Call,
}

pub(crate) struct Closure {
    /// Ast for closure.
    pub(crate) ast: ast::ExprClosure,
    /// Captures.
    pub(crate) captures: Arc<Vec<MetaClosureCapture>>,
    /// Calling convention used for closure.
    pub(crate) call: Call,
}

pub(crate) struct AsyncBlock {
    /// Ast for block.
    pub(crate) ast: ast::ExprBlock,
    /// Captures.
    pub(crate) captures: Arc<Vec<MetaClosureCapture>>,
    /// Calling convention used for async block.
    pub(crate) call: Call,
}

/// An entry in the build queue.
pub(crate) enum Build {
    Function(Function),
    InstanceFunction(InstanceFunction),
    Closure(Closure),
    AsyncBlock(AsyncBlock),
}

pub(crate) struct Query<'a> {
    pub(crate) source: Source<'a>,
    pub(crate) queue: VecDeque<(Item, Build)>,
    indexed: HashMap<Item, Indexed>,
    pub(crate) unit: Rc<RefCell<Unit>>,
}

impl<'a> Query<'a> {
    /// Construct a new compilation context.
    pub fn new(source: Source<'a>, unit: Rc<RefCell<Unit>>) -> Self {
        Self {
            source,
            queue: VecDeque::new(),
            indexed: HashMap::new(),
            unit,
        }
    }

    /// Add a new enum item.
    pub fn index_enum(&mut self, item: Item, span: Span) -> Result<(), CompileError> {
        log::trace!("new enum: {}", item);
        self.index(item, Indexed::Enum, span)?;
        Ok(())
    }

    /// Add a new struct item that can be queried.
    pub fn index_struct(&mut self, item: Item, ast: ast::DeclStruct) -> Result<(), CompileError> {
        log::trace!("new struct: {}", item);
        let span = ast.span();
        self.index(item, Indexed::Struct(Struct::new(ast)), span)?;
        Ok(())
    }

    /// Add a new variant item that can be queried.
    pub fn index_variant(
        &mut self,
        item: Item,
        enum_item: Item,
        ast: ast::DeclStructBody,
        span: Span,
    ) -> Result<(), CompileError> {
        log::trace!("new variant: {}", item);
        self.index(item, Indexed::Variant(Variant::new(enum_item, ast)), span)?;
        Ok(())
    }

    /// Add a new function that can be queried for.
    pub fn index_closure(
        &mut self,
        item: Item,
        ast: ast::ExprClosure,
        captures: Arc<Vec<MetaClosureCapture>>,
        call: Call,
    ) -> Result<(), CompileError> {
        let span = ast.span();
        log::trace!("new closure: {}", item);

        self.index(
            item,
            Indexed::Closure(Closure {
                ast,
                captures,
                call,
            }),
            span,
        )?;

        Ok(())
    }

    /// Add a new async block.
    pub fn index_async_block(
        &mut self,
        item: Item,
        ast: ast::ExprBlock,
        captures: Arc<Vec<MetaClosureCapture>>,
        call: Call,
    ) -> Result<(), CompileError> {
        let span = ast.span();
        log::trace!("new closure: {}", item);

        self.index(
            item,
            Indexed::AsyncBlock(AsyncBlock {
                ast,
                captures,
                call,
            }),
            span,
        )?;

        Ok(())
    }

    /// Index the given element.
    pub fn index(&mut self, item: Item, indexed: Indexed, span: Span) -> Result<(), CompileError> {
        log::trace!("indexed: {}", item);

        if let Some(..) = self.indexed.insert(item.clone(), indexed) {
            return Err(CompileError::ItemConflict {
                existing: item,
                span,
            });
        }

        Ok(())
    }

    /// Query for the given meta item.
    pub fn query_meta(&mut self, item: &Item, span: Span) -> Result<Option<Meta>, CompileError> {
        let item = Item::of(item);

        if let Some(meta) = self.unit.borrow().lookup_meta(&item) {
            return Ok(Some(meta));
        }

        // See if there's an index entry we can construct.
        let entry = match self.indexed.remove(&item) {
            Some(entry) => entry,
            None => return Ok(None),
        };

        let meta = match entry {
            Indexed::Enum => Meta::MetaEnum {
                value_type: ValueType::Type(Hash::type_hash(&item)),
                item: item.clone(),
            },
            Indexed::Variant(variant) => {
                // Assert that everything is built for the enum.
                self.query_meta(&variant.enum_item, span)?;
                self.ast_into_item_decl(&item, variant.ast, Some(variant.enum_item))?
            }
            Indexed::Struct(st) => self.ast_into_item_decl(&item, st.ast.body, None)?,
            Indexed::Function(f) => {
                self.queue.push_back((item.clone(), Build::Function(f)));

                Meta::MetaFunction {
                    value_type: ValueType::Type(Hash::type_hash(&item)),
                    item: item.clone(),
                }
            }
            Indexed::Closure(c) => {
                let captures = c.captures.clone();
                self.queue.push_back((item.clone(), Build::Closure(c)));

                Meta::MetaClosure {
                    value_type: ValueType::Type(Hash::type_hash(&item)),
                    item: item.clone(),
                    captures,
                }
            }
            Indexed::AsyncBlock(async_block) => {
                let captures = async_block.captures.clone();
                self.queue
                    .push_back((item.clone(), Build::AsyncBlock(async_block)));

                Meta::MetaAsyncBlock {
                    value_type: ValueType::Type(Hash::type_hash(&item)),
                    item: item.clone(),
                    captures,
                }
            }
        };

        self.unit.borrow_mut().insert_meta(meta)?;

        match self.unit.borrow().lookup_meta(&item) {
            Some(meta) => Ok(Some(meta)),
            None => Err(CompileError::MissingType { span, item }),
        }
    }

    /// Convert an ast declaration into a struct.
    fn ast_into_item_decl(
        &self,
        item: &Item,
        body: ast::DeclStructBody,
        enum_item: Option<Item>,
    ) -> Result<Meta, CompileError> {
        let value_type = ValueType::Type(Hash::type_hash(item));

        Ok(match body {
            ast::DeclStructBody::EmptyBody(..) => {
                let tuple = MetaTuple {
                    item: item.clone(),
                    args: 0,
                    hash: Hash::type_hash(item),
                };

                match enum_item {
                    Some(enum_item) => Meta::MetaVariantTuple {
                        value_type,
                        enum_item,
                        tuple,
                    },
                    None => Meta::MetaTuple { value_type, tuple },
                }
            }
            ast::DeclStructBody::TupleBody(tuple) => {
                let tuple = MetaTuple {
                    item: item.clone(),
                    args: tuple.fields.len(),
                    hash: Hash::type_hash(item),
                };

                match enum_item {
                    Some(enum_item) => Meta::MetaVariantTuple {
                        value_type,
                        enum_item,
                        tuple,
                    },
                    None => Meta::MetaTuple { value_type, tuple },
                }
            }
            ast::DeclStructBody::StructBody(st) => {
                let mut fields = HashSet::new();

                for (ident, _) in &st.fields {
                    let ident = ident.resolve(self.source)?;
                    fields.insert(ident.to_owned());
                }

                let object = MetaStruct {
                    item: item.clone(),
                    fields: Some(fields),
                };

                match enum_item {
                    Some(enum_item) => Meta::MetaVariantStruct {
                        value_type,
                        enum_item,
                        object,
                    },
                    None => Meta::MetaStruct { value_type, object },
                }
            }
        })
    }
}
