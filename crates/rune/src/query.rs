//! Lazy query system, used to compile and build items on demand.

use crate::ast;
use crate::collections::{HashMap, HashSet};
use crate::CompileResult;
use crate::{
    CompileError, CompileErrorKind, CompileVisitor, Resolve as _, SourceId, Spanned as _, Storage,
    UnitBuilder,
};
use runestick::{
    Call, CompileMeta, CompileMetaCapture, CompileMetaKind, CompileMetaStruct, CompileMetaTuple,
    Hash, Item, Source, Span, Type,
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
    ast: ast::ItemStruct,
}

impl Struct {
    /// Construct a new struct entry.
    pub fn new(ast: ast::ItemStruct) -> Self {
        Self { ast }
    }
}

pub struct Variant {
    /// Item of the enum type.
    enum_item: Item,
    /// Ast for declaration.
    ast: ast::ItemEnumVariant,
}

impl Variant {
    /// Construct a new variant.
    pub fn new(enum_item: Item, ast: ast::ItemEnumVariant) -> Self {
        Self { enum_item, ast }
    }
}

pub(crate) struct Function {
    /// Ast for declaration.
    pub(crate) ast: ast::ItemFn,
    pub(crate) call: Call,
}

pub(crate) struct InstanceFunction {
    /// Ast for the instance function.
    pub(crate) ast: ast::ItemFn,
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
    pub(crate) ast: ast::Block,
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
    /// If the queued up entry was unused or not.
    pub(crate) unused: bool,
}

pub(crate) struct IndexedEntry {
    /// The source location of the indexed entry.
    pub(crate) span: Span,
    /// The source of the indexed entry.
    pub(crate) source: Arc<Source>,
    /// The source id of the indexed entry.
    pub(crate) source_id: usize,
    /// The entry data.
    pub(crate) indexed: Indexed,
}

pub(crate) struct Query {
    pub(crate) storage: Storage,
    pub(crate) unit: Rc<RefCell<UnitBuilder>>,
    pub(crate) queue: VecDeque<BuildEntry>,
    pub(crate) indexed: HashMap<Item, IndexedEntry>,
}

impl Query {
    /// Construct a new compilation context.
    pub fn new(storage: Storage, unit: Rc<RefCell<UnitBuilder>>) -> Self {
        Self {
            storage,
            unit,
            queue: VecDeque::new(),
            indexed: HashMap::new(),
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
                span,
                source,
                source_id,
                indexed: Indexed::Enum,
            },
        )?;

