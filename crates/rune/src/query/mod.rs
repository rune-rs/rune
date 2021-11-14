//! Lazy query system, used to compile and build items on demand and keep track
//! of what's being used and not.

use crate::ast;
use crate::ast::{Span, Spanned};
use crate::collections::{HashMap, HashSet};
use crate::compile::{
    CompileError, CompileErrorKind, CompileVisitor, ComponentRef, ImportStep, IntoComponent, Item,
    Location, UnitBuilder, Visibility,
};
use crate::ir;
use crate::ir::{IrBudget, IrCompile, IrCompiler, IrInterpreter};
use crate::macros::Storage;
use crate::meta::{
    CompileItem, CompileMeta, CompileMetaCapture, CompileMetaEmpty, CompileMetaKind,
    CompileMetaStruct, CompileMetaTuple, CompileMod, CompileSource,
};
use crate::parse::{Id, Opaque, Resolve};
use crate::runtime::format;
use crate::runtime::{Call, Names};
use crate::shared::{Consts, Gen, Items};
use crate::{Context, Hash, SourceId, Sources};
use std::collections::VecDeque;
use std::fmt;
use std::num::NonZeroUsize;
use std::sync::Arc;

pub use self::query_error::{QueryError, QueryErrorKind};

mod query_error;

/// An internally resolved macro.
pub(crate) enum BuiltInMacro {
    Template(BuiltInTemplate),
    Format(Box<BuiltInFormat>),
    File(BuiltInFile),
    Line(BuiltInLine),
}

/// An internally resolved template.
pub(crate) struct BuiltInTemplate {
    /// The span of the built-in template.
    pub(crate) span: Span,
    /// Indicate if template originated from literal.
    pub(crate) from_literal: bool,
    /// Expressions being concatenated as a template.
    pub(crate) exprs: Vec<ast::Expr>,
}

/// An internal format specification.
pub(crate) struct BuiltInFormat {
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
pub(crate) struct BuiltInFile {
    /// The span of the built-in-file
    pub(crate) span: Span,
    /// Path value to use
    pub(crate) value: ast::LitStr,
}

/// Macro data for `line!()`
pub(crate) struct BuiltInLine {
    /// The span of the built-in-file
    pub(crate) span: Span,
    /// The line number
    pub(crate) value: ast::LitNumber,
}

#[derive(Default)]
pub(crate) struct QueryInner {
    /// Resolved meta about every single item during a compilation.
    meta: HashMap<Item, CompileMeta>,
    /// Build queue.
    queue: VecDeque<BuildEntry>,
    /// Indexed items that can be queried for, which will queue up for them to
    /// be compiled.
    indexed: HashMap<Item, Vec<IndexedEntry>>,
    /// Compiled constant functions.
    const_fns: HashMap<Id, Arc<QueryConstFn>>,
    /// Query paths.
    query_paths: HashMap<Id, Arc<QueryPath>>,
    /// The result of internally resolved macros.
    internal_macros: HashMap<Id, Arc<BuiltInMacro>>,
    /// Associated between `id` and `Item`. Use to look up items through
    /// `item_for` with an opaque id.
    ///
    /// These items are associated with AST elements, and encodoes the item path
    /// that the AST element was indexed.
    items: HashMap<Id, Arc<CompileItem>>,
    /// All available names in the context.
    names: Names,
    /// Modules and associated metadata.
    modules: HashMap<Item, Arc<CompileMod>>,
}

pub(crate) struct Query<'a> {
    /// The current unit being built.
    pub(crate) unit: &'a mut UnitBuilder,
    /// Cache of constants that have been expanded.
    pub(crate) consts: &'a mut Consts,
    /// Storage associated with the query.
    pub(crate) storage: &'a mut Storage,
    /// Sources available.
    pub(crate) sources: &'a mut Sources,
    /// Visitor for the compiler meta.
    pub(crate) visitor: &'a mut dyn CompileVisitor,
    /// Shared id generator.
    gen: &'a Gen,
    /// Inner state of the query engine.
    inner: &'a mut QueryInner,
}

impl<'a> Query<'a> {
    /// Construct a new compilation context.
    pub(crate) fn new(
        unit: &'a mut UnitBuilder,
        consts: &'a mut Consts,
        storage: &'a mut Storage,
        sources: &'a mut Sources,
        visitor: &'a mut dyn CompileVisitor,
        gen: &'a Gen,
        inner: &'a mut QueryInner,
    ) -> Self {
        Self {
            unit,
            consts,
            storage,
            sources,
            visitor,
            gen,
            inner,
        }
    }

