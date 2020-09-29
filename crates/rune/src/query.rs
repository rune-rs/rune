//! Lazy query system, used to compile and build items on demand.

use crate::ast;
use crate::collections::{HashMap, HashSet};
use crate::compiling::InsertMetaError;
use crate::ir::ir;
use crate::ir::{IrBudget, IrInterpreter};
use crate::ir::{IrCompile as _, IrCompiler};
use crate::parsing::Opaque;
use crate::shared::Consts;
use crate::{
    CompileError, CompileErrorKind, CompileVisitor, Id, IrError, IrErrorKind, ParseError,
    ParseErrorKind, Resolve as _, Spanned, Storage, UnitBuilder,
};
use runestick::{
    Call, CompileMeta, CompileMetaCapture, CompileMetaEmpty, CompileMetaKind, CompileMetaStruct,
    CompileMetaTuple, CompileSource, Hash, Item, Source, SourceId, Span, Type,
};
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Arc;
use thiserror::Error;

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

error! {
    /// An error raised during querying.
    #[derive(Debug)]
    pub struct QueryError {
        kind: QueryErrorKind,
    }

    impl From<IrError>;
    impl From<CompileError>;
    impl From<ParseError>;
}

/// Error raised during queries.
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum QueryErrorKind {
    #[error("internal error: {message}")]
    Internal { message: &'static str },
    #[error("failed to insert meta: {error}")]
    InsertMetaError {
        #[source]
        #[from]
        error: InsertMetaError,
    },
    #[error("interpreter error: {error}")]
    IrError {
        #[source]
        #[from]
        error: Box<IrErrorKind>,
    },
    #[error("compile error: {error}")]
    CompileError {
        #[source]
        #[from]
        error: Box<CompileErrorKind>,
    },
    #[error("parse error: {error}")]
    ParseError {
        #[source]
        #[from]
        error: ParseErrorKind,
    },
    #[error("missing item for {id:?}")]
    MissingItemId { id: Option<Id> },
    #[error("missing template for {id:?}")]
    MissingTemplateId { id: Option<Id> },
    #[error("missing const fn by id {id:?}")]
    MissingConstFnId { id: Option<Id> },
    #[error("found conflicting item `{existing}`")]
    ItemConflict { existing: Item },
}

pub(crate) enum Indexed {
    Enum,
    Struct(Struct),
    Variant(Variant),
    Function(Function),
    Closure(Closure),
    AsyncBlock(AsyncBlock),
    Const(Const),
    ConstFn(ConstFn),
}

pub struct Struct {
    /// The ast of the struct.
    ast: ast::ItemStruct,
}

impl Struct {
    /// Construct a new struct entry.
    pub fn new(ast: ast::ItemStruct) -> Self {
        Self { ast }
    }
}

pub struct Variant {
    /// Id of of the enum type.
    enum_id: Option<Id>,
    /// Ast for declaration.
    ast: ast::ItemVariant,
}

impl Variant {
    /// Construct a new variant.
    pub fn new(enum_id: Option<Id>, ast: ast::ItemVariant) -> Self {
        Self { enum_id, ast }
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

pub(crate) struct Const {
    /// The intermediate representation of the constant expression.
    pub(crate) ir: ir::Ir,
}

pub(crate) struct ConstFn {
    /// The const fn ast.
    pub(crate) item_fn: ast::ItemFn,
}

/// An entry in the build queue.
pub(crate) enum Build {
    Function(Function),
    InstanceFunction(InstanceFunction),
    Closure(Closure),
    AsyncBlock(AsyncBlock),
    UnusedConst(Const),
    UnusedConstFn(ConstFn),
}

/// An entry in the build queue.
pub(crate) struct BuildEntry {
    /// The span of the build entry.
    pub(crate) span: Span,
    /// The item of the build entry.
    pub(crate) item: Item,
    /// The build entry.
    pub(crate) build: Build,
    /// The source of the build entry.
    pub(crate) source: Arc<Source>,
    /// The source id of the build entry.
    pub(crate) source_id: usize,
    /// If the queued up entry was unused or not.
    pub(crate) used: Used,
}

pub(crate) struct IndexedEntry {
    /// The source location of the indexed entry.
    pub(crate) span: Span,
    /// The source of the indexed entry.
    pub(crate) source: Arc<Source>,
    /// The source id of the indexed entry.
    pub(crate) source_id: SourceId,
    /// The entry data.
    pub(crate) indexed: Indexed,
}

pub(crate) struct Query {
    /// Next opaque id generated.
    next_id: Id,
    pub(crate) storage: Storage,
    pub(crate) unit: UnitBuilder,
    /// Cache of constants that have been expanded.
    pub(crate) consts: Consts,
    /// Build queue.
    pub(crate) queue: VecDeque<BuildEntry>,
    /// Indexed items that can be queried for, which will queue up for them to
    /// be compiled.
    pub(crate) indexed: HashMap<Item, IndexedEntry>,
    /// Resolved templates.
    pub(crate) templates: HashMap<Id, Rc<ast::Template>>,
    /// Associated between `id` and `Item`. Use to look up items through
    /// `item_for` with an opaque id.
    ///
    /// These items are associated with AST elements, and encodoes the item path
    /// that the AST element was indexed.
    pub(crate) items: HashMap<Id, Item>,
    /// Reverse lookup for items to reduce the number of items used.
    pub(crate) items_rev: HashMap<Item, Id>,
    /// Compiled constant functions.
    pub(crate) const_fns: HashMap<Id, Rc<ir::IrFn>>,
}

impl Query {
    /// Construct a new compilation context.
    pub fn new(storage: Storage, unit: UnitBuilder, consts: Consts) -> Self {
        Self {
            next_id: Id::initial(),
            storage,
            unit,
            consts,
            queue: VecDeque::new(),
            indexed: HashMap::new(),
            templates: HashMap::new(),
            items: HashMap::new(),
            items_rev: HashMap::new(),
            const_fns: HashMap::new(),
        }
    }

    /// Insert an item and return its Id.
    pub(crate) fn insert_item(&mut self, item: &Item) -> Option<Id> {
        if let Some(id) = self.items_rev.get(&item) {
            return Some(*id);
        }

        let id = self.next_id.next()?;
        self.items_rev.insert(item.clone(), id);
        self.items.insert(id, item.clone());
        Some(id)
    }

    /// Insert a template and return its Id.
    pub(crate) fn insert_template(&mut self, template: ast::Template) -> Option<Id> {
        let id = self.next_id.next()?;
        self.templates.insert(id, Rc::new(template));
        Some(id)
    }

    /// Insert an item and return its Id.
    pub(crate) fn insert_const_fn(&mut self, ir_fn: ir::IrFn) -> Option<Id> {
        let id = self.next_id.next()?;
        self.const_fns.insert(id, Rc::new(ir_fn));
        Some(id)
    }

    /// Get the item for the given identifier.
    pub(crate) fn item_for<T>(&self, ast: T) -> Result<&Item, QueryError>
    where
        T: Spanned + Opaque,
    {
        let id = ast.id();

        let item = id
            .and_then(|n| self.items.get(&n))
            .ok_or_else(|| QueryError::new(ast, QueryErrorKind::MissingItemId { id }))?;

        Ok(item)
    }

    /// Get the template for the given identifier.
    pub(crate) fn template_for<T>(&self, ast: T) -> Result<Rc<ast::Template>, QueryError>
    where
        T: Spanned + Opaque,
    {
        let id = ast.id();

        let template = id
            .and_then(|n| self.templates.get(&n))
            .ok_or_else(|| QueryError::new(ast, QueryErrorKind::MissingTemplateId { id }))?;

        Ok(template.clone())
    }

    /// Get the constant function associated with the opaque.
    pub(crate) fn const_fn_for<T>(&self, ast: T) -> Result<Rc<ir::IrFn>, QueryError>
    where
        T: Spanned + Opaque,
    {
        let id = ast.id();

        let const_fn = id
            .and_then(|n| self.const_fns.get(&n))
            .ok_or_else(|| QueryError::new(ast, QueryErrorKind::MissingConstFnId { id }))?;

        Ok(const_fn.clone())
    }

    /// Index a constant expression.
    pub fn index_const<S>(
        &mut self,
        spanned: S,
        id: Option<Id>,
        source: Arc<Source>,
        source_id: usize,
        item_const: ast::ItemConst,
        span: Span,
    ) -> Result<(), QueryError>
    where
        S: Spanned,
    {
        log::trace!("new const: {:?}", id);

        let mut ir_compiler = IrCompiler {
            query: self,
            source: &*source,
            storage: &self.storage,
        };

        let ir = ir_compiler.compile(&*item_const.expr)?;

        self.index(
            spanned,
            id,
            IndexedEntry {
                span,
                source,
                source_id,
                indexed: Indexed::Const(Const { ir }),
            },
        )?;

        Ok(())
    }

    /// Index a constant function.
    pub fn index_const_fn<S>(
        &mut self,
        spanned: S,
        id: Option<Id>,
        source: Arc<Source>,
        source_id: usize,
        item_fn: ast::ItemFn,
        span: Span,
    ) -> Result<(), QueryError>
    where
        S: Spanned,
    {
        log::trace!("new const fn: {:?}", id);

        self.index(
            spanned,
            id,
            IndexedEntry {
                span,
                source,
                source_id,
                indexed: Indexed::ConstFn(ConstFn { item_fn }),
            },
        )?;

        Ok(())
    }

    /// Add a new enum item.
    pub fn index_enum<S>(
        &mut self,
        spanned: S,
        id: Option<Id>,
        source: Arc<Source>,
        source_id: usize,
        span: Span,
    ) -> Result<(), QueryError>
    where
        S: Spanned,
    {
        log::trace!("new enum: {:?}", id);

        self.index(
            spanned,
            id,
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
    pub fn index_struct<S>(
        &mut self,
        spanned: S,
        id: Option<Id>,
        ast: ast::ItemStruct,
        source: Arc<Source>,
        source_id: usize,
    ) -> Result<(), QueryError>
    where
        S: Spanned,
    {
        log::trace!("new struct: {:?}", id);
        let span = ast.span();

        self.index(
            spanned,
            id,
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
    pub fn index_variant<S>(
        &mut self,
        spanned: S,
        id: Option<Id>,
        enum_id: Option<Id>,
        ast: ast::ItemVariant,
        source: Arc<Source>,
        source_id: usize,
        span: Span,
    ) -> Result<(), QueryError>
    where
        S: Spanned,
    {
        log::trace!("new variant: {:?}", id);

        self.index(
            spanned,
            id,
            IndexedEntry {
                span,
                source,
                source_id,
                indexed: Indexed::Variant(Variant::new(enum_id, ast)),
            },
        )?;

        Ok(())
    }

    /// Add a new function that can be queried for.
    pub fn index_closure<S>(
        &mut self,
        spanned: S,
        id: Option<Id>,
        ast: ast::ExprClosure,
        captures: Arc<Vec<CompileMetaCapture>>,
        call: Call,
        source: Arc<Source>,
        source_id: usize,
    ) -> Result<(), QueryError>
    where
        S: Spanned,
    {
        let span = ast.span();
        log::trace!("new closure: {:?}", id);

        self.index(
            spanned,
            id,
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
    pub fn index_async_block<S>(
        &mut self,
        spanned: S,
        id: Option<Id>,
        ast: ast::Block,
        captures: Arc<Vec<CompileMetaCapture>>,
        call: Call,
        source: Arc<Source>,
        source_id: usize,
    ) -> Result<(), QueryError>
    where
        S: Spanned,
    {
        let span = ast.span();
        log::trace!("new closure: {:?}", id);

        self.index(
            spanned,
            id,
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
    pub fn index<S>(
        &mut self,
        spanned: S,
        id: Option<Id>,
        entry: IndexedEntry,
    ) -> Result<(), QueryError>
    where
        S: Spanned,
    {
        let item = id
            .and_then(|n| self.items.get(&n).cloned())
            .ok_or_else(|| QueryError::new(spanned, QueryErrorKind::MissingItemId { id }))?;

        log::trace!("indexed: {}", item);

        self.unit.insert_name(&item);

        if let Some(old) = self.indexed.insert(item.clone(), entry) {
            return Err(QueryError::new(
                &old.span,
                QueryErrorKind::ItemConflict { existing: item },
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
    ) -> Result<bool, (SourceId, QueryError)> {
        let unused = self
            .indexed
            .iter()
            .map(|(item, e)| (item.clone(), (e.span, e.source_id)))
            .collect::<Vec<_>>();

        if unused.is_empty() {
            return Ok(false);
        }

        for (item, (span, source_id)) in unused {
            // NB: recursive queries might remove from `indexed`, so we expect
            // to miss things here.
            if let Some(meta) = self
                .query_meta_with_use(&item, Used::Unused)
                .map_err(|e| (source_id, e))?
            {
                visitor.visit_meta(source_id, &meta, span);
            }
        }

        Ok(true)
    }

    /// Public query meta which marks things as used.
    pub(crate) fn query_meta(&mut self, item: &Item) -> Result<Option<CompileMeta>, QueryError> {
        self.query_meta_with_use(item, Used::Used)
    }

    /// Internal query meta with control over whether or not to mark things as unused.
    pub(crate) fn query_meta_with_use(
        &mut self,
        item: &Item,
        used: Used,
    ) -> Result<Option<CompileMeta>, QueryError> {
        if let Some(meta) = self.unit.lookup_meta(item) {
            return Ok(Some(meta));
        }

        // See if there's an index entry we can construct.
        let entry = match self.indexed.remove(item) {
            Some(entry) => entry,
            None => return Ok(None),
        };

        Ok(Some(self.build_indexed_entry(item, entry, used)?))
    }

    /// Build a single, indexed entry and return its metadata.
    pub(crate) fn build_indexed_entry(
        &mut self,
        item: &Item,
        entry: IndexedEntry,
        used: Used,
    ) -> Result<CompileMeta, QueryError> {
        let IndexedEntry {
            span: entry_span,
            indexed,
            source,
            source_id,
        } = entry;

        let path = source.path().map(ToOwned::to_owned);

        let kind = match indexed {
            Indexed::Enum => CompileMetaKind::Enum {
                type_of: Type::from(Hash::type_hash(item)),
                item: item.clone(),
            },
            Indexed::Variant(variant) => {
                let enum_item = self.item_for((entry_span, variant.enum_id))?.clone();
                // Assert that everything is built for the enum.
                self.query_meta(&enum_item)?;
                self.variant_into_item_decl(item, variant.ast.body, Some(&enum_item), &*source)?
            }
            Indexed::Struct(st) => self.struct_into_item_decl(item, st.ast.body, None, &*source)?,
            Indexed::Function(f) => {
                self.queue.push_back(BuildEntry {
                    span: f.ast.span(),
                    item: item.clone(),
                    build: Build::Function(f),
                    source,
                    source_id,
                    used,
                });

                CompileMetaKind::Function {
                    type_of: Type::from(Hash::type_hash(item)),
                    item: item.clone(),
                }
            }
            Indexed::Closure(c) => {
                let captures = c.captures.clone();

                self.queue.push_back(BuildEntry {
                    span: c.ast.span(),
                    item: item.clone(),
                    build: Build::Closure(c),
                    source,
                    source_id,
                    used,
                });

                CompileMetaKind::Closure {
                    type_of: Type::from(Hash::type_hash(item)),
                    item: item.clone(),
                    captures,
                }
            }
            Indexed::AsyncBlock(b) => {
                let captures = b.captures.clone();

                self.queue.push_back(BuildEntry {
                    span: b.ast.span(),
                    item: item.clone(),
                    build: Build::AsyncBlock(b),
                    source,
                    source_id,
                    used,
                });

                CompileMetaKind::AsyncBlock {
                    type_of: Type::from(Hash::type_hash(item)),
                    item: item.clone(),
                    captures,
                }
            }
            Indexed::Const(c) => {
                let mut const_compiler = IrInterpreter {
                    budget: IrBudget::new(1_000_000),
                    scopes: Default::default(),
                    item: item.clone(),
                    query: self,
                };

                let const_value = const_compiler.eval_const(&c.ir, used)?;

                if used.is_unused() {
                    self.queue.push_back(BuildEntry {
                        span: c.ir.span(),
                        item: item.clone(),
                        build: Build::UnusedConst(c),
                        source,
                        source_id,
                        used,
                    });
                }

                CompileMetaKind::Const {
                    const_value,
                    item: item.clone(),
                }
            }
            Indexed::ConstFn(c) => {
                let mut ir_compiler = IrCompiler {
                    query: self,
                    source: &*source,
                    storage: &self.storage,
                };

                let ir_fn = ir_compiler.compile(&c.item_fn)?;

                let id = if used.is_unused() {
                    self.queue.push_back(BuildEntry {
                        span: c.item_fn.span(),
                        item: item.clone(),
                        build: Build::UnusedConstFn(c),
                        source,
                        source_id,
                        used,
                    });

                    None
                } else {
                    self.insert_const_fn(ir_fn)
                };

                CompileMetaKind::ConstFn {
                    id,
                    item: item.clone(),
                }
            }
        };

        let meta = CompileMeta {
            kind,
            source: Some(CompileSource {
                span: entry_span,
                path,
                source_id,
            }),
        };

        self.unit
            .insert_meta(meta.clone())
            .map_err(|error| QueryError::new(entry_span, error))?;

        Ok(meta)
    }

    /// Construct metadata for an empty body.
    fn unit_body_meta(&self, item: &Item, enum_item: Option<&Item>) -> CompileMetaKind {
        let type_of = Type::from(Hash::type_hash(item));

        let empty = CompileMetaEmpty {
            item: item.clone(),
            hash: Hash::type_hash(item),
        };

        match enum_item {
            Some(enum_item) => CompileMetaKind::UnitVariant {
                type_of,
                enum_item: enum_item.clone(),
                empty,
            },
            None => CompileMetaKind::UnitStruct { type_of, empty },
        }
    }

    /// Construct metadata for an empty body.
    fn tuple_body_meta(
        &self,
        item: &Item,
        enum_item: Option<&Item>,
        tuple: ast::Parenthesized<ast::Field, ast::Comma>,
    ) -> CompileMetaKind {
        let type_of = Type::from(Hash::type_hash(item));

        let tuple = CompileMetaTuple {
            item: item.clone(),
            args: tuple.len(),
            hash: Hash::type_hash(item),
        };

        match enum_item {
            Some(enum_item) => CompileMetaKind::TupleVariant {
                type_of,
                enum_item: enum_item.clone(),
                tuple,
            },
            None => CompileMetaKind::TupleStruct { type_of, tuple },
        }
    }

    /// Construct metadata for a struct body.
    fn struct_body_meta(
        &self,
        item: &Item,
        enum_item: Option<&Item>,
        source: &Source,
        st: ast::Braced<ast::Field, ast::Comma>,
    ) -> Result<CompileMetaKind, QueryError> {
        let type_of = Type::from(Hash::type_hash(item));

        let mut fields = HashSet::new();

        for (ast::Field { name, .. }, _) in st {
            let name = name.resolve(&self.storage, &*source)?;
            fields.insert(name.to_string());
        }

        let object = CompileMetaStruct {
            item: item.clone(),
            fields: Some(fields),
        };

        Ok(match enum_item {
            Some(enum_item) => CompileMetaKind::StructVariant {
                type_of,
                enum_item: enum_item.clone(),
                object,
            },
            None => CompileMetaKind::Struct { type_of, object },
        })
    }

    /// Convert an ast declaration into a struct.
    fn variant_into_item_decl(
        &self,
        item: &Item,
        body: ast::ItemVariantBody,
        enum_item: Option<&Item>,
        source: &Source,
    ) -> Result<CompileMetaKind, QueryError> {
        Ok(match body {
            ast::ItemVariantBody::UnitBody => self.unit_body_meta(item, enum_item),
            ast::ItemVariantBody::TupleBody(tuple) => self.tuple_body_meta(item, enum_item, tuple),
            ast::ItemVariantBody::StructBody(st) => {
                self.struct_body_meta(item, enum_item, source, st)?
            }
        })
    }

    /// Convert an ast declaration into a struct.
    fn struct_into_item_decl(
        &self,
        item: &Item,
        body: ast::ItemStructBody,
        enum_item: Option<&Item>,
        source: &Source,
    ) -> Result<CompileMetaKind, QueryError> {
        Ok(match body {
            ast::ItemStructBody::UnitBody => self.unit_body_meta(item, enum_item),
            ast::ItemStructBody::TupleBody(tuple) => self.tuple_body_meta(item, enum_item, tuple),
            ast::ItemStructBody::StructBody(st) => {
                self.struct_body_meta(item, enum_item, source, st)?
            }
        })
    }
}