        Ok(())
    }

    /// Add a new struct item that can be queried.
    pub fn index_struct(
        &mut self,
        item: Item,
        ast: ast::ItemStruct,
        source: Arc<Source>,
        source_id: usize,
    ) -> Result<(), CompileError> {
        log::trace!("new struct: {}", item);
        let span = ast.span();

        self.index(
            item,
            IndexedEntry {
                span,
                source,
                source_id,
                indexed: Indexed::Struct(Struct::new(ast)),
            },
        )?;

        Ok(())
    }

    /// Add a new variant item that can be queried.
    pub fn index_variant(
        &mut self,
        item: Item,
        enum_item: Item,
        ast: ast::ItemEnumVariant,
        source: Arc<Source>,
        source_id: usize,
        span: Span,
    ) -> Result<(), CompileError> {
        log::trace!("new variant: {}", item);

        self.index(
            item,
            IndexedEntry {
                span,
                source,
                source_id,
                indexed: Indexed::Variant(Variant::new(enum_item, ast)),
            },
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
                span,
                source,
                source_id,
                indexed: Indexed::Closure(Closure {
                    ast,
                    captures,
                    call,
                }),
            },
        )?;

        Ok(())
    }

    /// Add a new async block.
    pub fn index_async_block(
        &mut self,
        item: Item,
        ast: ast::Block,
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
                span,
                source,
                source_id,
                indexed: Indexed::AsyncBlock(AsyncBlock {
                    ast,
                    captures,
                    call,
                }),
            },
        )?;

        Ok(())
    }

    /// Index the given element.
    pub fn index(&mut self, item: Item, entry: IndexedEntry) -> Result<(), CompileError> {
        log::trace!("indexed: {}", item);

        self.unit.borrow_mut().insert_name(&item);

        if let Some(old) = self.indexed.insert(item.clone(), entry) {
            return Err(CompileError::new(
                old.span,
                CompileErrorKind::ItemConflict { existing: item },
            ));
        }

        Ok(())
    }

    /// Remove and queue up unused entries for building.
    ///
    /// Returns boolean indicating if any unused entries were queued up.
    pub(crate) fn queue_unused_entries(
        &mut self,
        visitor: &mut dyn CompileVisitor,
    ) -> Result<bool, (SourceId, CompileError)> {
        let unused = std::mem::take(&mut self.indexed);

        if unused.is_empty() {
            return Ok(false);
        }

        for (item, entry) in unused {
            let span = entry.span;
            let source_id = entry.source_id;
            let url = entry.source.url().cloned();

            let meta = self
                .build_indexed_entry(item, entry, true)
                .map_err(|error| (source_id, error))?;

            if let Some(url) = url {
                visitor.visit_meta(&url, &meta, span);
            }
        }

        Ok(true)
    }

    /// Query for the given meta item.
    pub fn query_meta(&mut self, item: &Item) -> Result<Option<CompileMeta>, CompileError> {
        let item = Item::of(item);

        if let Some(meta) = self.unit.borrow().lookup_meta(&item) {
            return Ok(Some(meta));
        }

        // See if there's an index entry we can construct.
        let entry = match self.indexed.remove(&item) {
            Some(entry) => entry,
            None => return Ok(None),
        };

        Ok(Some(self.build_indexed_entry(item, entry, false)?))
    }

    /// Build a single, indexed entry and return its metadata.
    pub(crate) fn build_indexed_entry(
        &mut self,
        item: Item,
        entry: IndexedEntry,
        unused: bool,
    ) -> Result<CompileMeta, CompileError> {
        let IndexedEntry {
            span: entry_span,
            indexed,
            source,
            source_id,
        } = entry;

        let url = source.url().cloned();

        let kind = match indexed {
            Indexed::Enum => CompileMetaKind::Enum {
                type_of: Type::from(Hash::type_hash(&item)),
                item: item.clone(),
            },
            Indexed::Variant(variant) => {
                // Assert that everything is built for the enum.
                self.query_meta(&variant.enum_item)?;
                self.variant_into_item_decl(&item, variant.ast, Some(variant.enum_item), &*source)?
            }
            Indexed::Struct(st) => {
                self.struct_into_item_decl(&item, st.ast.body, None, &*source)?
            }
            Indexed::Function(f) => {
                self.queue.push_back(BuildEntry {
                    item: item.clone(),
                    build: Build::Function(f),
                    source,
                    source_id,
                    unused,
                });

                CompileMetaKind::Function {
                    type_of: Type::from(Hash::type_hash(&item)),
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
                    unused,
                });

                CompileMetaKind::Closure {
                    type_of: Type::from(Hash::type_hash(&item)),
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
                    unused,
                });

                CompileMetaKind::AsyncBlock {
                    type_of: Type::from(Hash::type_hash(&item)),
                    item: item.clone(),
                    captures,
                }
            }
        };

        let meta = CompileMeta {
            span: Some(entry_span),
            url,
            kind,
        };

        self.unit.borrow_mut().insert_meta(meta.clone())?;
        Ok(meta)
    }

    /// Construct metadata for an empty body.
    fn empty_body_meta(&self, item: &Item, enum_item: Option<Item>) -> CompileMetaKind {
        let type_of = Type::from(Hash::type_hash(item));

        let tuple = CompileMetaTuple {
            item: item.clone(),
            args: 0,
            hash: Hash::type_hash(item),
        };

        match enum_item {
            Some(enum_item) => CompileMetaKind::TupleVariant {
                type_of,
                enum_item,
                tuple,
            },
            None => CompileMetaKind::Tuple { type_of, tuple },
        }
    }

    /// Construct metadata for an empty body.
    fn tuple_body_meta(
        &self,
        item: &Item,
        enum_item: Option<Item>,
        tuple: ast::TupleBody,
    ) -> CompileMetaKind {
        let type_of = Type::from(Hash::type_hash(item));

        let tuple = CompileMetaTuple {
            item: item.clone(),
            args: tuple.fields.len(),
            hash: Hash::type_hash(item),
        };

        match enum_item {
            Some(enum_item) => CompileMetaKind::TupleVariant {
                type_of,
                enum_item,
                tuple,
            },
            None => CompileMetaKind::Tuple { type_of, tuple },
        }
    }

    /// Construct metadata for a struct body.
    fn struct_body_meta(
        &self,
        item: &Item,
        enum_item: Option<Item>,
        source: &Source,
        st: ast::StructBody,
    ) -> CompileResult<CompileMetaKind> {
        let type_of = Type::from(Hash::type_hash(item));

        let mut fields = HashSet::new();

        for (ident, _) in &st.fields {
            let ident = ident.resolve(&self.storage, &*source)?;
            fields.insert(ident.to_string());
        }

        let object = CompileMetaStruct {
            item: item.clone(),
            fields: Some(fields),
        };

        Ok(match enum_item {
            Some(enum_item) => CompileMetaKind::StructVariant {
                type_of,
                enum_item,
                object,
            },
            None => CompileMetaKind::Struct { type_of, object },
        })
    }

    /// Convert an ast declaration into a struct.
    fn variant_into_item_decl(
        &self,
        item: &Item,
        body: ast::ItemEnumVariant,
        enum_item: Option<Item>,
        source: &Source,
    ) -> Result<CompileMetaKind, CompileError> {
        Ok(match body {
            ast::ItemEnumVariant::EmptyBody => self.empty_body_meta(item, enum_item),
            ast::ItemEnumVariant::TupleBody(tuple) => self.tuple_body_meta(item, enum_item, tuple),
            ast::ItemEnumVariant::StructBody(st) => {
                self.struct_body_meta(item, enum_item, source, st)?
            }
        })
    }

    /// Convert an ast declaration into a struct.
    fn struct_into_item_decl(
        &self,
        item: &Item,
        body: ast::ItemStructBody,
        enum_item: Option<Item>,
        source: &Source,
    ) -> Result<CompileMetaKind, CompileError> {
        Ok(match body {
            ast::ItemStructBody::EmptyBody(_) => self.empty_body_meta(item, enum_item),
            ast::ItemStructBody::TupleBody(tuple, _) => {
                self.tuple_body_meta(item, enum_item, tuple)
            }
            ast::ItemStructBody::StructBody(st) => {
                self.struct_body_meta(item, enum_item, source, st)?
            }
        })
    }
}