    /// Reborrow the query engine from a reference to `self`.
    pub(crate) fn borrow(&mut self) -> Query<'_> {
        Query {
            unit: self.unit,
            consts: self.consts,
            storage: self.storage,
            sources: self.sources,
            visitor: self.visitor,
            gen: self.gen,
            inner: self.inner,
        }
    }

    /// Access reference to storage.
    pub(crate) fn storage(&self) -> &Storage {
        &self.storage
    }

    /// Insert the given compile meta.
    pub(crate) fn insert_meta(&mut self, span: Span, meta: CompileMeta) -> Result<(), QueryError> {
        let item = meta.item.item.clone();

        self.visitor.register_meta(&meta);

        if let Some(existing) = self.inner.meta.insert(item, meta.clone()) {
            return Err(QueryError::new(
                span,
                QueryErrorKind::MetaConflict {
                    current: meta,
                    existing,
                },
            ));
        }

        Ok(())
    }

    /// Get the next build entry from the build queue associated with the query
    /// engine.
    pub(crate) fn next_build_entry(&mut self) -> Option<BuildEntry> {
        self.inner.queue.pop_front()
    }

    /// Push a build entry.
    pub(crate) fn push_build_entry(&mut self, entry: BuildEntry) {
        self.inner.queue.push_back(entry)
    }

    /// Insert path information.
    pub(crate) fn insert_path(
        &mut self,
        module: &Arc<CompileMod>,
        impl_item: Option<&Arc<Item>>,
        item: &Item,
    ) -> Id {
        let query_path = Arc::new(QueryPath {
            module: module.clone(),
            impl_item: impl_item.cloned(),
            item: item.clone(),
        });

        let id = self.gen.next();
        self.inner.query_paths.insert(id, query_path);
        id
    }

    /// Remove a reference to the given path by id.
    pub(crate) fn remove_path_by_id(&mut self, id: Option<Id>) {
        if let Some(id) = id {
            self.inner.query_paths.remove(&id);
        }
    }

    /// Insert module and associated metadata.
    pub(crate) fn insert_mod(
        &mut self,
        items: &Items,
        source_id: SourceId,
        span: Span,
        parent: &Arc<CompileMod>,
        visibility: Visibility,
    ) -> Result<Arc<CompileMod>, QueryError> {
        let item = self.insert_new_item(items, source_id, span, parent, visibility)?;

        let query_mod = Arc::new(CompileMod {
            location: Location::new(source_id, span),
            item: item.item.clone(),
            visibility,
            parent: Some(parent.clone()),
        });

        self.inner
            .modules
            .insert(item.item.clone(), query_mod.clone());
        self.insert_name(&item.item);
        Ok(query_mod)
    }

    /// Insert module and associated metadata.
    pub(crate) fn insert_root_mod(
        &mut self,
        source_id: SourceId,
        spanned: Span,
    ) -> Result<Arc<CompileMod>, QueryError> {
        let query_mod = Arc::new(CompileMod {
            location: Location::new(source_id, spanned),
            item: Item::new(),
            visibility: Visibility::Public,
            parent: None,
        });

        self.inner.modules.insert(Item::new(), query_mod.clone());
        self.insert_name(&Item::new());
        Ok(query_mod)
    }

    /// Get the compile item for the given item.
    pub(crate) fn get_item(&self, span: Span, id: Id) -> Result<Arc<CompileItem>, QueryError> {
        if let Some(item) = self.inner.items.get(&id) {
            return Ok(item.clone());
        }

        Err(QueryError::new(span, QueryErrorKind::MissingRevId { id }))
    }

    /// Inserts an item that *has* to be unique, else cause an error.
    ///
    /// This are not indexed and does not generate an ID, they're only visible
    /// in reverse lookup.
    pub(crate) fn insert_new_item(
        &mut self,
        items: &Items,
        source_id: SourceId,
        spanned: Span,
        module: &Arc<CompileMod>,
        visibility: Visibility,
    ) -> Result<Arc<CompileItem>, QueryError> {
        let id = items.id();
        let item = &*items.item();

        self.insert_new_item_with(id, item, source_id, spanned, module, visibility)
    }

    /// Insert a new expanded internal macro.
    pub(crate) fn insert_new_builtin_macro(
        &mut self,
        internal_macro: BuiltInMacro,
    ) -> Result<Id, QueryError> {
        let id = self.gen.next();
        self.inner
            .internal_macros
            .insert(id, Arc::new(internal_macro));
        Ok(id)
    }

    /// Get the item for the given identifier.
    pub(crate) fn item_for<T>(&self, ast: T) -> Result<Arc<CompileItem>, QueryError>
    where
        T: Spanned + Opaque,
    {
        let item = ast
            .id()
            .and_then(|n| self.inner.items.get(&n))
            .ok_or_else(|| {
                QueryError::new(
                    ast.span(),
                    QueryErrorKind::MissingId {
                        what: "item",
                        id: ast.id(),
                    },
                )
            })?;

        Ok(item.clone())
    }

    pub(crate) fn builtin_macro_for<T>(&self, ast: T) -> Result<Arc<BuiltInMacro>, QueryError>
    where
        T: Spanned + Opaque,
    {
        let internal_macro = ast
            .id()
            .and_then(|n| self.inner.internal_macros.get(&n))
            .ok_or_else(|| {
                QueryError::new(
                    ast.span(),
                    QueryErrorKind::MissingId {
                        what: "builtin macro",
                        id: ast.id(),
                    },
                )
            })?;

        Ok(internal_macro.clone())
    }

    /// Get the constant function associated with the opaque.
    pub(crate) fn const_fn_for<T>(&self, ast: T) -> Result<Arc<QueryConstFn>, QueryError>
    where
        T: Spanned + Opaque,
    {
        let const_fn = ast
            .id()
            .and_then(|n| self.inner.const_fns.get(&n))
            .ok_or_else(|| {
                QueryError::new(
                    ast.span(),
                    QueryErrorKind::MissingId {
                        what: "constant function",
                        id: ast.id(),
                    },
                )
            })?;

        Ok(const_fn.clone())
    }

    /// Index the given entry. It is not allowed to overwrite other entries.
    pub(crate) fn index(&mut self, entry: IndexedEntry) {
        log::trace!("indexed: {}", entry.item.item);

        self.insert_name(&entry.item.item);
        self.inner
            .indexed
            .entry(entry.item.item.clone())
            .or_default()
            .push(entry);
    }

    /// Index a constant expression.
    pub(crate) fn index_const<T>(
        &mut self,
        item: &Arc<CompileItem>,
        expr: &T,
    ) -> Result<(), QueryError>
    where
        T: IrCompile<Output = ir::Ir>,
    {
        log::trace!("new const: {:?}", item.item);

        let mut ir_compiler = IrCompiler { q: self.borrow() };

        let ir = ir_compiler.compile(expr)?;

        self.index(IndexedEntry {
            item: item.clone(),
            indexed: Indexed::Const(Const {
                module: item.module.clone(),
                ir,
            }),
        });

        Ok(())
    }

    /// Index a constant function.
    pub(crate) fn index_const_fn(
        &mut self,
        item: &Arc<CompileItem>,
        item_fn: Box<ast::ItemFn>,
    ) -> Result<(), QueryError> {
        log::trace!("new const fn: {:?}", item.item);

        self.index(IndexedEntry {
            item: item.clone(),
            indexed: Indexed::ConstFn(ConstFn { item_fn }),
        });

        Ok(())
    }

    /// Add a new enum item.
    pub(crate) fn index_enum(&mut self, item: &Arc<CompileItem>) -> Result<(), QueryError> {
        log::trace!("new enum: {:?}", item.item);

        self.index(IndexedEntry {
            item: item.clone(),
            indexed: Indexed::Enum,
        });

        Ok(())
    }

    /// Add a new struct item that can be queried.
    pub(crate) fn index_struct(
        &mut self,
        item: &Arc<CompileItem>,
        ast: Box<ast::ItemStruct>,
    ) -> Result<(), QueryError> {
        log::trace!("new struct: {:?}", item.item);

        self.index(IndexedEntry {
            item: item.clone(),
            indexed: Indexed::Struct(Struct::new(ast)),
        });

        Ok(())
    }

    /// Add a new variant item that can be queried.
    pub(crate) fn index_variant(
        &mut self,
        item: &Arc<CompileItem>,
        enum_id: Id,
        ast: ast::ItemVariant,
    ) -> Result<(), QueryError> {
        log::trace!("new variant: {:?}", item.item);

        self.index(IndexedEntry {
            item: item.clone(),
            indexed: Indexed::Variant(Variant::new(enum_id, ast)),
        });

        Ok(())
    }

    /// Add a new function that can be queried for.
    pub(crate) fn index_closure(
        &mut self,
        item: &Arc<CompileItem>,
        ast: Box<ast::ExprClosure>,
        captures: Arc<[CompileMetaCapture]>,
        call: Call,
        do_move: bool,
    ) -> Result<(), QueryError> {
        log::trace!("new closure: {:?}", item.item);

        self.index(IndexedEntry {
            item: item.clone(),
            indexed: Indexed::Closure(Closure {
                ast,
                captures,
                call,
                do_move,
            }),
        });

        Ok(())
    }

    /// Add a new async block.
    pub(crate) fn index_async_block(
        &mut self,
        item: &Arc<CompileItem>,
        ast: ast::Block,
        captures: Arc<[CompileMetaCapture]>,
        call: Call,
        do_move: bool,
    ) -> Result<(), QueryError> {
        log::trace!("new closure: {:?}", item.item);

        self.index(IndexedEntry {
            item: item.clone(),
            indexed: Indexed::AsyncBlock(AsyncBlock {
                ast,
                captures,
                call,
                do_move,
            }),
        });

        Ok(())
    }

    /// Remove and queue up unused entries for building.
    ///
    /// Returns boolean indicating if any unused entries were queued up.
    pub(crate) fn queue_unused_entries(&mut self) -> Result<bool, (SourceId, QueryError)> {
        let unused = self
            .inner
            .indexed
            .values()
            .flat_map(|entries| entries.iter())
            .map(|e| e.item.clone())
            .collect::<Vec<_>>();

        if unused.is_empty() {
            return Ok(false);
        }

        for query_item in unused {
            // NB: recursive queries might remove from `indexed`, so we expect
            // to miss things here.
            if let Some(meta) = self
                .query_meta(query_item.location.span, &query_item.item, Used::Unused)
                .map_err(|e| (query_item.location.source_id, e))?
            {
                self.visitor.visit_meta(
                    query_item.location.source_id,
                    &meta,
                    query_item.location.span,
                );
            }
        }

        Ok(true)
    }

    /// Query for the given meta by looking up the reverse of the specified
    /// item.
    pub(crate) fn query_meta(
        &mut self,
        span: Span,
        item: &Item,
        used: Used,
    ) -> Result<Option<CompileMeta>, QueryError> {
        if let Some(meta) = self.inner.meta.get(item) {
            return Ok(Some(meta.clone()));
        }

        // See if there's an index entry we can construct and insert.
        let entry = match self.remove_indexed(span, item)? {
            Some(entry) => entry,
            None => return Ok(None),
        };

        let meta = self.build_indexed_entry(span, entry, used)?;
        self.unit.insert_meta(span, &meta)?;
        self.insert_meta(span, meta.clone())?;
        Ok(Some(meta))
    }

    /// Perform a path lookup on the current state of the unit.
    pub(crate) fn convert_path(
        &mut self,
        context: &Context,
        path: &ast::Path,
    ) -> Result<Named, CompileError> {
        let id = path.id();

        let qp = id
            .and_then(|id| self.inner.query_paths.get(&id))
            .ok_or_else(|| QueryError::new(path, QueryErrorKind::MissingId { what: "path", id }))?
            .clone();

        let mut in_self_type = false;
        let mut local = None;

        let mut item = match (&path.global, &path.first) {
            (Some(..), ast::PathSegment::Ident(ident)) => {
                let ident = ident.resolve(&self.storage, self.sources)?;
                Item::with_crate(ident.as_ref())
            }
            (Some(global), _) => {
                return Err(CompileError::new(
                    global.span(),
                    CompileErrorKind::UnsupportedGlobal,
                ));
            }
            (None, segment) => match segment {
                ast::PathSegment::Ident(ident) => {
                    if path.rest.is_empty() {
                        local = Some(*ident);
                    }

                    self.convert_initial_path(context, &qp.module, &qp.item, ident)?
                }
                ast::PathSegment::Super(super_value) => {
                    let mut item = qp.module.item.clone();

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
                ast::PathSegment::SelfValue(..) => qp.module.item.clone(),
                ast::PathSegment::Crate(..) => Item::new(),
                ast::PathSegment::Generics(arguments) => {
                    return Err(CompileError::new(
                        arguments,
                        CompileErrorKind::UnsupportedGenerics,
                    ));
                }
            },
        };

        for (_, segment) in &path.rest {
            log::trace!("item = {}", item);

            match segment {
                ast::PathSegment::Ident(ident) => {
                    let ident = ident.resolve(&self.storage, self.sources)?;
                    item.push(ident);
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
                ast::PathSegment::Generics(arguments) => {
                    return Err(CompileError::new(
                        arguments,
                        CompileErrorKind::UnsupportedGenerics,
                    ));
                }
                other => {
                    return Err(CompileError::new(
                        other,
                        CompileErrorKind::ExpectedLeadingPathSegment,
                    ));
                }
            }
        }

        let span = path.span();

        let local = match local {
            Some(local) => Some(local.resolve(&self.storage, self.sources)?.into()),
            None => None,
        };

        if let Some(new) = self.import(span, &qp.module, &item, Used::Used)? {
            return Ok(Named { local, item: new });
        }

        Ok(Named { local, item })
    }

    /// Declare a new import.
    pub(crate) fn insert_import(
        &mut self,
        source_id: SourceId,
        span: Span,
        module: &Arc<CompileMod>,
        visibility: Visibility,
        at: Item,
        target: Item,
        alias: Option<ast::Ident>,
        wildcard: bool,
    ) -> Result<(), QueryError> {
        let alias = match alias {
            Some(alias) => Some(alias.resolve(&self.storage, self.sources)?),
            None => None,
        };

        let last = alias
            .as_ref()
            .map(IntoComponent::as_component_ref)
            .or_else(|| target.last())
            .ok_or_else(|| QueryError::new(span, QueryErrorKind::LastUseComponent))?;

        let item = at.extended(last);
        let location = Location::new(source_id, span);

        let entry = ImportEntry {
            location,
            target: target.clone(),
            module: module.clone(),
        };

        let id = self.gen.next();
        let item = self.insert_new_item_with(id, &item, source_id, span, module, visibility)?;

        // toplevel public uses are re-exported.
        if item.is_public() {
            self.inner.queue.push_back(BuildEntry {
                location,
                item: item.clone(),
                build: Build::ReExport,
                used: Used::Used,
            });
        }

        self.index(IndexedEntry {
            item,
            indexed: Indexed::Import(Import { wildcard, entry }),
        });

        Ok(())
    }

    /// Check if unit contains the given name by prefix.
    pub(crate) fn contains_prefix(&self, item: &Item) -> bool {
        self.inner.names.contains_prefix(item)
    }

    /// Iterate over known child components of the given name.
    pub(crate) fn iter_components<'it, I: 'it>(
        &'it self,
        iter: I,
    ) -> impl Iterator<Item = ComponentRef<'it>> + 'it
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        self.inner.names.iter_components(iter)
    }

    /// Get the given import by name.
    pub(crate) fn import(
        &mut self,
        span: Span,
        module: &Arc<CompileMod>,
        item: &Item,
        used: Used,
    ) -> Result<Option<Item>, QueryError> {
        let mut visited = HashSet::<Item>::new();
        let mut path = Vec::new();
        let mut module = module.clone();
        let mut item = item.clone();
        let mut any_matched = false;

        'outer: loop {
            let mut cur = Item::new();
            let mut it = item.iter();

            while let Some(c) = it.next() {
                cur.push(c);

                let update = self.import_step(span, &module, &cur, used, &mut path)?;

                let update = match update {
                    Some(update) => update,
                    None => continue,
                };

                path.push(ImportStep {
                    location: update.location,
                    item: update.target.clone(),
                });

                if !visited.insert(item.clone()) {
                    return Err(QueryError::new(span, QueryErrorKind::ImportCycle { path }));
                }

                module = update.module;
                item = update.target.join(it);
                any_matched = true;
                continue 'outer;
            }

            break;
        }

        if any_matched {
            return Ok(Some(item));
        }

        Ok(None)
    }

    /// Inner import implementation that doesn't walk the imported name.
    fn import_step(
        &mut self,
        span: Span,
        module: &Arc<CompileMod>,
        item: &Item,
        used: Used,
        path: &mut Vec<ImportStep>,
    ) -> Result<Option<QueryImportStep>, QueryError> {
        // already resolved query.
        if let Some(meta) = self.inner.meta.get(item) {
            return Ok(match &meta.kind {
                CompileMetaKind::Import {
                    module,
                    location,
                    target,
                } => Some(QueryImportStep {
                    module: module.clone(),
                    location: *location,
                    target: target.clone(),
                }),
                _ => None,
            });
        }

        // resolve query.
        let entry = match self.remove_indexed(span, item)? {
            Some(entry) => entry,
            _ => return Ok(None),
        };

        self.check_access_to(
            span,
            &*module,
            item,
            &entry.item.module,
            entry.item.location,
            entry.item.visibility,
            path,
        )?;

        let import = match entry.indexed {
            Indexed::Import(import) => import.entry,
            indexed => {
                self.import_indexed(span, entry.item, indexed, used)?;
                return Ok(None);
            }
        };

        let meta = CompileMeta {
            item: entry.item.clone(),
            kind: CompileMetaKind::Import {
                module: import.module.clone(),
                location: import.location,
                target: import.target.clone(),
            },
            source: None,
        };

        self.insert_meta(span, meta)?;

        Ok(Some(QueryImportStep {
            module: import.module,
            location: import.location,
            target: import.target,
        }))
    }

    /// Build a single, indexed entry and return its metadata.
    fn build_indexed_entry(
        &mut self,
        span: Span,
        entry: IndexedEntry,
        used: Used,
    ) -> Result<CompileMeta, QueryError> {
        let IndexedEntry {
            item: query_item,
            indexed,
        } = entry;

        let kind = match indexed {
            Indexed::Enum => CompileMetaKind::Enum {
                type_hash: Hash::type_hash(&query_item.item),
            },
            Indexed::Variant(variant) => {
                let enum_item = self.item_for((query_item.location.span, Some(variant.enum_id)))?;

                // Assert that everything is built for the enum.
                self.query_meta(span, &enum_item.item, Default::default())?;

                variant_into_item_decl(
                    &query_item.item,
                    variant.ast.body,
                    Some(&enum_item.item),
                    &self.storage,
                    self.sources,
                )?
            }
            Indexed::Struct(st) => struct_into_item_decl(
                &query_item.item,
                st.ast.body,
                None,
                &self.storage,
                self.sources,
            )?,
            Indexed::Function(f) => {
                self.inner.queue.push_back(BuildEntry {
                    location: query_item.location,
                    item: query_item.clone(),
                    build: Build::Function(f),
                    used,
                });

                CompileMetaKind::Function {
                    type_hash: Hash::type_hash(&query_item.item),
                    is_test: false,
                    is_bench: false,
                }
            }
            Indexed::Closure(c) => {
                let captures = c.captures.clone();
                let do_move = c.do_move;

                self.inner.queue.push_back(BuildEntry {
                    location: query_item.location,
                    item: query_item.clone(),
                    build: Build::Closure(c),
                    used,
                });

                CompileMetaKind::Closure {
                    type_hash: Hash::type_hash(&query_item.item),
                    captures,
                    do_move,
                }
            }
            Indexed::AsyncBlock(b) => {
                let captures = b.captures.clone();
                let do_move = b.do_move;

                self.inner.queue.push_back(BuildEntry {
                    location: query_item.location,
                    item: query_item.clone(),
                    build: Build::AsyncBlock(b),
                    used,
                });

                CompileMetaKind::AsyncBlock {
                    type_hash: Hash::type_hash(&query_item.item),
                    captures,
                    do_move,
                }
            }
            Indexed::Const(c) => {
                let mut const_compiler = IrInterpreter {
                    budget: IrBudget::new(1_000_000),
                    scopes: Default::default(),
                    module: c.module.clone(),
                    item: query_item.item.clone(),
                    q: self.borrow(),
                };

                let const_value = const_compiler.eval_const(&c.ir, used)?;

                if used.is_unused() {
                    self.inner.queue.push_back(BuildEntry {
                        location: query_item.location,
                        item: query_item.clone(),
                        build: Build::Unused,
                        used,
                    });
                }

                CompileMetaKind::Const { const_value }
            }
            Indexed::ConstFn(c) => {
                let mut ir_compiler = IrCompiler { q: self.borrow() };

                let ir_fn = ir_compiler.compile(&*c.item_fn)?;

                let id = self.insert_const_fn(&query_item, ir_fn);

                if used.is_unused() {
                    self.inner.queue.push_back(BuildEntry {
                        location: query_item.location,
                        item: query_item.clone(),
                        build: Build::Unused,
                        used,
                    });
                }

                CompileMetaKind::ConstFn { id, is_test: false }
            }
            Indexed::Import(import) => {
                let module = import.entry.module.clone();
                let location = import.entry.location;
                let target = import.entry.target.clone();

                if !import.wildcard {
                    self.inner.queue.push_back(BuildEntry {
                        location: query_item.location,
                        item: query_item.clone(),
                        build: Build::Import(import),
                        used,
                    });
                }

                CompileMetaKind::Import {
                    module,
                    location,
                    target,
                }
            }
        };

        let source = CompileSource {
            location: query_item.location,
            path: self
                .sources
                .path(query_item.location.source_id)
                .map(Into::into),
        };

        Ok(CompileMeta {
            item: query_item,
            kind,
            source: Some(source),
        })
    }

    /// Insert the given name into the unit.
    fn insert_name(&mut self, item: &Item) {
        self.inner.names.insert(item);
    }

    fn insert_new_item_with(
        &mut self,
        id: Id,
        item: &Item,
        source_id: SourceId,
        spanned: Span,
        module: &Arc<CompileMod>,
        visibility: Visibility,
    ) -> Result<Arc<CompileItem>, QueryError> {
        let query_item = Arc::new(CompileItem {
            location: Location::new(source_id, spanned),
            id,
            item: item.clone(),
            module: module.clone(),
            visibility,
        });

        self.inner.items.insert(id, query_item.clone());
        Ok(query_item)
    }

    /// Handle an imported indexed entry.
    fn import_indexed(
        &mut self,
        span: Span,
        item: Arc<CompileItem>,
        indexed: Indexed,
        used: Used,
    ) -> Result<(), QueryError> {
        // NB: if we find another indexed entry, queue it up for
        // building and clone its built meta to the other
        // results.
        let entry = IndexedEntry { item, indexed };

        let meta = self.build_indexed_entry(span, entry, used)?;
        self.unit.insert_meta(span, &meta)?;
        self.insert_meta(span, meta)?;
        Ok(())
    }

    /// Remove the indexed entry corresponding to the given item..
    fn remove_indexed(
        &mut self,
        span: Span,
        item: &Item,
    ) -> Result<Option<IndexedEntry>, QueryError> {
        // See if there's an index entry we can construct and insert.
        let entries = match self.inner.indexed.remove(item) {
            Some(entries) => entries,
            None => return Ok(None),
        };

        let mut it = entries.into_iter().peekable();

        let mut cur = match it.next() {
            Some(first) => first,
            None => return Ok(None),
        };

        if it.peek().is_none() {
            return Ok(Some(cur));
        }

        let mut locations = vec![(cur.item.location, cur.item().clone())];

        while let Some(oth) = it.next() {
            locations.push((oth.item.location, oth.item().clone()));

            if let (Indexed::Import(a), Indexed::Import(b)) = (&cur.indexed, &oth.indexed) {
                if a.wildcard {
                    cur = oth;
                    continue;
                }

                if b.wildcard {
                    continue;
                }
            }

            for oth in it {
                locations.push((oth.item.location, oth.item().clone()));
            }

            return Err(QueryError::new(
                span,
                QueryErrorKind::AmbiguousItem {
                    item: cur.item.item.clone(),
                    locations,
                },
            ));
        }

        if let Indexed::Import(Import { wildcard: true, .. }) = &cur.indexed {
            return Err(QueryError::new(
                span,
                QueryErrorKind::AmbiguousItem {
                    item: cur.item.item.clone(),
                    locations,
                },
            ));
        }

        Ok(Some(cur))
    }

    /// Walk the names to find the first one that is contained in the unit.
    fn convert_initial_path(
        &mut self,
        context: &Context,
        module: &Arc<CompileMod>,
        base: &Item,
        local: &ast::Ident,
    ) -> Result<Item, CompileError> {
        debug_assert!(base.starts_with(&module.item));
        let mut base = base.clone();

        let local = local.resolve(&self.storage, self.sources)?;

        while base.starts_with(&module.item) {
            base.push(local);

            if self.inner.names.contains(&base) {
                return Ok(base);
            }

            let c = base.pop();
            debug_assert!(c.is_some());

            if base.pop().is_none() {
                break;
            }
        }

        if let Some(item) = self.unit.prelude().get(local) {
            return Ok(item.clone());
        }

        if context.contains_crate(local) {
            return Ok(Item::with_crate(local));
        }

        Ok(module.item.extended(local))
    }

    /// Insert an item and return its Id.
    fn insert_const_fn(&mut self, item: &Arc<CompileItem>, ir_fn: ir::IrFn) -> Id {
        let id = self.gen.next();

        self.inner.const_fns.insert(
            id,
            Arc::new(QueryConstFn {
                item: item.clone(),
                ir_fn,
            }),
        );

        id
    }

    /// Check that the given item is accessible from the given module.
    fn check_access_to(
        &self,
        span: Span,
        from: &CompileMod,
        item: &Item,
        module: &CompileMod,
        location: Location,
        visibility: Visibility,
        chain: &mut Vec<ImportStep>,
    ) -> Result<(), QueryError> {
        let (common, tree) = from.item.ancestry(&module.item);
        let mut current_module = common.clone();

        // Check each module from the common ancestrly to the module.
        for c in &tree {
            current_module.push(c);

            let m = self.inner.modules.get(&current_module).ok_or_else(|| {
                QueryError::new(
                    span,
                    QueryErrorKind::MissingMod {
                        item: current_module.clone(),
                    },
                )
            })?;

            if !m.visibility.is_visible(&common, &current_module) {
                return Err(QueryError::new(
                    span,
                    QueryErrorKind::NotVisibleMod {
                        chain: into_chain(std::mem::take(chain)),
                        location: m.location,
                        visibility: m.visibility,
                        item: current_module,
                        from: from.item.clone(),
                    },
                ));
            }
        }

        if !visibility.is_visible_inside(&common, &module.item) {
            return Err(QueryError::new(
                span,
                QueryErrorKind::NotVisible {
                    chain: into_chain(std::mem::take(chain)),
                    location,
                    visibility,
                    item: item.clone(),
                    from: from.item.clone(),
                },
            ));
        }

        Ok(())
    }
}

