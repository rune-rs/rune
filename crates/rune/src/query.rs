//! Lazy query system, used to compile and build items on demand.

use crate::ast;
use crate::collections::{HashMap, HashSet};
use crate::error::CompileError;
use crate::traits::Resolve as _;
use runestick::{
    Call, CompileMeta, CompileMetaCapture, CompileMetaStruct, CompileMetaTuple, Hash, Item, Source,
    Span, Type, UnitBuilder,
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
    pub(crate) captures: Arc<Vec<CompileMetaCapture>>,
    /// Calling convention used for closure.
    pub(crate) call: Call,
}

pub(crate) struct AsyncBlock {
    /// Ast for block.
    pub(crate) ast: ast::ExprBlock,
    /// Captures.
    pub(crate) captures: Arc<Vec<CompileMetaCapture>>,
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

/// An entry in the build queue.
pub(crate) struct BuildEntry {
    pub(crate) item: Item,
    pub(crate) build: Build,
    pub(crate) source: Arc<Source>,
    pub(crate) source_id: usize,
}

pub(crate) struct IndexedEntry {
    pub(crate) indexed: Indexed,
    pub(crate) source: Arc<Source>,
    pub(crate) source_id: usize,
}

pub(crate) struct Query {
    pub(crate) queue: VecDeque<BuildEntry>,
    indexed: HashMap<Item, IndexedEntry>,
    pub(crate) unit: Rc<RefCell<UnitBuilder>>,
}

impl Query {
    /// Construct a new compilation context.
    pub fn new(unit: Rc<RefCell<UnitBuilder>>) -> Self {
        Self {
            queue: VecDeque::new(),
            indexed: HashMap::new(),
            unit,
        }
    }

    /// Add a new enum item.
    pub fn index_enum(
        &mut self,
        item: Item,
        source: Arc<Source>,
        source_id: usize,
        span: Span,
    ) -> Result<(), CompileError> {
        log::trace!("new enum: {}", item);
        self.index(
            item,
            IndexedEntry {
                indexed: Indexed::Enum,
                source,
                source_id,
            },
            span,
        )?;
        Ok(())
    }

    /// Add a new struct item that can be queried.
    pub fn index_struct(
        &mut self,
        item: Item,
        ast: ast::DeclStruct,
        source: Arc<Source>,
        source_id: usize,
    ) -> Result<(), CompileError> {
        log::trace!("new struct: {}", item);
        let span = ast.span();
        self.index(
            item,
            IndexedEntry {
                indexed: Indexed::Struct(Struct::new(ast)),
                source,
                source_id,
            },
            span,
        )?;
        Ok(())
    }

    /// Add a new variant item that can be queried.
    pub fn index_variant(
        &mut self,
        item: Item,
        enum_item: Item,
        ast: ast::DeclStructBody,
        source: Arc<Source>,
        source_id: usize,
        span: Span,
    ) -> Result<(), CompileError> {
        log::trace!("new variant: {}", item);
        self.index(
            item,
            IndexedEntry {
                indexed: Indexed::Variant(Variant::new(enum_item, ast)),
                source,
                source_id,
            },
            span,
        )?;
        Ok(())
    }

    /// Add a new function that can be queried for.
    pub fn index_closure(
        &mut self,
        item: Item,
        ast: ast::ExprClosure,
        captures: Arc<Vec<CompileMetaCapture>>,
        call: Call,
        source: Arc<Source>,
        source_id: usize,
    ) -> Result<(), CompileError> {
        let span = ast.span();
        log::trace!("new closure: {}", item);

        self.index(
            item,
            IndexedEntry {
                indexed: Indexed::Closure(Closure {
                    ast,
                    captures,
                    call,
                }),
                source,
                source_id,
            },
            span,
        )?;

        Ok(())
    }

    /// Add a new async block.
    pub fn index_async_block(
        &mut self,
        item: Item,
        ast: ast::ExprBlock,
        captures: Arc<Vec<CompileMetaCapture>>,
        call: Call,
        source: Arc<Source>,
        source_id: usize,
    ) -> Result<(), CompileError> {
        let span = ast.span();
        log::trace!("new closure: {}", item);

        self.index(
            item,
            IndexedEntry {
                indexed: Indexed::AsyncBlock(AsyncBlock {
                    ast,
                    captures,
                    call,
                }),
                source,
                source_id,
            },
            span,
        )?;

        Ok(())
    }

