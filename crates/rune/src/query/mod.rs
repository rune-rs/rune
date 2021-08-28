//! Lazy query system, used to compile and build items on demand.

use crate::ast;
use crate::collections::{HashMap, HashSet};
use crate::ir;
use crate::ir::{IrBudget, IrCompile, IrCompiler, IrInterpreter, IrQuery};
use crate::parsing::Opaque;
use crate::shared::{Consts, Gen, Items};
use crate::{
    CompileError, CompileErrorKind, CompileVisitor, Id, ImportEntryStep, NoopCompileVisitor,
    Resolve as _, Spanned, Storage, UnitBuilder,
};
use runestick::format;
use runestick::{
    Call, CompileItem, CompileMeta, CompileMetaCapture, CompileMetaEmpty, CompileMetaKind,
    CompileMetaStruct, CompileMetaTuple, CompileMod, CompileSource, Component, ComponentRef,
    Context, Hash, IntoComponent, Item, Location, Names, Source, SourceId, Span, Visibility,
};
use std::cell::{RefCell, RefMut};
use std::collections::VecDeque;
use std::fmt;
use std::num::NonZeroUsize;
use std::rc::Rc;
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
pub struct BuiltInFile {
    /// The span of the built-in-file
    pub(crate) span: Span,
    /// Path value to use
    pub(crate) value: ast::LitStr,
}

/// Macro data for `line!()`
pub struct BuiltInLine {
    /// The span of the built-in-file
    pub(crate) span: Span,
    /// The line number
    pub(crate) value: ast::LitNumber,
}

impl IrQuery for QueryInner {
    fn query_meta(
        &mut self,
        span: Span,
        item: &Item,
        used: Used,
    ) -> Result<Option<CompileMeta>, QueryError> {
        QueryInner::query_meta(self, span, item, used)
    }

    fn builtin_macro_for(
        &self,
        span: Span,
        id: Option<Id>,
    ) -> Result<Arc<BuiltInMacro>, QueryError> {
        QueryInner::builtin_macro_for(self, span, id)
    }

    fn const_fn_for(&self, span: Span, id: Option<Id>) -> Result<Arc<QueryConstFn>, QueryError> {
        QueryInner::const_fn_for(self, span, id)
    }
}

#[derive(Clone, Default)]
pub(crate) struct Query {
    inner: Rc<RefCell<QueryInner>>,
}

impl Query {
    /// Construct a new compilation context.
    pub fn new(
        visitor: Rc<dyn CompileVisitor>,
        storage: Storage,
        unit: UnitBuilder,
        consts: Consts,
        gen: Gen,
    ) -> Self {
        Self {
            inner: Rc::new(RefCell::new(QueryInner {
                visitor,
                meta: HashMap::new(),
                storage,
                prelude: unit.prelude(),
                unit,
                consts,
                gen,
                queue: VecDeque::new(),
                indexed: HashMap::new(),
                const_fns: HashMap::new(),
                query_paths: HashMap::new(),
                internal_macros: HashMap::new(),
                items: HashMap::new(),
                names: Names::default(),
                modules: HashMap::new(),
            })),
        }
    }

