//! Lazy query system, used to compile and build items on demand.

use crate::ast;
use crate::collections::{HashMap, HashSet};
use crate::compiling::InsertMetaError;
use crate::indexing::Visibility;
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
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;
use thiserror::Error;

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
    #[error("missing {what} for id {id:?}")]
    MissingId { what: &'static str, id: Option<Id> },
    #[error("found conflicting item `{existing}`")]
    ItemConflict { existing: Item },
    #[error("item `{item}` with {visibility} visibility, is not accessible from here")]
    NotVisible {
        visibility: Visibility,
        item: Item,
        from: Item,
    },
    #[error("module `{item}` with {visibility} visibility, is not accessible from here")]
    NotVisibleMod { visibility: Visibility, item: Item },
    #[error("missing reverse lookup for `{item}`")]
    MissingRevItem { item: Item },
    #[error("missing item for id {id:?}")]
    MissingRevId { id: Id },
    #[error("missing query meta for module {item}")]
    MissingMod { item: Item },
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
    items: HashMap<Id, Rc<QueryItem>>,
    /// Modules and associated metadata.
    modules: HashMap<Item, Rc<QueryMod>>,
    /// Reverse lookup for items to reduce the number of items used.
    items_rev: HashMap<Item, Rc<QueryItem>>,
    /// Compiled constant functions.
    const_fns: HashMap<Id, Rc<QueryConstFn>>,
    /// Query paths.
    query_paths: HashMap<Id, Rc<QueryPath>>,
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
            modules: HashMap::new(),
            items_rev: HashMap::new(),
            const_fns: HashMap::new(),
            query_paths: HashMap::new(),
        }
    }

    /// Insert path information.
    pub(crate) fn insert_path(
        &mut self,
        mod_item: &Rc<QueryMod>,
        impl_item: Option<&Rc<Item>>,
        item: &Item,
    ) -> Option<Id> {
        let query_path = Rc::new(QueryPath {
            mod_item: mod_item.clone(),
            impl_item: impl_item.cloned().clone(),
            item: item.clone(),
        });

        let id = self.next_id.next()?;
        self.query_paths.insert(id, query_path);
        Some(id)
    }

    /// Insert module and associated metadata.
    pub(crate) fn insert_mod(
        &mut self,
        spanned: Span,
        item: &Item,
        visibility: Visibility,
    ) -> Result<(Id, Rc<QueryMod>), CompileError> {
        let id = self.next_id.next().expect("ran out of ids");

        let query_mod = Rc::new(QueryMod {
            item: item.clone(),
            visibility,
        });

        self.unit.insert_name(spanned, item)?;
        self.modules.insert(item.clone(), query_mod.clone());
        Ok((id, query_mod))
    }

    /// Insert an item and return its Id.
    pub(crate) fn insert_item(
        &mut self,
        span: Span,
        item: &Item,
        mod_item: &Rc<QueryMod>,
        visibility: Visibility,
    ) -> Result<(Id, Rc<QueryItem>), CompileError> {
        if let Some(existing) = self.items_rev.get(item) {
            return Ok((existing.id, existing.clone()));
        }

        let id = self.next_id.next().expect("ran out of ids");

        let query_item = Rc::new(QueryItem {
            id,
            item: item.clone(),
            mod_item: mod_item.clone(),
            visibility,
        });

        if let Some(..) = self.items_rev.insert(item.clone(), query_item.clone()) {
            return Err(CompileError::new(
                span,
                CompileErrorKind::ItemConflict { item: item.clone() },
            ));
        }

        self.items.insert(id, query_item.clone());
        Ok((id, query_item))
    }

    /// Insert a template and return its Id.
    pub(crate) fn insert_template(&mut self, template: ast::Template) -> Id {
        let id = self.next_id.next().expect("ran out of ids");
        self.templates.insert(id, Rc::new(template));
        id
    }

    /// Insert an item and return its Id.
    pub(crate) fn insert_const_fn(&mut self, item: &Rc<QueryItem>, ir_fn: ir::IrFn) -> Id {
        let id = self.next_id.next().expect("ran out of ids");
        self.const_fns.insert(
            id,
            Rc::new(QueryConstFn {
                item: item.clone(),
                ir_fn,
            }),
        );
        id
    }

    /// Get path information for the given ast.
    pub(crate) fn path_for<T>(&self, ast: T) -> Result<&Rc<QueryPath>, QueryError>
    where
        T: Spanned + Opaque,
    {
        let id = ast.id();

        let query_path = id
            .and_then(|n| self.query_paths.get(&n))
            .ok_or_else(|| QueryError::new(ast, QueryErrorKind::MissingId { what: "path", id }))?;

        Ok(query_path)
    }

    /// Get the item for the given identifier.
    pub(crate) fn item_for<T>(&self, ast: T) -> Result<&Rc<QueryItem>, QueryError>
    where
        T: Spanned + Opaque,
    {
        let id = ast.id();

        let item = id
            .and_then(|n| self.items.get(&n))
            .ok_or_else(|| QueryError::new(ast, QueryErrorKind::MissingId { what: "item", id }))?;

        Ok(item)
    }

    /// Get the template for the given identifier.
    pub(crate) fn template_for<T>(&self, ast: T) -> Result<Rc<ast::Template>, QueryError>
    where
        T: Spanned + Opaque,
    {
        let id = ast.id();

        let template = id.and_then(|n| self.templates.get(&n)).ok_or_else(|| {
            QueryError::new(
                ast,
                QueryErrorKind::MissingId {
                    what: "template",
                    id,
                },
            )
        })?;

        Ok(template.clone())
    }

    /// Get the constant function associated with the opaque.
    pub(crate) fn const_fn_for<T>(&self, ast: T) -> Result<Rc<QueryConstFn>, QueryError>
    where
        T: Spanned + Opaque,
    {
        let id = ast.id();

        let const_fn = id.and_then(|n| self.const_fns.get(&n)).ok_or_else(|| {
            QueryError::new(
                ast,
                QueryErrorKind::MissingId {
                    what: "constant function",
                    id,
                },
            )
        })?;

        Ok(const_fn.clone())
    }

    /// Index a constant expression.
    pub fn index_const(
        &mut self,
        item: &Rc<QueryItem>,
        source: Arc<Source>,
        source_id: usize,
        item_const: ast::ItemConst,
        span: Span,
    ) -> Result<(), QueryError> {
        log::trace!("new const: {:?}", item.item);

        let mut ir_compiler = IrCompiler {
            query: self,
            source: &*source,
            storage: &self.storage,
        };

        let ir = ir_compiler.compile(&*item_const.expr)?;

        self.index(
            &item.item,
            IndexedEntry {
                item: item.clone(),
                span,
                source,
                source_id,
                indexed: Indexed::Const(Const {
                    mod_item: item.mod_item.clone(),
                    ir,
                }),
            },
        )?;

        Ok(())
    }

    /// Index a constant function.
    pub fn index_const_fn(
        &mut self,
        item: &Rc<QueryItem>,
        source: Arc<Source>,
        source_id: usize,
        item_fn: ast::ItemFn,
        span: Span,
    ) -> Result<(), QueryError> {
        log::trace!("new const fn: {:?}", item.item);

        self.index(
            &item.item,
            IndexedEntry {
                item: item.clone(),
                span,
                source,
                source_id,
                indexed: Indexed::ConstFn(ConstFn { item_fn }),
            },
        )?;

        Ok(())
    }

    /// Add a new enum item.
    pub fn index_enum(
        &mut self,
        item: &Rc<QueryItem>,
        source: Arc<Source>,
        source_id: usize,
        span: Span,
    ) -> Result<(), QueryError> {
        log::trace!("new enum: {:?}", item.item);

        self.index(
            &item.item,
            IndexedEntry {
                item: item.clone(),
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
        item: &Rc<QueryItem>,
        ast: ast::ItemStruct,
        source: Arc<Source>,
        source_id: usize,
    ) -> Result<(), QueryError> {
        log::trace!("new struct: {:?}", item);

        self.index(
            &item.item,
            IndexedEntry {
                item: item.clone(),
                span: ast.span(),
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
        item: &Rc<QueryItem>,
        enum_id: Id,
        ast: ast::ItemVariant,
        source: Arc<Source>,
        source_id: usize,
        span: Span,
    ) -> Result<(), QueryError> {
        log::trace!("new variant: {:?}", item);

        self.index(
            &item.item,
            IndexedEntry {
                item: item.clone(),
                span,
                source,
                source_id,
                indexed: Indexed::Variant(Variant::new(enum_id, ast)),
            },
        )?;

        Ok(())
    }

    /// Add a new function that can be queried for.
    pub fn index_closure(
        &mut self,
        item: &Rc<QueryItem>,
        ast: ast::ExprClosure,
        captures: Arc<Vec<CompileMetaCapture>>,
        call: Call,
        source: Arc<Source>,
        source_id: usize,
    ) -> Result<(), QueryError> {
        log::trace!("new closure: {:?}", item.item);

        self.index(
            &item.item,
            IndexedEntry {
                item: item.clone(),
                span: ast.span(),
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
        item: &Rc<QueryItem>,
        ast: ast::Block,
        captures: Arc<Vec<CompileMetaCapture>>,
        call: Call,
        source: Arc<Source>,
        source_id: usize,
    ) -> Result<(), QueryError> {
        log::trace!("new closure: {:?}", item.item);

        self.index(
            &item.item,
            IndexedEntry {
                item: item.clone(),
                span: ast.span(),
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
    pub fn index(&mut self, item: &Item, entry: IndexedEntry) -> Result<(), QueryError> {
        log::trace!("indexed: {}", item);
        self.unit.insert_name(entry.span, item)?;

        if let Some(old) = self.indexed.insert(item.clone(), entry) {
            return Err(QueryError::new(
                &old.span,
                QueryErrorKind::ItemConflict {
                    existing: item.clone(),
                },
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
            .values()
            .map(|e| (e.item.clone(), e.span, e.source_id))
            .collect::<Vec<_>>();

        if unused.is_empty() {
            return Ok(false);
        }

        for (item, span, source_id) in unused {
            // NB: recursive queries might remove from `indexed`, so we expect
            // to miss things here.
            if let Some(meta) = self
                .query_meta_with(span, None, &*item, Used::Unused)
                .map_err(|e| (source_id, e))?
            {
                visitor.visit_meta(source_id, &meta, span);
            }
        }

        Ok(true)
    }

    /// Public query meta which marks things as used.
    pub(crate) fn query_meta(
        &mut self,
        spanned: Span,
        from: Option<&QueryMod>,
        item: &Item,
        used: Used,
    ) -> Result<Option<CompileMeta>, QueryError> {
        let item = match self.items_rev.get(item) {
            Some(item) => item.clone(),
            None => return Ok(None),
        };

        self.query_meta_with(spanned, from, &*item, used)
    }

    /// Query the exact meta item without performing a reverse lookup for it.
    pub(crate) fn query_meta_with(
        &mut self,
        spanned: Span,
        from: Option<&QueryMod>,
        item: &QueryItem,
        used: Used,
    ) -> Result<Option<CompileMeta>, QueryError> {
        // Test for visibility if from is specified.
        if let Some(from) = from {
            self.check_access_from(spanned, from, item)?;
        }

        if let Some(meta) = self.unit.lookup_meta(&item.item) {
            return Ok(Some(meta));
        }

        // See if there's an index entry we can construct and insert.
        let entry = match self.indexed.remove(&item.item) {
            Some(entry) => entry,
            None => return Ok(None),
        };

        let meta = self.build_indexed_entry(spanned, from, item, entry, used)?;

        self.unit
            .insert_meta(meta.clone())
            .map_err(|error| QueryError::new(spanned, error))?;

        Ok(Some(meta))
    }

    /// Build a single, indexed entry and return its metadata.
    fn build_indexed_entry(
        &mut self,
        spanned: Span,
        from: Option<&QueryMod>,
        item: &QueryItem,
        entry: IndexedEntry,
        used: Used,
    ) -> Result<CompileMeta, QueryError> {
        let IndexedEntry {
            span,
            indexed,
            source,
            source_id,
            item: entry_item,
        } = entry;

        let path = source.path().map(ToOwned::to_owned);

        let kind = match indexed {
            Indexed::Enum => CompileMetaKind::Enum {
                type_of: Type::from(Hash::type_hash(&item.item)),
            },
            Indexed::Variant(variant) => {
                let enum_item = self.item_for((span, variant.enum_id))?.clone();
                // Assert that everything is built for the enum.
                self.query_meta_with(spanned, from, &enum_item, Default::default())?;
                self.variant_into_item_decl(
                    &item.item,
                    variant.ast.body,
                    Some(&enum_item.item),
                    &*source,
                )?
            }
            Indexed::Struct(st) => {
                self.struct_into_item_decl(&item.item, st.ast.body, None, &*source)?
            }
            Indexed::Function(f) => {
                self.queue.push_back(BuildEntry {
                    span: f.ast.span(),
                    item: item.item.clone(),
                    build: Build::Function(f),
                    source,
                    source_id,
                    used,
                });

                CompileMetaKind::Function {
                    type_of: Type::from(Hash::type_hash(&item.item)),
                }
            }
            Indexed::Closure(c) => {
                let captures = c.captures.clone();

                self.queue.push_back(BuildEntry {
                    span: c.ast.span(),
                    item: item.item.clone(),
                    build: Build::Closure(c),
                    source,
                    source_id,
                    used,
                });

                CompileMetaKind::Closure {
                    type_of: Type::from(Hash::type_hash(&item.item)),
                    captures,
                }
            }
            Indexed::AsyncBlock(b) => {
                let captures = b.captures.clone();

                self.queue.push_back(BuildEntry {
                    span: b.ast.span(),
                    item: item.item.clone(),
                    build: Build::AsyncBlock(b),
                    source,
                    source_id,
                    used,
                });

                CompileMetaKind::AsyncBlock {
                    type_of: Type::from(Hash::type_hash(&item.item)),
                    captures,
                }
            }
            Indexed::Const(c) => {
                let mut const_compiler = IrInterpreter {
                    budget: IrBudget::new(1_000_000),
                    scopes: Default::default(),
                    mod_item: c.mod_item.clone(),
                    item: item.item.clone(),
                    query: self,
                };

                let const_value = const_compiler.eval_const(&c.ir, used)?;

                if used.is_unused() {
                    self.queue.push_back(BuildEntry {
                        span: c.ir.span(),
                        item: item.item.clone(),
                        build: Build::UnusedConst(c),
                        source,
                        source_id,
                        used,
                    });
                }

                CompileMetaKind::Const { const_value }
            }
            Indexed::ConstFn(c) => {
                let mut ir_compiler = IrCompiler {
                    query: self,
                    source: &*source,
                    storage: &self.storage,
                };

                let ir_fn = ir_compiler.compile(&c.item_fn)?;

                let id = self.insert_const_fn(&entry_item, ir_fn);

                if used.is_unused() {
                    self.queue.push_back(BuildEntry {
                        span: c.item_fn.span(),
                        item: item.item.clone(),
                        build: Build::UnusedConstFn(c),
                        source,
                        source_id,
                        used,
                    });
                }

                CompileMetaKind::ConstFn { id }
            }
        };

        Ok(CompileMeta {
            item: item.item.clone(),
            kind,
            source: Some(CompileSource {
                span,
                path,
                source_id,
            }),
        })
    }

    fn check_access_from(
        &self,
        spanned: Span,
        from: &QueryMod,
        item: &QueryItem,
    ) -> Result<(), QueryError> {
        let (mut mod_item, suffix, is_strict_prefix) =
            from.item.module_difference(&item.mod_item.item);

        // NB: if we are an immediate parent module, we're allowed to peek into
        // a nested private module in one level of depth.
        let mut permit_one_level = is_strict_prefix;
        let mut suffix_len = 0;

        for c in &suffix {
            suffix_len += 1;
            let permit_one_level = std::mem::take(&mut permit_one_level);

            mod_item.push(c);

            let m = self.modules.get(&mod_item).ok_or_else(|| {
                QueryError::new(
                    spanned,
                    QueryErrorKind::MissingMod {
                        item: mod_item.clone(),
                    },
                )
            })?;

            match m.visibility {
                Visibility::Public => (),
                Visibility::Crate => (),
                Visibility::Inherited => {
                    if !permit_one_level {
                        return Err(QueryError::new(
                            spanned,
                            QueryErrorKind::NotVisibleMod {
                                visibility: m.visibility,
                                item: mod_item,
                            },
                        ));
                    }
                }
            }
        }

        if suffix_len == 0 || is_strict_prefix && suffix_len > 1 {
            match item.visibility {
                Visibility::Inherited => {
                    if !from.item.can_see_private_mod(&item.mod_item.item) {
                        return Err(QueryError::new(
                            spanned,
                            QueryErrorKind::NotVisible {
                                visibility: item.visibility,
                                item: item.item.clone(),
                                from: from.item.clone(),
                            },
                        ));
                    }
                }
                Visibility::Public => (),
                Visibility::Crate => (),
            }
        }

        Ok(())
    }

    /// Construct metadata for an empty body.
    fn unit_body_meta(&self, item: &Item, enum_item: Option<&Item>) -> CompileMetaKind {
        let type_of = Type::from(Hash::type_hash(item));

        let empty = CompileMetaEmpty {
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

    /// Perform a path lookup on the current state of the unit.
    pub(crate) fn convert_path(
        &mut self,
        base: &Item,
        mod_item: &QueryMod,
        impl_item: Option<&Item>,
        path: &ast::Path,
        storage: &Storage,
        source: &Source,
    ) -> Result<Named, CompileError> {
        if let Some(global) = &path.global {
            return Err(CompileError::internal(
                global,
                "global scopes are not supported yet",
            ));
        }

        let mut in_self_type = false;
        let mut local = None;

        let mut item = match &path.first {
            ast::PathSegment::Ident(ident) => {
                let ident = ident.resolve(storage, source)?;

                let item = if let Some(entry) =
                    self.unit
                        .walk_names(path.span(), mod_item, &base, ident.as_ref())?
                {
                    entry
                } else {
                    Item::of(&[ident.as_ref()])
                };

                if path.rest.is_empty() {
                    local = Some(<Box<str>>::from(ident));
                }

                item
            }
            ast::PathSegment::Super(super_value) => {
                let mut item = mod_item.item.clone();

                item.pop()
                    .ok_or_else(CompileError::unsupported_super(super_value))?;

                item
            }
            ast::PathSegment::SelfType(self_type) => {
                let impl_item = impl_item.ok_or_else(|| {
                    CompileError::new(self_type, CompileErrorKind::UnsupportedSelfType)
                })?;

                in_self_type = true;
                impl_item.clone()
            }
            ast::PathSegment::SelfValue(..) => mod_item.item.clone(),
            ast::PathSegment::Crate(..) => Item::new(),
        };

        for (_, segment) in &path.rest {
            log::trace!("item = {}", item);

            match segment {
                ast::PathSegment::Ident(ident) => {
                    let ident = ident.resolve(storage, source)?;
                    item.push(ident.as_ref());

                    if let Some(new) = self.unit.get_import(path.span(), &item)? {
                        item = new;
                    }
                }
                ast::PathSegment::Super(super_token) => {
                    if in_self_type {
                        return Err(CompileError::new(
                            super_token,
                            CompileErrorKind::UnsupportedSuperInSelfType,
                        ));
                    }

                    item.pop()
                        .ok_or_else(CompileError::unsupported_super(super_token))?;
                }
                other => {
                    return Err(CompileError::new(
                        other,
                        CompileErrorKind::ExpectedLeadingPathSegment,
                    ));
                }
            }
        }

        Ok(Named { local, item })
    }
}

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
    enum_id: Id,
    /// Ast for declaration.
    ast: ast::ItemVariant,
}

impl Variant {
    /// Construct a new variant.
    pub fn new(enum_id: Id, ast: ast::ItemVariant) -> Self {
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
    pub(crate) impl_item: Rc<Item>,
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
    /// The module item the constant is defined in.
    pub(crate) mod_item: Rc<QueryMod>,
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
    /// The query item this indexed entry belongs to.
    pub(crate) item: Rc<QueryItem>,
    /// The source location of the indexed entry.
    pub(crate) span: Span,
    /// The source of the indexed entry.
    pub(crate) source: Arc<Source>,
    /// The source id of the indexed entry.
    pub(crate) source_id: SourceId,
    /// The entry data.
    pub(crate) indexed: Indexed,
}

/// Query information for a path.
#[derive(Debug)]
pub(crate) struct QueryPath {
    pub(crate) mod_item: Rc<QueryMod>,
    pub(crate) impl_item: Option<Rc<Item>>,
    pub(crate) item: Item,
}

/// Item and the module that the item belongs to.
#[derive(Debug)]
pub(crate) struct QueryItem {
    pub(crate) id: Id,
    pub(crate) item: Item,
    pub(crate) mod_item: Rc<QueryMod>,
    pub(crate) visibility: Visibility,
}

/// An indexed constant function.
#[derive(Debug)]
pub(crate) struct QueryConstFn {
    /// The item of the const fn.
    pub(crate) item: Rc<QueryItem>,
    /// The compiled constant function.
    pub(crate) ir_fn: ir::IrFn,
}

/// Module, its item and its visibility.
#[derive(Debug)]
pub(crate) struct QueryMod {
    pub(crate) item: Item,
    pub(crate) visibility: Visibility,
}

/// The result of calling [Query::find_named].
#[derive(Debug)]
pub struct Named {
    /// If the resolved value is local.
    pub local: Option<Box<str>>,
    /// The path resolved to the given item.
    pub item: Item,
}

impl Named {
    /// Get the local identifier of this named.
    pub fn as_local(&self) -> Option<&str> {
        self.local.as_deref()
    }
}

impl fmt::Display for Named {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.item, f)
    }
}