    /// Index the given element.
    pub fn index(
        &mut self,
        item: Item,
        entry: IndexedEntry,
        span: Span,
    ) -> Result<(), CompileError> {
        log::trace!("indexed: {}", item);

        self.unit.borrow_mut().insert_name(&item);

        if let Some(..) = self.indexed.insert(item.clone(), entry) {
            return Err(CompileError::ItemConflict {
                existing: item,
                span,
            });
        }

        Ok(())
    }

    /// Query for the given meta item.
    pub fn query_meta(
        &mut self,
        item: &Item,
        span: Span,
    ) -> Result<Option<CompileMeta>, CompileError> {
        let item = Item::of(item);

        if let Some(meta) = self.unit.borrow().lookup_meta(&item) {
            return Ok(Some(meta));
        }

        // See if there's an index entry we can construct.
        let IndexedEntry {
            indexed,
            source,
            source_id,
        } = match self.indexed.remove(&item) {
            Some(entry) => entry,
            None => return Ok(None),
        };

        let meta = match indexed {
            Indexed::Enum => CompileMeta::Enum {
                value_type: Type::Hash(Hash::type_hash(&item)),
                item: item.clone(),
            },
            Indexed::Variant(variant) => {
                // Assert that everything is built for the enum.
                self.query_meta(&variant.enum_item, span)?;
                self.ast_into_item_decl(&item, variant.ast, Some(variant.enum_item), source)?
            }
            Indexed::Struct(st) => self.ast_into_item_decl(&item, st.ast.body, None, source)?,
            Indexed::Function(f) => {
                self.queue.push_back(BuildEntry {
                    item: item.clone(),
                    build: Build::Function(f),
                    source,
                    source_id,
                });

                CompileMeta::Function {
                    value_type: Type::Hash(Hash::type_hash(&item)),
                    item: item.clone(),
                }
            }
            Indexed::Closure(c) => {
                let captures = c.captures.clone();
                self.queue.push_back(BuildEntry {
                    item: item.clone(),
                    build: Build::Closure(c),
                    source,
                    source_id,
                });

                CompileMeta::Closure {
                    value_type: Type::Hash(Hash::type_hash(&item)),
                    item: item.clone(),
                    captures,
                }
            }
            Indexed::AsyncBlock(async_block) => {
                let captures = async_block.captures.clone();
                self.queue.push_back(BuildEntry {
                    item: item.clone(),
                    build: Build::AsyncBlock(async_block),
                    source,
                    source_id,
                });

                CompileMeta::AsyncBlock {
                    value_type: Type::Hash(Hash::type_hash(&item)),
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
        source: Arc<Source>,
    ) -> Result<CompileMeta, CompileError> {
        let value_type = Type::Hash(Hash::type_hash(item));

        Ok(match body {
            ast::DeclStructBody::EmptyBody(..) => {
                let tuple = CompileMetaTuple {
                    item: item.clone(),
                    args: 0,
                    hash: Hash::type_hash(item),
                };

                match enum_item {
                    Some(enum_item) => CompileMeta::TupleVariant {
                        value_type,
                        enum_item,
                        tuple,
                    },
                    None => CompileMeta::Tuple { value_type, tuple },
                }
            }
            ast::DeclStructBody::TupleBody(tuple) => {
                let tuple = CompileMetaTuple {
                    item: item.clone(),
                    args: tuple.fields.len(),
                    hash: Hash::type_hash(item),
                };

                match enum_item {
                    Some(enum_item) => CompileMeta::TupleVariant {
                        value_type,
                        enum_item,
                        tuple,
                    },
                    None => CompileMeta::Tuple { value_type, tuple },
                }
            }
            ast::DeclStructBody::StructBody(st) => {
                let mut fields = HashSet::new();

                for (ident, _) in &st.fields {
                    let ident = ident.resolve(&*source)?;
                    fields.insert(ident.to_owned());
                }

                let object = CompileMetaStruct {
                    item: item.clone(),
                    fields: Some(fields),
                };

                match enum_item {
                    Some(enum_item) => CompileMeta::StructVariant {
                        value_type,
                        enum_item,
                        object,
                    },
                    None => CompileMeta::Struct { value_type, object },
                }
            }
        })
    }
}