    /// Acquire mutable access and coerce into a `&mut dyn IrQuery`, suitable
    /// for use with ir interpreter/compiler etc...
    pub(crate) fn as_ir_query(&self) -> RefMut<'_, dyn IrQuery> {
        let inner = self.inner.borrow_mut();
        RefMut::map(inner, |inner| inner)
    }

    /// Insert the given compile meta.
    pub(crate) fn insert_meta(&self, spanned: Span, meta: CompileMeta) -> Result<(), QueryError> {
        let mut inner = self.inner.borrow_mut();

        inner
            .unit
            .insert_meta(&meta)
            .map_err(|error| QueryError::new(spanned, error))?;

        inner.insert_meta(spanned, meta)?;
        Ok(())
    }

    /// Get the next build entry from the build queue associated with the query
    /// engine.
    pub(crate) fn next_build_entry(&self) -> Option<BuildEntry> {
        self.inner.borrow_mut().queue.pop_front()
    }

    /// Push a build entry.
    pub(crate) fn push_build_entry(&self, entry: BuildEntry) {
        self.inner.borrow_mut().queue.push_back(entry)
    }

    /// Access a clone of the storage associated with query.
    pub(crate) fn storage(&self) -> Storage {
        self.inner.borrow().storage.clone()
    }

    /// Insert path information.
    pub(crate) fn insert_path(
        &self,
        module: &Arc<CompileMod>,
        impl_item: Option<&Arc<Item>>,
        item: &Item,
    ) -> Id {
        let mut inner = self.inner.borrow_mut();

        let query_path = Arc::new(QueryPath {
            module: module.clone(),
            impl_item: impl_item.cloned(),
            item: item.clone(),
        });

        let id = inner.gen.next();
        inner.query_paths.insert(id, query_path);
        id
    }

    /// Remove a reference to the given path by id.
    pub(crate) fn remove_path_by_id(&self, id: Option<Id>) {
        let mut inner = self.inner.borrow_mut();

        if let Some(id) = id {
            inner.query_paths.remove(&id);
        }
    }

    /// Insert module and associated metadata.
    pub(crate) fn insert_mod(
        &self,
        items: &Items,
        source_id: SourceId,
        span: Span,
        parent: &Arc<CompileMod>,
        visibility: Visibility,
    ) -> Result<Arc<CompileMod>, QueryError> {
        let mut inner = self.inner.borrow_mut();

        let item = inner.insert_new_item(items, source_id, span, parent, visibility)?;

        let query_mod = Arc::new(CompileMod {
            location: Location::new(source_id, span),
            item: item.item.clone(),
            visibility,
            parent: Some(parent.clone()),
        });

        inner.modules.insert(item.item.clone(), query_mod.clone());
        inner.insert_name(&item.item);
        Ok(query_mod)
    }

    /// Insert module and associated metadata.
    pub(crate) fn insert_root_mod(
        &self,
        source_id: SourceId,
        spanned: Span,
    ) -> Result<Arc<CompileMod>, QueryError> {
        let mut inner = self.inner.borrow_mut();

        let query_mod = Arc::new(CompileMod {
            location: Location::new(source_id, spanned),
            item: Item::new(),
            visibility: Visibility::Public,
            parent: None,
        });

        inner.modules.insert(Item::new(), query_mod.clone());
        inner.insert_name(&Item::new());
        Ok(query_mod)
    }

    /// Get the compile item for the given item.
    pub(crate) fn get_item(&self, span: Span, id: Id) -> Result<Arc<CompileItem>, QueryError> {
        let inner = self.inner.borrow();

        if let Some(item) = inner.items.get(&id) {
            return Ok(item.clone());
        }

        Err(QueryError::new(span, QueryErrorKind::MissingRevId { id }))
    }

    /// Insert a new item with the given parameters.
    pub(crate) fn insert_new_item(
        &self,
        items: &Items,
        source_id: SourceId,
        spanned: Span,
        module: &Arc<CompileMod>,
        visibility: Visibility,
    ) -> Result<Arc<CompileItem>, QueryError> {
        self.inner
            .borrow_mut()
            .insert_new_item(items, source_id, spanned, module, visibility)
    }

    /// Insert a new expanded internal macro.
    pub(crate) fn insert_new_builtin_macro(
        &mut self,
        internal_macro: BuiltInMacro,
    ) -> Result<Id, QueryError> {
        self.inner
            .borrow_mut()
            .insert_new_builtin_macro(internal_macro)
    }

    /// Get the item for the given identifier.
    pub(crate) fn item_for<T>(&self, ast: T) -> Result<Arc<CompileItem>, QueryError>
    where
        T: Spanned + Opaque,
    {
        self.inner.borrow().item_for(ast.span(), ast.id())
    }

    /// Get the expanded internal macro for the given identifier.
    pub(crate) fn builtin_macro_for<T>(&self, ast: T) -> Result<Arc<BuiltInMacro>, QueryError>
    where
        T: Spanned + Opaque,
    {
        self.inner.borrow().builtin_macro_for(ast.span(), ast.id())
    }

    /// Get the constant function associated with the opaque.
    pub(crate) fn const_fn_for<T>(&self, ast: T) -> Result<Arc<QueryConstFn>, QueryError>
    where
        T: Spanned + Opaque,
    {
        self.inner.borrow().const_fn_for(ast.span(), ast.id())
    }

    /// Index the given entry. It is not allowed to overwrite other entries.
    pub fn index(&self, entry: IndexedEntry) {
        self.inner.borrow_mut().index(entry);
    }

    /// Index a constant expression.
    pub fn index_const<T>(
        &self,
        item: &Arc<CompileItem>,
        source: &Arc<Source>,
        expr: &T,
    ) -> Result<(), QueryError>
    where
        T: IrCompile<Output = ir::Ir>,
    {
        log::trace!("new const: {:?}", item.item);

        let mut inner = self.inner.borrow_mut();

        let mut ir_compiler = IrCompiler {
            storage: inner.storage.clone(),
            source: source.clone(),
            query: &mut *inner,
        };

        let ir = ir_compiler.compile(expr)?;

        inner.index(IndexedEntry {
            item: item.clone(),
            source: source.clone(),
            indexed: Indexed::Const(Const {
                module: item.module.clone(),
                ir,
            }),
        });

        Ok(())
    }

    /// Index a constant function.
    pub fn index_const_fn(
        &self,
        item: &Arc<CompileItem>,
        source: &Arc<Source>,
        item_fn: Box<ast::ItemFn>,
    ) -> Result<(), QueryError> {
        log::trace!("new const fn: {:?}", item.item);

        self.inner.borrow_mut().index(IndexedEntry {
            item: item.clone(),
            source: source.clone(),
            indexed: Indexed::ConstFn(ConstFn { item_fn }),
        });

        Ok(())
    }

    /// Add a new enum item.
    pub fn index_enum(
        &self,
        item: &Arc<CompileItem>,
        source: &Arc<Source>,
    ) -> Result<(), QueryError> {
        log::trace!("new enum: {:?}", item.item);

        self.inner.borrow_mut().index(IndexedEntry {
            item: item.clone(),
            source: source.clone(),
            indexed: Indexed::Enum,
        });

        Ok(())
    }

    /// Add a new struct item that can be queried.
    pub fn index_struct(
        &self,
        item: &Arc<CompileItem>,
        source: &Arc<Source>,
        ast: Box<ast::ItemStruct>,
    ) -> Result<(), QueryError> {
        log::trace!("new struct: {:?}", item.item);

        self.inner.borrow_mut().index(IndexedEntry {
            item: item.clone(),
            source: source.clone(),
            indexed: Indexed::Struct(Struct::new(ast)),
        });

        Ok(())
    }

    /// Add a new variant item that can be queried.
    pub fn index_variant(
        &self,
        item: &Arc<CompileItem>,
        source: &Arc<Source>,
        enum_id: Id,
        ast: ast::ItemVariant,
    ) -> Result<(), QueryError> {
        log::trace!("new variant: {:?}", item.item);

        self.inner.borrow_mut().index(IndexedEntry {
            item: item.clone(),
            source: source.clone(),
            indexed: Indexed::Variant(Variant::new(enum_id, ast)),
        });

        Ok(())
    }

    /// Add a new function that can be queried for.
    pub fn index_closure(
        &self,
        item: &Arc<CompileItem>,
        source: &Arc<Source>,
        ast: Box<ast::ExprClosure>,
        captures: Arc<[CompileMetaCapture]>,
        call: Call,
        do_move: bool,
    ) -> Result<(), QueryError> {
        log::trace!("new closure: {:?}", item.item);

        self.inner.borrow_mut().index(IndexedEntry {
            item: item.clone(),
            source: source.clone(),
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
    pub fn index_async_block(
        &self,
        item: &Arc<CompileItem>,
        source: &Arc<Source>,
        ast: ast::Block,
        captures: Arc<[CompileMetaCapture]>,
        call: Call,
        do_move: bool,
    ) -> Result<(), QueryError> {
        log::trace!("new closure: {:?}", item.item);

        self.inner.borrow_mut().index(IndexedEntry {
            item: item.clone(),
            source: source.clone(),
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
    pub(crate) fn queue_unused_entries(&self) -> Result<bool, (SourceId, QueryError)> {
        let mut inner = self.inner.borrow_mut();

        let unused = inner
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
            if let Some(meta) = inner
                .query_meta(query_item.location.span, &query_item.item, Used::Unused)
                .map_err(|e| (query_item.location.source_id, e))?
            {
                inner.visitor.visit_meta(
                    query_item.location.source_id,
                    &meta,
                    query_item.location.span,
                );
            }
        }

        Ok(true)
    }

    /// Perform a meta query with a plain item that will be looked up in the
    /// items reverse map to identify.
    pub(crate) fn query_meta(
        &self,
        span: Span,
        item: &Item,
        used: Used,
    ) -> Result<Option<CompileMeta>, QueryError> {
        self.inner.borrow_mut().query_meta(span, item, used)
    }

    /// Convert the given path.
    pub(crate) fn convert_path(
        &self,
        context: &Context,
        storage: &Storage,
        source: &Source,
        path: &ast::Path,
    ) -> Result<Named, CompileError> {
        self.inner
            .borrow_mut()
            .convert_path(context, storage, source, path)
    }

    /// Declare a new import.
    pub(crate) fn insert_import(
        &self,
        source_id: SourceId,
        span: Span,
        source: &Arc<Source>,
        module: &Arc<CompileMod>,
        visibility: Visibility,
        at: Item,
        target: Item,
        alias: Option<&str>,
        wildcard: bool,
    ) -> Result<(), QueryError> {
        let mut inner = self.inner.borrow_mut();

        let last = alias
            .as_ref()
            .map(IntoComponent::as_component_ref)
            .or_else(|| target.last())
            .ok_or_else(|| QueryError::new(span, QueryErrorKind::LastUseComponent))?;

        let item = at.extended(last);
        let location = Location::new(source_id, span);

        let entry = ImportEntry {
            location,
            visibility,
            target: target.clone(),
            module: module.clone(),
        };

        let id = inner.gen.next();
        let item = inner.insert_new_item_with(id, &item, source_id, span, module, visibility)?;

        // toplevel public uses are re-exported.
        if item.is_public() {
            inner.queue.push_back(BuildEntry {
                location,
                item: item.clone(),
                build: Build::ReExport,
                source: source.clone(),
                used: Used::Used,
            });
        }

        inner.index(IndexedEntry {
            item,
            source: source.clone(),
            indexed: Indexed::Import(Import { wildcard, entry }),
        });

        Ok(())
    }

    /// Check if unit contains the given name by prefix.
    pub(crate) fn contains_prefix(&self, item: &Item) -> bool {
        self.inner.borrow().names.contains_prefix(item)
    }

    /// Iterate over known child components of the given name.
    pub(crate) fn iter_components<I>(&self, iter: I) -> Vec<Component>
    where
        I: IntoIterator,
        I::Item: IntoComponent,
    {
        let inner = self.inner.borrow();

        inner
            .names
            .iter_components(iter)
            .map(ComponentRef::into_component)
            .collect::<Vec<_>>()
    }

    pub(crate) fn import(
        &self,
        span: Span,
        module: &Arc<CompileMod>,
        item: &Item,
        used: Used,
    ) -> Result<Option<Item>, QueryError> {
        let mut inner = self.inner.borrow_mut();
        inner.import(span, module, item, used)
    }
}

#[derive(Clone)]
struct QueryInner {
    /// Visitor for the compiler meta.
    visitor: Rc<dyn CompileVisitor>,
    /// Resolved meta about every single item during a compilation.
    meta: HashMap<Item, CompileMeta>,
    /// Macro storage.
    storage: Storage,
    /// Prelude from the prelude.
    prelude: HashMap<Box<str>, Item>,
    /// Unit being built.
    unit: UnitBuilder,
    /// Cache of constants that have been expanded.
    consts: Consts,
    /// Shared id generator.
    gen: Gen,
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

impl Default for QueryInner {
    fn default() -> Self {
        Self {
            visitor: Rc::new(NoopCompileVisitor::new()),
            meta: Default::default(),
            storage: Default::default(),
            prelude: Default::default(),
            unit: Default::default(),
            consts: Default::default(),
            gen: Default::default(),
            queue: Default::default(),
            indexed: Default::default(),
            const_fns: Default::default(),
            query_paths: Default::default(),
            internal_macros: Default::default(),
            items: Default::default(),
            names: Default::default(),
            modules: Default::default(),
        }
    }
}

impl QueryInner {
    /// Get the item for the given identifier.
    fn item_for(&self, span: Span, id: Option<Id>) -> Result<Arc<CompileItem>, QueryError> {
        let item = id
            .and_then(|n| self.items.get(&n))
            .ok_or_else(|| QueryError::new(span, QueryErrorKind::MissingId { what: "item", id }))?;

        Ok(item.clone())
    }

    /// Get the internally resolved macro for the specified id.
    fn builtin_macro_for(
        &self,
        span: Span,
        id: Option<Id>,
    ) -> Result<Arc<BuiltInMacro>, QueryError> {
        let internal_macro = id
            .and_then(|n| self.internal_macros.get(&n))
            .ok_or_else(|| {
                QueryError::new(
                    span,
                    QueryErrorKind::MissingId {
                        what: "builtin macro",
                        id,
                    },
                )
            })?;

        Ok(internal_macro.clone())
    }

    /// Get the constant function associated with the opaque.
    fn const_fn_for(&self, spanned: Span, id: Option<Id>) -> Result<Arc<QueryConstFn>, QueryError> {
        let const_fn = id.and_then(|n| self.const_fns.get(&n)).ok_or_else(|| {
            QueryError::new(
                spanned,
                QueryErrorKind::MissingId {
                    what: "constant function",
                    id,
                },
            )
        })?;

        Ok(const_fn.clone())
    }

    /// Insert the given name into the unit.
    fn insert_name(&mut self, item: &Item) {
        self.names.insert(item);
    }

    /// Inserts an item that *has* to be unique, else cause an error.
    ///
    /// This are not indexed and does not generate an ID, they're only visible
    /// in reverse lookup.
    fn insert_new_item(
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

        self.items.insert(id, query_item.clone());
        Ok(query_item)
    }

    /// Insert a new expanded internal macro.
    pub(crate) fn insert_new_builtin_macro(
        &mut self,
        internal_macro: BuiltInMacro,
    ) -> Result<Id, QueryError> {
        let id = self.gen.next();
        self.internal_macros.insert(id, Arc::new(internal_macro));
        Ok(id)
    }

    /// Internal implementation for indexing an entry.
    fn index(&mut self, entry: IndexedEntry) {
        log::trace!("indexed: {}", entry.item.item);

        self.insert_name(&entry.item.item);
        self.indexed
            .entry(entry.item.item.clone())
            .or_default()
            .push(entry);
    }

    /// Handle an imported indexed entry.
    fn import_indexed(
        &mut self,
        span: Span,
        item: Arc<CompileItem>,
        source: Arc<Source>,
        indexed: Indexed,
        used: Used,
    ) -> Result<(), QueryError> {
        // NB: if we find another indexed entry, queue it up for
        // building and clone its built meta to the other
        // results.
        let entry = IndexedEntry {
            item,
            source,
            indexed,
        };

        let meta = self.build_indexed_entry(span, entry, used)?;

        self.unit
            .insert_meta(&meta)
            .map_err(|error| QueryError::new(span, error))?;

        self.insert_meta(span, meta)?;
        Ok(())
    }

    /// Get the given import by name.
    fn import(
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

                path.push(ImportEntryStep {
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
        path: &mut Vec<ImportEntryStep>,
    ) -> Result<Option<ImportStep>, QueryError> {
        // already resolved query.
        if let Some(meta) = self.meta.get(item) {
            return Ok(match &meta.kind {
                CompileMetaKind::Import {
                    module,
                    location,
                    target,
                } => Some(ImportStep {
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
                self.import_indexed(span, entry.item, entry.source, indexed, used)?;
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

        Ok(Some(ImportStep {
            module: import.module,
            location: import.location,
            target: import.target,
        }))
    }

    /// Query for the given meta by looking up the reverse of the specified
    /// item.
    fn query_meta(
        &mut self,
        span: Span,
        item: &Item,
        used: Used,
    ) -> Result<Option<CompileMeta>, QueryError> {
        if let Some(meta) = self.meta.get(item) {
            return Ok(Some(meta.clone()));
        }

        // See if there's an index entry we can construct and insert.
        let entry = match self.remove_indexed(span, item)? {
            Some(entry) => entry,
            None => return Ok(None),
        };

        let meta = self.build_indexed_entry(span, entry, used)?;

        self.unit
            .insert_meta(&meta)
            .map_err(|error| QueryError::new(span, error))?;

        self.insert_meta(span, meta.clone())?;
        Ok(Some(meta))
    }

    /// Remove the indexed entry corresponding to the given item..
    fn remove_indexed(
        &mut self,
        span: Span,
        item: &Item,
    ) -> Result<Option<IndexedEntry>, QueryError> {
        // See if there's an index entry we can construct and insert.
        let entries = match self.indexed.remove(item) {
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

    /// Insert meta without registering peripherals under the assumption that it
    /// already has been registered.
    fn insert_meta(&mut self, span: Span, meta: CompileMeta) -> Result<(), QueryError> {
        let item = meta.item.item.clone();

        self.visitor.register_meta(&meta);

        if let Some(existing) = self.meta.insert(item, meta.clone()) {
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

    /// Walk the names to find the first one that is contained in the unit.
    fn lookup_initial(
        &mut self,
        context: &Context,
        module: &Arc<CompileMod>,
        base: &Item,
        local: &str,
    ) -> Result<Item, CompileError> {
        debug_assert!(base.starts_with(&module.item));
        let mut base = base.clone();

        while base.starts_with(&module.item) {
            let item = base.extended(local);

            if self.names.contains(&item) {
                return Ok(item);
            }

            if base.pop().is_none() {
                break;
            }
        }

        if let Some(item) = self.prelude.get(local) {
            return Ok(item.clone());
        }

        if context.contains_crate(local) {
            return Ok(Item::with_crate(local));
        }

        Ok(module.item.extended(local))
    }

    /// Perform a path lookup on the current state of the unit.
    fn convert_path(
        &mut self,
        context: &Context,
        storage: &Storage,
        source: &Source,
        path: &ast::Path,
    ) -> Result<Named, CompileError> {
        let id = path.id();

        let qp = id
            .and_then(|id| self.query_paths.get(&id))
            .ok_or_else(|| QueryError::new(path, QueryErrorKind::MissingId { what: "path", id }))?
            .clone();

        let mut in_self_type = false;
        let mut local = None;

        let mut item = match (&path.global, &path.first) {
            (Some(..), ast::PathSegment::Ident(ident)) => {
                let ident = ident.resolve(storage, source)?;
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
                    let ident = ident.resolve(storage, source)?;

                    if path.rest.is_empty() {
                        local = Some(<Box<str>>::from(ident.as_ref()));
                    }

                    self.lookup_initial(context, &qp.module, &qp.item, &*ident)?
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
                    let ident = ident.resolve(storage, source)?;
                    item.push(ident.as_ref());
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

        if let Some(new) = self.import(span, &qp.module, &item, Used::Used)? {
            return Ok(Named { local, item: new });
        }

        Ok(Named { local, item })
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
            source,
        } = entry;

        let path = source.path().map(ToOwned::to_owned);

        let kind = match indexed {
            Indexed::Enum => CompileMetaKind::Enum {
                type_hash: Hash::type_hash(&query_item.item),
            },
            Indexed::Variant(variant) => {
                let enum_item = self.item_for(query_item.location.span, Some(variant.enum_id))?;

                // Assert that everything is built for the enum.
                self.query_meta(span, &enum_item.item, Default::default())?;

                variant_into_item_decl(
                    &query_item.item,
                    variant.ast.body,
                    Some(&enum_item.item),
                    &self.storage,
                    &*source,
                )?
            }
            Indexed::Struct(st) => {
                struct_into_item_decl(&query_item.item, st.ast.body, None, &self.storage, &*source)?
            }
            Indexed::Function(f) => {
                self.queue.push_back(BuildEntry {
                    location: query_item.location,
                    item: query_item.clone(),
                    build: Build::Function(f),
                    source,
                    used,
                });

                CompileMetaKind::Function {
                    type_hash: Hash::type_hash(&query_item.item),
                    is_test: false,
                }
            }
            Indexed::Closure(c) => {
                let captures = c.captures.clone();
                let do_move = c.do_move;

                self.queue.push_back(BuildEntry {
                    location: query_item.location,
                    item: query_item.clone(),
                    build: Build::Closure(c),
                    source,
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

                self.queue.push_back(BuildEntry {
                    location: query_item.location,
                    item: query_item.clone(),
                    build: Build::AsyncBlock(b),
                    source,
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
                    consts: self.consts.clone(),
                    query: self,
                };

                let const_value = const_compiler.eval_const(&c.ir, used)?;

                if used.is_unused() {
                    self.queue.push_back(BuildEntry {
                        location: query_item.location,
                        item: query_item.clone(),
                        build: Build::Unused,
                        source,
                        used,
                    });
                }

                CompileMetaKind::Const { const_value }
            }
            Indexed::ConstFn(c) => {
                let mut ir_compiler = IrCompiler {
                    storage: self.storage.clone(),
                    source: source.clone(),
                    query: self,
                };

                let ir_fn = ir_compiler.compile(&*c.item_fn)?;

                let id = self.insert_const_fn(&query_item, ir_fn);

                if used.is_unused() {
                    self.queue.push_back(BuildEntry {
                        location: query_item.location,
                        item: query_item.clone(),
                        build: Build::Unused,
                        source,
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
                    self.queue.push_back(BuildEntry {
                        location: query_item.location,
                        item: query_item.clone(),
                        build: Build::Import(import),
                        source,
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
            source_id: query_item.location.source_id,
            span: query_item.location.span,
            path,
        };

        Ok(CompileMeta {
            item: query_item,
            kind,
            source: Some(source),
        })
    }

    /// Insert an item and return its Id.
    fn insert_const_fn(&mut self, item: &Arc<CompileItem>, ir_fn: ir::IrFn) -> Id {
        let id = self.gen.next();

        self.const_fns.insert(
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
        chain: &mut Vec<ImportEntryStep>,
    ) -> Result<(), QueryError> {
        let (common, tree) = from.item.ancestry(&module.item);
        let mut current_module = common.clone();

        // Check each module from the common ancestrly to the module.
        for c in &tree {
            current_module.push(c);

            let m = self.modules.get(&current_module).ok_or_else(|| {
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
pub struct Import {
    /// The import entry.
    pub(crate) entry: ImportEntry,
    /// Indicates if the import is a wildcard or not.
    ///
    /// Wildcard imports do not cause unused warnings.
    pub(crate) wildcard: bool,
}

#[derive(Debug, Clone)]
pub struct Struct {
    /// The ast of the struct.
    ast: Box<ast::ItemStruct>,
}

impl Struct {
    /// Construct a new struct entry.
    pub fn new(ast: Box<ast::ItemStruct>) -> Self {
        Self { ast }
    }
}

#[derive(Debug, Clone)]
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
    /// The source of the build entry.
    pub(crate) source: Arc<Source>,
    /// If the queued up entry was unused or not.
    pub(crate) used: Used,
}

#[derive(Debug, Clone)]
pub(crate) struct IndexedEntry {
    /// The query item this indexed entry belongs to.
    pub(crate) item: Arc<CompileItem>,
    /// The source of the indexed entry.
    pub(crate) source: Arc<Source>,
    /// The entry data.
    pub(crate) indexed: Indexed,
}

impl IndexedEntry {
    /// The item that best describes this indexed entry.
    pub fn item(&self) -> &Item {
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
    source: &Source,
    st: ast::Braced<ast::Field, T![,]>,
) -> Result<CompileMetaKind, QueryError> {
    let type_hash = Hash::type_hash(item);

    let mut fields = HashSet::new();

    for (ast::Field { name, .. }, _) in st {
        let name = name.resolve(storage, &*source)?;
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
    source: &Source,
) -> Result<CompileMetaKind, QueryError> {
    Ok(match body {
        ast::ItemVariantBody::UnitBody => unit_body_meta(item, enum_item),
        ast::ItemVariantBody::TupleBody(tuple) => tuple_body_meta(item, enum_item, tuple),
        ast::ItemVariantBody::StructBody(st) => {
            struct_body_meta(item, enum_item, storage, source, st)?
        }
    })
}

/// Convert an ast declaration into a struct.
fn struct_into_item_decl(
    item: &Item,
    body: ast::ItemStructBody,
    enum_item: Option<&Item>,
    storage: &Storage,
    source: &Source,
) -> Result<CompileMetaKind, QueryError> {
    Ok(match body {
        ast::ItemStructBody::UnitBody => unit_body_meta(item, enum_item),
        ast::ItemStructBody::TupleBody(tuple) => tuple_body_meta(item, enum_item, tuple),
        ast::ItemStructBody::StructBody(st) => {
            struct_body_meta(item, enum_item, storage, source, st)?
        }
    })
}

/// An imported entry.
#[derive(Debug, Clone)]
pub struct ImportEntry {
    /// The location of the import.
    pub location: Location,
    /// The visibility of the import.
    pub visibility: Visibility,
    /// The item being imported.
    pub target: Item,
    /// The module in which the imports is located.
    pub(crate) module: Arc<CompileMod>,
}

struct ImportStep {
    module: Arc<CompileMod>,
    location: Location,
    target: Item,
}

fn into_chain(chain: Vec<ImportEntryStep>) -> Vec<Location> {
    chain.into_iter().map(|c| c.location).collect()
}
