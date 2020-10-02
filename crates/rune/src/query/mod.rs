//! Lazy query system, used to compile and build items on demand.

use crate::ast;
use crate::collections::{HashMap, HashSet};
use crate::indexing::Visibility;
use crate::ir::ir;
use crate::ir::{IrBudget, IrInterpreter};
use crate::ir::{IrCompile as _, IrCompiler};
use crate::parsing::Opaque;
use crate::shared::{Consts, Location};
use crate::{
    CompileError, CompileErrorKind, CompileVisitor, Id, Resolve as _, Spanned, Storage, UnitBuilder,
};
use runestick::{
    Call, CompileMeta, CompileMetaCapture, CompileMetaEmpty, CompileMetaKind, CompileMetaStruct,
    CompileMetaTuple, CompileSource, ComponentRef, Hash, IntoComponent, Item, Names, Source,
    SourceId, Span, Type,
};
use std::collections::VecDeque;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

mod imports;
mod query_error;

pub use self::query_error::{QueryError, QueryErrorKind};

use self::imports::{ImportEntry, NameKind};

pub(crate) struct Query {
    /// Next opaque id generated.
    next_id: Id,
    imports: imports::Imports,
    pub(crate) storage: Storage,
    /// Unit being built.
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
            imports: imports::Imports {
                prelude: unit.prelude(),
                imports: Default::default(),
                names: Names::default(),
                items: HashMap::new(),
                modules: HashMap::new(),
                items_rev: HashMap::new(),
            },
            storage,
            unit,
            consts,
            queue: VecDeque::new(),
            indexed: HashMap::new(),
            templates: HashMap::new(),
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
    ) -> Id {
        let query_path = Rc::new(QueryPath {
            mod_item: mod_item.clone(),
            impl_item: impl_item.cloned().clone(),
            item: item.clone(),
        });

        let id = self.next_id.next().expect("ran out of ids");
        self.query_paths.insert(id, query_path);
        id
    }

    /// Remove a reference to the given path by id.
    pub(crate) fn remove_path_by_id(&mut self, id: Option<Id>) {
        if let Some(id) = id {
            self.query_paths.remove(&id);
        }
    }

    /// Insert module and associated metadata.
    pub(crate) fn insert_mod(
        &mut self,
        source_id: SourceId,
        spanned: Span,
        item: &Item,
        visibility: Visibility,
    ) -> Result<(Id, Rc<QueryMod>), QueryError> {
        let id = self.next_id.next().expect("ran out of ids");

        let query_mod = Rc::new(QueryMod {
            location: Location::new(source_id, spanned),
            item: item.clone(),
            visibility,
        });

        self.imports.modules.insert(item.clone(), query_mod.clone());

        self.insert_name(source_id, spanned, item)?;
        self.insert_item(source_id, spanned, item, &query_mod, visibility)?;
        Ok((id, query_mod))
    }

    /// Insert an item and return its Id.
    pub(crate) fn insert_item(
        &mut self,
        source_id: SourceId,
        spanned: Span,
        item: &Item,
        mod_item: &Rc<QueryMod>,
        visibility: Visibility,
    ) -> Result<(Id, Rc<QueryItem>), QueryError> {
        if let Some(existing) = self.imports.items_rev.get(item) {
            return Ok((existing.id, existing.clone()));
        }

        let id = self.next_id.next().expect("ran out of ids");

        let query_item = Rc::new(QueryItem {
            location: Location::new(source_id, spanned),
            id,
            item: item.clone(),
            mod_item: mod_item.clone(),
            visibility,
        });

        if let Some(old) = self
            .imports
            .items_rev
            .insert(item.clone(), query_item.clone())
        {
            return Err(QueryError::new(
                spanned,
                QueryErrorKind::ItemConflict {
                    item: item.clone(),
                    other: old.location,
                },
            ));
        }

        self.imports.items.insert(id, query_item.clone());
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

    /// Get the item for the given identifier.
    pub(crate) fn item_for<T>(&self, ast: T) -> Result<&Rc<QueryItem>, QueryError>
    where
        T: Spanned + Opaque,
    {
        let id = ast.id();

        let item = id
            .and_then(|n| self.imports.items.get(&n))
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
        source_id: SourceId,
        item: &Rc<QueryItem>,
        source: Arc<Source>,
        item_const: ast::ItemConst,
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
                location: Location::new(source_id, item_const.span()),
                source,
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
        source_id: SourceId,
        item: &Rc<QueryItem>,
        source: Arc<Source>,
        item_fn: ast::ItemFn,
    ) -> Result<(), QueryError> {
        log::trace!("new const fn: {:?}", item.item);

        self.index(
            &item.item,
            IndexedEntry {
                item: item.clone(),
                location: Location::new(source_id, item_fn.span()),
                source,
                indexed: Indexed::ConstFn(ConstFn { item_fn }),
            },
        )?;

        Ok(())
    }

    /// Add a new enum item.
    pub fn index_enum(
        &mut self,
        source_id: SourceId,
        span: Span,
        item: &Rc<QueryItem>,
        source: Arc<Source>,
    ) -> Result<(), QueryError> {
        log::trace!("new enum: {:?}", item.item);

        self.index(
            &item.item,
            IndexedEntry {
                item: item.clone(),
                location: Location::new(source_id, span),
                source,
                indexed: Indexed::Enum,
            },
        )?;

        Ok(())
    }

    /// Add a new struct item that can be queried.
    pub fn index_struct(
        &mut self,
        source_id: SourceId,
        item: &Rc<QueryItem>,
        ast: ast::ItemStruct,
        source: Arc<Source>,
    ) -> Result<(), QueryError> {
        log::trace!("new struct: {:?}", item);

        self.index(
            &item.item,
            IndexedEntry {
                item: item.clone(),
                location: Location::new(source_id, ast.span()),
                source,
                indexed: Indexed::Struct(Struct::new(ast)),
            },
        )?;

        Ok(())
    }

    /// Add a new variant item that can be queried.
    pub fn index_variant(
        &mut self,
        source_id: SourceId,
        item: &Rc<QueryItem>,
        enum_id: Id,
        ast: ast::ItemVariant,
        source: Arc<Source>,
    ) -> Result<(), QueryError> {
        log::trace!("new variant: {:?}", item);

        self.index(
            &item.item,
            IndexedEntry {
                item: item.clone(),
                location: Location::new(source_id, ast.span()),
                source,
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
                location: Location::new(source_id, ast.span()),
                source,
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
                location: Location::new(source_id, ast.span()),
                source,
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
        self.insert_name(entry.location.source_id, entry.location.span, item)?;
        let span = entry.location.span;

        if let Some(old) = self.indexed.insert(item.clone(), entry) {
            return Err(QueryError::new(
                span,
                QueryErrorKind::ItemConflict {
                    item: item.clone(),
                    other: old.location,
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
            .map(|e| (e.item.clone(), e.location))
            .collect::<Vec<_>>();

        if unused.is_empty() {
            return Ok(false);
        }

        for (item, location) in unused {
            // NB: recursive queries might remove from `indexed`, so we expect
            // to miss things here.
            if let Some(meta) = self
                .query_meta_with(location.span, &*item, Used::Unused)
                .map_err(|e| (location.source_id, e))?
            {
                visitor.visit_meta(location.source_id, &meta, location.span);
            }
        }

        Ok(true)
    }

    /// Public query meta which marks things as used.
    pub(crate) fn query_meta(
        &mut self,
        spanned: Span,
        item: &Item,
        used: Used,
    ) -> Result<Option<CompileMeta>, QueryError> {
        let item = match self.imports.items_rev.get(item) {
            Some(item) => item.clone(),
            None => return Ok(None),
        };

        self.query_meta_with(spanned, &*item, used)
    }

    /// Query the exact meta item without performing a reverse lookup for it.
    pub(crate) fn query_meta_with(
        &mut self,
        spanned: Span,
        item: &QueryItem,
        used: Used,
    ) -> Result<Option<CompileMeta>, QueryError> {
        if let Some(meta) = self.unit.lookup_meta(&item.item) {
            return Ok(Some(meta));
        }

        // See if there's an index entry we can construct and insert.
        let entry = match self.indexed.remove(&item.item) {
            Some(entry) => entry,
            None => return Ok(None),
        };

        let meta = self.build_indexed_entry(spanned, item, entry, used)?;

        self.unit
            .insert_meta(meta.clone())
            .map_err(|error| QueryError::new(spanned, error))?;

        Ok(Some(meta))
    }

    /// Build a single, indexed entry and return its metadata.
    fn build_indexed_entry(
        &mut self,
        spanned: Span,
        item: &QueryItem,
        entry: IndexedEntry,
        used: Used,
    ) -> Result<CompileMeta, QueryError> {
        let IndexedEntry {
            location,
            indexed,
            source,
            item: entry_item,
        } = entry;

        let path = source.path().map(ToOwned::to_owned);

        let kind = match indexed {
            Indexed::Enum => CompileMetaKind::Enum {
                type_of: Type::from(Hash::type_hash(&item.item)),
            },
            Indexed::Variant(variant) => {
                let enum_item = self.item_for((location.span, variant.enum_id))?.clone();
                // Assert that everything is built for the enum.
                self.query_meta_with(spanned, &enum_item, Default::default())?;
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
                    location: Location::new(location.source_id, f.ast.span()),
                    item: item.item.clone(),
                    build: Build::Function(f),
                    source,
                    used,
                });

                CompileMetaKind::Function {
                    type_of: Type::from(Hash::type_hash(&item.item)),
                }
            }
            Indexed::Closure(c) => {
                let captures = c.captures.clone();

                self.queue.push_back(BuildEntry {
                    location: Location::new(location.source_id, c.ast.span()),
                    item: item.item.clone(),
                    build: Build::Closure(c),
                    source,
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
                    location: Location::new(location.source_id, b.ast.span()),
                    item: item.item.clone(),
                    build: Build::AsyncBlock(b),
                    source,
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
                        location,
                        item: item.item.clone(),
                        build: Build::UnusedConst(c),
                        source,
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
                        location: Location::new(location.source_id, c.item_fn.span()),
                        item: item.item.clone(),
                        build: Build::UnusedConstFn(c),
                        source,
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
                source_id: location.source_id,
                span: location.span,
                path,
            }),
        })
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
        path: &ast::Path,
        storage: &Storage,
        source: &Source,
    ) -> Result<Named, CompileError> {
        let id = path.id();

        // NB: this rather awkward looking section is to permit the borrow
        // checker to understand that we're borrowing distinct subfields of
        // `Query`.
        let qp = match id {
            Some(id) => self.query_paths.get(&id),
            None => None,
        };

        let qp = qp
            .ok_or_else(|| QueryError::new(path, QueryErrorKind::MissingId { what: "path", id }))?;

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
                    self.imports
                        .walk_names(path.span(), &qp.mod_item, &qp.item, ident.as_ref())?
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
                let mut item = qp.mod_item.item.clone();

                item.pop()
                    .ok_or_else(CompileError::unsupported_super(super_value))?;

                item
            }
            ast::PathSegment::SelfType(self_type) => {
                let impl_item = qp.impl_item.as_deref().ok_or_else(|| {
                    CompileError::new(self_type, CompileErrorKind::UnsupportedSelfType)
                })?;

                in_self_type = true;
                impl_item.clone()
            }
            ast::PathSegment::SelfValue(..) => qp.mod_item.item.clone(),
            ast::PathSegment::Crate(..) => Item::new(),
        };

        for (_, segment) in &path.rest {
            log::trace!("item = {}", item);

            match segment {
                ast::PathSegment::Ident(ident) => {
                    let ident = ident.resolve(storage, source)?;
                    item.push(ident.as_ref());

                    if let Some(new) = self.imports.get_import(&qp.mod_item, path.span(), &item)? {
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

    /// Insert the given name into the unit.
    pub(crate) fn insert_name(
        &mut self,
        source_id: SourceId,
        spanned: Span,
        item: &Item,
    ) -> Result<(), QueryError> {
        if let Some((_, other)) = self
            .imports
            .names
            .insert(item, (NameKind::Other, Location::new(source_id, spanned)))
        {
            return Err(QueryError::new(
                spanned,
                QueryErrorKind::ItemConflict {
                    item: item.clone(),
                    other,
                },
            ));
        }

        Ok(())
    }

    /// Declare a new import.
    pub(crate) fn insert_import(
        &mut self,
        source_id: SourceId,
        spanned: Span,
        mod_item: &Rc<QueryMod>,
        visibility: Visibility,
        at: Item,
        path: Item,
        alias: Option<&str>,
        wildcard: bool,
    ) -> Result<(), QueryError> {
        let last = alias
            .as_ref()
            .map(IntoComponent::as_component_ref)
            .or_else(|| path.last())
            .ok_or_else(|| QueryError::new(spanned, QueryErrorKind::LastUseComponent))?;

        let item = at.extended(last);

        // NB: wildcard expansions do not overwite local names.
        if wildcard && self.imports.names.contains(&item) {
            return Ok(());
        }

        let entry = Rc::new(ImportEntry {
            location: Location::new(source_id, spanned.span()),
            visibility,
            name: item.clone(),
            imported: path.clone(),
            mod_item: mod_item.clone(),
        });

        if let Some(entry) = self.imports.imports.insert(item.clone(), entry) {
            return Err(QueryError::new(
                spanned,
                QueryErrorKind::ImportConflict {
                    item,
                    other: entry.location,
                },
            ));
        }

        self.insert_item(source_id, spanned, &item, mod_item, visibility)?;

        if let Some((_, other)) = self.imports.names.insert(
            item.clone(),
            (NameKind::Use, Location::new(source_id, spanned)),
        ) {
            return Err(QueryError::new(
                spanned,
                QueryErrorKind::ItemConflict {
                    item: item.clone(),
                    other,
                },
            ));
        }

        Ok(())
    }

    /// Iterate over all imports.
    pub(crate) fn imports<'a>(&'a self) -> impl Iterator<Item = (&'a Item, &'a Rc<ImportEntry>)> {
        self.imports.imports.iter()
    }

    /// Check if unit contains the given name by prefix.
    pub(crate) fn contains_prefix(&self, item: &Item) -> bool {
        self.imports.names.contains_prefix(item)
    }

    /// Iterate over known child components of the given name.
    pub(crate) fn iter_components<'a, I: 'a>(
        &'a self,
        iter: I,
    ) -> impl Iterator<Item = ComponentRef<'a>> + 'a
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        self.imports.names.iter_components(iter)
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
    /// The location of the build entry.
    pub(crate) location: Location,
    /// The item of the build entry.
    pub(crate) item: Item,
    /// The build entry.
    pub(crate) build: Build,
    /// The source of the build entry.
    pub(crate) source: Arc<Source>,
    /// If the queued up entry was unused or not.
    pub(crate) used: Used,
}

pub(crate) struct IndexedEntry {
    /// The query item this indexed entry belongs to.
    pub(crate) item: Rc<QueryItem>,
    /// The source location of the indexed entry.
    pub(crate) location: Location,
    /// The source of the indexed entry.
    pub(crate) source: Arc<Source>,
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
    pub(crate) location: Location,
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
    pub(crate) location: Location,
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