/// Indication whether a value is being evaluated because it's being used or not.
#[derive(Debug, Clone, Copy)]
pub enum Used {
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

#[derive(Debug, Clone)]
pub(crate) enum Indexed {
    Enum,
    Struct(Struct),
    Variant(Variant),
    Function(Function),
    Closure(Closure),
    AsyncBlock(AsyncBlock),
    Const(Const),
    ConstFn(ConstFn),
    Import(Import),
}

#[derive(Debug, Clone)]
pub(crate) struct Import {
    /// The import entry.
    pub(crate) entry: ImportEntry,
    /// Indicates if the import is a wildcard or not.
    ///
    /// Wildcard imports do not cause unused warnings.
    pub(crate) wildcard: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct Struct {
    /// The ast of the struct.
    ast: Box<ast::ItemStruct>,
}

impl Struct {
    /// Construct a new struct entry.
    pub(crate) fn new(ast: Box<ast::ItemStruct>) -> Self {
        Self { ast }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Variant {
    /// Id of of the enum type.
    enum_id: Id,
    /// Ast for declaration.
    ast: ast::ItemVariant,
}

impl Variant {
    /// Construct a new variant.
    pub(crate) fn new(enum_id: Id, ast: ast::ItemVariant) -> Self {
        Self { enum_id, ast }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Function {
    /// Ast for declaration.
    pub(crate) ast: Box<ast::ItemFn>,
    pub(crate) call: Call,
}

#[derive(Debug, Clone)]
pub(crate) struct InstanceFunction {
    /// Ast for the instance function.
    pub(crate) ast: Box<ast::ItemFn>,
    /// The item of the instance function.
    pub(crate) impl_item: Arc<Item>,
    /// The span of the instance function.
    pub(crate) instance_span: Span,
    /// Calling convention of the instance function.
    pub(crate) call: Call,
}

#[derive(Debug, Clone)]
pub(crate) struct Closure {
    /// Ast for closure.
    pub(crate) ast: Box<ast::ExprClosure>,
    /// Captures.
    pub(crate) captures: Arc<[CompileMetaCapture]>,
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
    pub(crate) captures: Arc<[CompileMetaCapture]>,
    /// Calling convention used for async block.
    pub(crate) call: Call,
    /// If the block moves its captures.
    pub(crate) do_move: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct Const {
    /// The module item the constant is defined in.
    pub(crate) module: Arc<CompileMod>,
    /// The intermediate representation of the constant expression.
    pub(crate) ir: ir::Ir,
}

#[derive(Debug, Clone)]
pub(crate) struct ConstFn {
    /// The const fn ast.
    pub(crate) item_fn: Box<ast::ItemFn>,
}

/// An entry in the build queue.
#[derive(Debug, Clone)]
pub(crate) enum Build {
    Function(Function),
    InstanceFunction(InstanceFunction),
    Closure(Closure),
    AsyncBlock(AsyncBlock),
    Unused,
    Import(Import),
    /// A public re-export.
    ReExport,
}

/// An entry in the build queue.
#[derive(Debug, Clone)]
pub(crate) struct BuildEntry {
    /// The location of the build entry.
    pub(crate) location: Location,
    /// The item of the build entry.
    pub(crate) item: Arc<CompileItem>,
    /// The build entry.
    pub(crate) build: Build,
    /// If the queued up entry was unused or not.
    pub(crate) used: Used,
}

#[derive(Debug, Clone)]
pub(crate) struct IndexedEntry {
    /// The query item this indexed entry belongs to.
    pub(crate) item: Arc<CompileItem>,
    /// The entry data.
    pub(crate) indexed: Indexed,
}

impl IndexedEntry {
    /// The item that best describes this indexed entry.
    pub(crate) fn item(&self) -> &Item {
        match &self.indexed {
            Indexed::Import(Import { entry, .. }) => &entry.target,
            _ => &self.item.item,
        }
    }
}

/// Query information for a path.
#[derive(Debug)]
pub(crate) struct QueryPath {
    pub(crate) module: Arc<CompileMod>,
    pub(crate) impl_item: Option<Arc<Item>>,
    pub(crate) item: Item,
}

/// An indexed constant function.
#[derive(Debug)]
pub(crate) struct QueryConstFn {
    /// The item of the const fn.
    pub(crate) item: Arc<CompileItem>,
    /// The compiled constant function.
    pub(crate) ir_fn: ir::IrFn,
}

/// The result of calling [Query::convert_path].
#[derive(Debug)]
pub(crate) struct Named {
    /// If the resolved value is local.
    pub local: Option<Box<str>>,
    /// The path resolved to the given item.
    pub item: Item,
}

impl Named {
    /// Get the local identifier of this named.
    pub(crate) fn as_local(&self) -> Option<&str> {
        self.local.as_deref()
    }
}

impl fmt::Display for Named {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.item, f)
    }
}

/// Construct metadata for an empty body.
fn unit_body_meta(item: &Item, enum_item: Option<&Item>) -> CompileMetaKind {
    let type_hash = Hash::type_hash(item);

    let empty = CompileMetaEmpty {
        hash: Hash::type_hash(item),
    };

    match enum_item {
        Some(enum_item) => CompileMetaKind::UnitVariant {
            type_hash,
            enum_item: enum_item.clone(),
            empty,
        },
        None => CompileMetaKind::UnitStruct { type_hash, empty },
    }
}

/// Construct metadata for an empty body.
fn tuple_body_meta(
    item: &Item,
    enum_item: Option<&Item>,
    tuple: ast::Parenthesized<ast::Field, T![,]>,
) -> CompileMetaKind {
    let type_hash = Hash::type_hash(item);

    let tuple = CompileMetaTuple {
        args: tuple.len(),
        hash: Hash::type_hash(item),
    };

    match enum_item {
        Some(enum_item) => CompileMetaKind::TupleVariant {
            type_hash,
            enum_item: enum_item.clone(),
            tuple,
        },
        None => CompileMetaKind::TupleStruct { type_hash, tuple },
    }
}

/// Construct metadata for a struct body.
fn struct_body_meta(
    item: &Item,
    enum_item: Option<&Item>,
    storage: &Storage,
    sources: &Sources,
    st: ast::Braced<ast::Field, T![,]>,
) -> Result<CompileMetaKind, QueryError> {
    let type_hash = Hash::type_hash(item);

    let mut fields = HashSet::new();

    for (ast::Field { name, .. }, _) in st {
        let name = name.resolve(storage, sources)?;
        fields.insert(name.into());
    }

    let object = CompileMetaStruct { fields };

    Ok(match enum_item {
        Some(enum_item) => CompileMetaKind::StructVariant {
            type_hash,
            enum_item: enum_item.clone(),
            object,
        },
        None => CompileMetaKind::Struct { type_hash, object },
    })
}

/// Convert an ast declaration into a struct.
fn variant_into_item_decl(
    item: &Item,
    body: ast::ItemVariantBody,
    enum_item: Option<&Item>,
    storage: &Storage,
    sources: &Sources,
) -> Result<CompileMetaKind, QueryError> {
    Ok(match body {
        ast::ItemVariantBody::UnitBody => unit_body_meta(item, enum_item),
        ast::ItemVariantBody::TupleBody(tuple) => tuple_body_meta(item, enum_item, tuple),
        ast::ItemVariantBody::StructBody(st) => {
            struct_body_meta(item, enum_item, storage, sources, st)?
        }
    })
}

/// Convert an ast declaration into a struct.
fn struct_into_item_decl(
    item: &Item,
    body: ast::ItemStructBody,
    enum_item: Option<&Item>,
    storage: &Storage,
    sources: &Sources,
) -> Result<CompileMetaKind, QueryError> {
    Ok(match body {
        ast::ItemStructBody::UnitBody => unit_body_meta(item, enum_item),
        ast::ItemStructBody::TupleBody(tuple) => tuple_body_meta(item, enum_item, tuple),
        ast::ItemStructBody::StructBody(st) => {
            struct_body_meta(item, enum_item, storage, sources, st)?
        }
    })
}

/// An imported entry.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub(crate) struct ImportEntry {
    /// The location of the import.
    pub(crate) location: Location,
    /// The item being imported.
    pub(crate) target: Item,
    /// The module in which the imports is located.
    pub(crate) module: Arc<CompileMod>,
}

struct QueryImportStep {
    module: Arc<CompileMod>,
    location: Location,
    target: Item,
}

fn into_chain(chain: Vec<ImportStep>) -> Vec<Location> {
    chain.into_iter().map(|c| c.location).collect()
}
