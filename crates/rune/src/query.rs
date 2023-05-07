//! Lazy query system, used to compile and build items on demand and keep track
//! of what's being used and not.

use core::fmt;
use core::mem::take;
use core::num::NonZeroUsize;

use crate::no_std::prelude::*;
use crate::no_std::sync::Arc;

use crate::ast;
use crate::ast::{Span, Spanned};
use crate::collections::{hash_map, BTreeMap, HashMap, HashSet, VecDeque};
use crate::compile::context;
use crate::compile::ir;
use crate::compile::meta;
use crate::compile::{
    self, CompileErrorKind, CompileVisitor, ComponentRef, Doc, ImportStep, IntoComponent, IrBudget,
    IrCompiler, IrInterpreter, Item, ItemBuf, ItemId, ItemMeta, Location, ModId, ModMeta, Names,
    Pool, Prelude, QueryErrorKind, SourceMeta, UnitBuilder, Visibility,
};
use crate::hir;
use crate::macros::Storage;
use crate::parse::{Id, NonZeroId, Opaque, Resolve, ResolveContext};
use crate::runtime::format;
use crate::runtime::Call;
use crate::shared::{Consts, Gen, Items};
use crate::{Context, Hash, SourceId, Sources};

/// The permitted number of import recursions when constructing a path.
const IMPORT_RECURSION_LIMIT: usize = 128;

/// An internally resolved macro.
#[allow(clippy::large_enum_variant)]
pub(crate) enum BuiltInMacro {
    Template(BuiltInTemplate),
    Format(BuiltInFormat),
    File(BuiltInFile),
    Line(BuiltInLine),
}

/// An internally resolved template.
#[derive(Spanned)]
pub(crate) struct BuiltInTemplate {
    /// The span of the built-in template.
    #[rune(span)]
    pub(crate) span: Span,
    /// Indicate if template originated from literal.
    pub(crate) from_literal: bool,
    /// Expressions being concatenated as a template.
    pub(crate) exprs: Vec<ast::Expr>,
}

/// An internal format specification.
#[derive(Spanned)]
pub(crate) struct BuiltInFormat {
    #[rune(span)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Spanned)]
pub(crate) struct BuiltInFile {
    /// The span of the built-in-file
    #[rune(span)]
    pub(crate) span: Span,
    /// Path value to use
    pub(crate) value: ast::LitStr,
}

/// Macro data for `line!()`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Spanned)]
pub(crate) struct BuiltInLine {
    /// The span of the built-in-file
    #[rune(span)]
    pub(crate) span: Span,
    /// The line number
    pub(crate) value: ast::LitNumber,
}

#[derive(Default)]
pub(crate) struct QueryInner {
    /// Resolved meta about every single item during a compilation.
    meta: HashMap<ItemId, meta::Meta>,
    /// Build queue.
    queue: VecDeque<BuildEntry>,
    /// Indexed items that can be queried for, which will queue up for them to
    /// be compiled.
    indexed: BTreeMap<ItemId, Vec<IndexedEntry>>,
    /// Compiled constant functions.
    const_fns: HashMap<NonZeroId, Arc<QueryConstFn>>,
    /// Query paths.
    query_paths: HashMap<NonZeroId, QueryPath>,
    /// The result of internally resolved macros.
    internal_macros: HashMap<NonZeroId, BuiltInMacro>,
    /// Associated between `id` and `Item`. Use to look up items through
    /// `item_for` with an opaque id.
    ///
    /// These items are associated with AST elements, and encodoes the item path
    /// that the AST element was indexed.
    items: HashMap<NonZeroId, ItemMeta>,
    /// All available names in the context.
    names: Names,
}

/// Query system of the rune compiler.
///
/// Once an item is queried for it is queued up for compilation.
pub(crate) struct Query<'a> {
    /// The current unit being built.
    pub(crate) unit: &'a mut UnitBuilder,
    /// The prelude in effect.
    prelude: &'a Prelude,
    /// Cache of constants that have been expanded.
    pub(crate) consts: &'a mut Consts,
    /// Storage associated with the query.
    pub(crate) storage: &'a mut Storage,
    /// Sources available.
    pub(crate) sources: &'a mut Sources,
    /// Pool of allocates items and modules.
    pub(crate) pool: &'a mut Pool,
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
        prelude: &'a Prelude,
        consts: &'a mut Consts,
        storage: &'a mut Storage,
        sources: &'a mut Sources,
        pool: &'a mut Pool,
        visitor: &'a mut dyn CompileVisitor,
        gen: &'a Gen,
        inner: &'a mut QueryInner,
    ) -> Self {
        Self {
            unit,
            prelude,
            consts,
            storage,
            sources,
            pool,
            visitor,
            gen,
            inner,
        }
    }

    /// Reborrow the query engine from a reference to `self`.
    pub(crate) fn borrow(&mut self) -> Query<'_> {
        Query {
            unit: self.unit,
            prelude: self.prelude,
            consts: self.consts,
            storage: self.storage,
            pool: self.pool,
            sources: self.sources,
            visitor: self.visitor,
            gen: self.gen,
            inner: self.inner,
        }
    }

    /// Get the next build entry from the build queue associated with the query
    /// engine.
    pub(crate) fn next_build_entry(&mut self) -> Option<BuildEntry> {
        self.inner.queue.pop_front()
    }

    /// Insert path information.
    pub(crate) fn insert_path(
        &mut self,
        module: ModId,
        impl_item: Option<ItemId>,
        item: &Item,
    ) -> NonZeroId {
        let item = self.pool.alloc_item(item);
        let id = self.gen.next();

        let old = self.inner.query_paths.insert(
            id,
            QueryPath {
                module,
                impl_item,
                item,
            },
        );

        debug_assert!(old.is_none(), "should use a unique identifier");
        id
    }

    /// Remove a reference to the given path by id.
    pub(crate) fn remove_path_by_id(&mut self, id: Id) {
        if let Some(id) = id.as_ref() {
            self.inner.query_paths.remove(id);
        }
    }

    /// Insert module and associated metadata.
    pub(crate) fn insert_mod(
        &mut self,
        items: &Items,
        location: Location,
        parent: ModId,
        visibility: Visibility,
        docs: &[Doc],
    ) -> compile::Result<ModId> {
        let item = self.insert_new_item(items, location, parent, visibility, docs)?;

        let query_mod = self.pool.alloc_module(ModMeta {
            location,
            item: item.item,
            visibility,
            parent: Some(parent),
        });

        self.index_and_build(IndexedEntry {
            item_meta: item,
            indexed: Indexed::Module,
        });
        Ok(query_mod)
    }

    /// Insert module and associated metadata.
    pub(crate) fn insert_root_mod(
        &mut self,
        source_id: SourceId,
        spanned: Span,
    ) -> compile::Result<ModId> {
        let query_mod = self.pool.alloc_module(ModMeta {
            location: Location::new(source_id, spanned),
            item: ItemId::default(),
            visibility: Visibility::Public,
            parent: None,
        });

        self.insert_name(ItemId::default());
        Ok(query_mod)
    }

    /// Inserts an item that *has* to be unique, else cause an error.
    ///
    /// This are not indexed and does not generate an ID, they're only visible
    /// in reverse lookup.
    pub(crate) fn insert_new_item(
        &mut self,
        items: &Items,
        location: Location,
        module: ModId,
        visibility: Visibility,
        docs: &[Doc],
    ) -> compile::Result<ItemMeta> {
        let id = items
            .id()
            .map_err(|e| compile::Error::msg(location.span, e))?;
        let item = self.pool.alloc_item(&*items.item());
        self.insert_new_item_with(id, item, location, module, visibility, docs)
    }

    /// Insert the given compile meta.
    fn insert_meta(&mut self, span: Span, meta: meta::Meta) -> compile::Result<()> {
        self.visitor.register_meta(meta.as_meta_ref(self.pool));

        match self.inner.meta.entry(meta.item_meta.item) {
            hash_map::Entry::Occupied(e) => {
                return Err(compile::Error::new(
                    span,
                    QueryErrorKind::MetaConflict {
                        current: meta.info(self.pool),
                        existing: e.get().info(self.pool),
                    },
                ));
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(meta);
            }
        }

        Ok(())
    }

    /// Insert a new item with the given newly allocated identifier and complete
    /// `Item`.
    fn insert_new_item_with(
        &mut self,
        id: NonZeroId,
        item: ItemId,
        location: Location,
        module: ModId,
        visibility: Visibility,
        docs: &[Doc],
    ) -> compile::Result<ItemMeta> {
        // Emit documentation comments for the given item.
        if !docs.is_empty() {
            let ctx = resolve_context!(self);

            for doc in docs {
                self.visitor.visit_doc_comment(
                    Location::new(location.source_id, doc.span),
                    self.pool.item(item),
                    self.pool.item_type_hash(item),
                    doc.doc_string.resolve(ctx)?.as_ref(),
                );
            }
        }

        let item_meta = ItemMeta {
            location,
            id: Id::new(id),
            item,
            module,
            visibility,
        };

        self.inner.items.insert(id, item_meta);
        Ok(item_meta)
    }

    /// Insert a new expanded internal macro.
    pub(crate) fn insert_new_builtin_macro(
        &mut self,
        internal_macro: BuiltInMacro,
    ) -> compile::Result<NonZeroId> {
        let id = self.gen.next();
        self.inner.internal_macros.insert(id, internal_macro);
        Ok(id)
    }

    /// Get the item for the given identifier.
    pub(crate) fn item_for<T>(&self, ast: T) -> compile::Result<ItemMeta>
    where
        T: Spanned + Opaque,
    {
        match ast.id().as_ref().and_then(|n| self.inner.items.get(n)) {
            Some(item_meta) => Ok(*item_meta),
            None => Err(compile::Error::new(
                ast.span(),
                QueryErrorKind::MissingId {
                    what: "item",
                    id: ast.id(),
                },
            )),
        }
    }

    /// Get the built-in macro matching the given ast.
    pub(crate) fn builtin_macro_for<T>(&self, ast: T) -> compile::Result<&BuiltInMacro>
    where
        T: Spanned + Opaque,
    {
        match ast
            .id()
            .as_ref()
            .and_then(|n| self.inner.internal_macros.get(n))
        {
            Some(internal_macro) => Ok(internal_macro),
            None => Err(compile::Error::new(
                ast.span(),
                QueryErrorKind::MissingId {
                    what: "builtin macro",
                    id: ast.id(),
                },
            )),
        }
    }

    /// Insert an item and return its Id.
    #[tracing::instrument(skip_all)]
    fn insert_const_fn(&mut self, item_meta: ItemMeta, ir_fn: ir::IrFn) -> NonZeroId {
        let id = self.gen.next();
        tracing::trace!(item = ?self.pool.item(item_meta.item), id = ?id);

        self.inner
            .const_fns
            .insert(id, Arc::new(QueryConstFn { item_meta, ir_fn }));

        id
    }

    /// Get the constant function associated with the opaque.
    pub(crate) fn const_fn_for<T>(&self, ast: T) -> compile::Result<Arc<QueryConstFn>>
    where
        T: Spanned + Opaque,
    {
        match ast.id().as_ref().and_then(|n| self.inner.const_fns.get(n)) {
            Some(const_fn) => Ok(const_fn.clone()),
            None => Err(compile::Error::new(
                ast.span(),
                QueryErrorKind::MissingId {
                    what: "constant function",
                    id: ast.id(),
                },
            )),
        }
    }

    /// Index the given entry. It is not allowed to overwrite other entries.
    #[tracing::instrument(skip_all)]
    pub(crate) fn index(&mut self, entry: IndexedEntry) {
        tracing::trace!(item = ?entry.item_meta.item);

        self.insert_name(entry.item_meta.item);

        self.inner
            .indexed
            .entry(entry.item_meta.item)
            .or_default()
            .push(entry);
    }

    /// Same as `index`, but also queues the indexed entry up for building.
    #[tracing::instrument(skip_all)]
    pub(crate) fn index_and_build(&mut self, entry: IndexedEntry) {
        self.inner.queue.push_back(BuildEntry {
            item_meta: entry.item_meta,
            used: Used::Used,
            build: Build::Query,
        });

        self.index(entry);
    }

    /// Index a constant expression.
    #[tracing::instrument(skip_all)]
    pub(crate) fn index_const<T>(
        &mut self,
        item_meta: ItemMeta,
        value: &T,
        f: fn(&T, &mut IrCompiler) -> compile::Result<ir::Ir>,
    ) -> compile::Result<()> {
        tracing::trace!(item = ?self.pool.item(item_meta.item));

        let mut c = IrCompiler {
            source_id: item_meta.location.source_id,
            q: self.borrow(),
        };
        let ir = f(value, &mut c)?;

        self.index(IndexedEntry {
            item_meta,
            indexed: Indexed::Const(Const {
                module: item_meta.module,
                ir,
            }),
        });

        Ok(())
    }

    /// Index a constant function.
    #[tracing::instrument(skip_all)]
    pub(crate) fn index_const_fn(
        &mut self,
        item_meta: ItemMeta,
        item_fn: Box<ast::ItemFn>,
    ) -> compile::Result<()> {
        tracing::trace!(item = ?self.pool.item(item_meta.item));

        self.index(IndexedEntry {
            item_meta,
            indexed: Indexed::ConstFn(ConstFn {
                location: item_meta.location,
                item_fn,
            }),
        });

        Ok(())
    }

    /// Add a new enum item.
    #[tracing::instrument(skip_all)]
    pub(crate) fn index_enum(&mut self, item_meta: ItemMeta) -> compile::Result<()> {
        tracing::trace!(item = ?self.pool.item(item_meta.item));

        self.index(IndexedEntry {
            item_meta,
            indexed: Indexed::Enum,
        });

        Ok(())
    }

    /// Add a new struct item that can be queried.
    #[tracing::instrument(skip_all)]
    pub(crate) fn index_struct(
        &mut self,
        item_meta: ItemMeta,
        ast: Box<ast::ItemStruct>,
    ) -> compile::Result<()> {
        tracing::trace!(item = ?self.pool.item(item_meta.item));

        self.index(IndexedEntry {
            item_meta,
            indexed: Indexed::Struct(Struct::new(ast)),
        });

        Ok(())
    }

    /// Add a new variant item that can be queried.
    #[tracing::instrument(skip_all)]
    pub(crate) fn index_variant(
        &mut self,
        item_meta: ItemMeta,
        enum_id: Id,
        ast: ast::ItemVariant,
        index: usize,
    ) -> compile::Result<()> {
        tracing::trace!(item = ?self.pool.item(item_meta.item));

        self.index(IndexedEntry {
            item_meta,
            indexed: Indexed::Variant(Variant::new(enum_id, ast, index)),
        });

        Ok(())
    }

    /// Add a new function that can be queried for.
    #[tracing::instrument(skip_all)]
    pub(crate) fn index_closure(
        &mut self,
        item_meta: ItemMeta,
        ast: Box<ast::ExprClosure>,
        captures: Arc<[String]>,
        call: Call,
        do_move: bool,
    ) -> compile::Result<()> {
        tracing::trace!(item = ?self.pool.item(item_meta.item));

        self.index(IndexedEntry {
            item_meta,
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
    #[tracing::instrument(skip_all)]
    pub(crate) fn index_async_block(
        &mut self,
        item_meta: ItemMeta,
        ast: ast::Block,
        captures: Arc<[String]>,
        call: Call,
        do_move: bool,
    ) -> compile::Result<()> {
        tracing::trace!(item = ?self.pool.item(item_meta.item));

        self.index(IndexedEntry {
            item_meta,
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
    #[tracing::instrument(skip_all)]
    pub(crate) fn queue_unused_entries(
        &mut self,
    ) -> compile::Result<bool, (SourceId, compile::Error)> {
        tracing::trace!("queue unused");

        let unused = self
            .inner
            .indexed
            .values()
            .flat_map(|entries| entries.iter())
            .map(|e| (e.item_meta.location, e.item_meta.item))
            .collect::<Vec<_>>();

        if unused.is_empty() {
            return Ok(false);
        }

        for (location, item) in unused {
            let _ = self
                .query_indexed_meta(location.span, item, Used::Unused)
                .map_err(|e| (location.source_id, e))?;
        }

        Ok(true)
    }

    /// Insert context meta.
    pub(crate) fn insert_context_meta(
        &mut self,
        span: Span,
        context_meta: &context::ContextMeta,
    ) -> compile::Result<meta::Meta> {
        let meta = meta::Meta {
            context: true,
            hash: context_meta.hash,
            associated_container: context_meta.associated_container,
            item_meta: ItemMeta {
                id: Default::default(),
                location: Default::default(),
                item: self.pool.alloc_item(&context_meta.item),
                visibility: Default::default(),
                module: Default::default(),
            },
            kind: context_meta.kind.clone(),
            source: None,
        };

        self.insert_meta(span, meta.clone())?;
        Ok(meta)
    }

    /// Query for the given meta by looking up the reverse of the specified
    /// item.
    #[tracing::instrument(skip(self, span, item), fields(item = ?self.pool.item(item)))]
    pub(crate) fn query_meta(
        &mut self,
        span: Span,
        item: ItemId,
        used: Used,
    ) -> compile::Result<Option<meta::Meta>> {
        if let Some(meta) = self.inner.meta.get(&item) {
            tracing::trace!(item = ?item, meta = ?meta, "cached");
            // Ensure that the given item is not indexed, cause if it is
            // `queue_unused_entries` might end up spinning indefinitely since
            // it will never be exhausted.
            debug_assert!(!self.inner.indexed.contains_key(&item));
            return Ok(Some(meta.clone()));
        }

        self.query_indexed_meta(span, item, used)
    }

    /// Only try and query for meta among items which have been indexed.
    fn query_indexed_meta(
        &mut self,
        span: Span,
        item: ItemId,
        used: Used,
    ) -> compile::Result<Option<meta::Meta>> {
        if let Some(entry) = self.remove_indexed(span, item)? {
            let meta = self.build_indexed_entry(span, entry, used)?;
            self.unit.insert_meta(span, &meta, self.pool)?;
            self.insert_meta(span, meta.clone())?;
            tracing::trace!(item = ?item, meta = ?meta, "build");
            return Ok(Some(meta));
        }

        Ok(None)
    }

    /// Perform a path lookup on the current state of the unit.
    #[tracing::instrument(skip_all)]
    pub(crate) fn convert_path<'hir>(
        &mut self,
        context: &Context,
        path: &'hir hir::Path<'hir>,
    ) -> compile::Result<Named<'hir>> {
        let id = path.id();

        let qp = *id
            .as_ref()
            .and_then(|id| self.inner.query_paths.get(id))
            .ok_or_else(|| {
                compile::Error::new(path, QueryErrorKind::MissingId { what: "path", id })
            })?;

        let mut in_self_type = false;
        let mut local = None;
        let mut generics = None;

        let item = match (path.global, path.first) {
            (
                Some(..),
                hir::PathSegment {
                    kind: hir::PathSegmentKind::Ident(ident),
                    ..
                },
            ) => self
                .pool
                .alloc_item(ItemBuf::with_crate(ident.resolve(resolve_context!(self))?)),
            (Some(span), _) => {
                return Err(compile::Error::new(
                    span,
                    CompileErrorKind::UnsupportedGlobal,
                ));
            }
            (None, segment) => match segment.kind {
                hir::PathSegmentKind::Ident(ident) => {
                    if path.rest.is_empty() {
                        local = Some(ident);
                    }

                    self.convert_initial_path(context, qp.module, qp.item, ident)?
                }
                hir::PathSegmentKind::Super => self
                    .pool
                    .try_map_alloc(self.pool.module(qp.module).item, Item::parent)
                    .ok_or_else(compile::Error::unsupported_super(segment.span()))?,
                hir::PathSegmentKind::SelfType => {
                    let impl_item = qp.impl_item.ok_or_else(|| {
                        compile::Error::new(segment.span(), CompileErrorKind::UnsupportedSelfType)
                    })?;

                    in_self_type = true;
                    impl_item
                }
                hir::PathSegmentKind::SelfValue => self.pool.module(qp.module).item,
                hir::PathSegmentKind::Crate => ItemId::default(),
                hir::PathSegmentKind::Generics(..) => {
                    return Err(compile::Error::new(
                        segment.span(),
                        CompileErrorKind::UnsupportedGenerics,
                    ));
                }
            },
        };

        let mut item = self.pool.item(item).to_owned();

        for segment in path.rest {
            tracing::trace!("item = {}", item);

            if generics.is_some() {
                return Err(compile::Error::new(
                    segment,
                    CompileErrorKind::UnsupportedAfterGeneric,
                ));
            }

            match segment.kind {
                hir::PathSegmentKind::Ident(ident) => {
                    let ident = ident.resolve(resolve_context!(self))?;
                    item.push(ident);
                }
                hir::PathSegmentKind::Super => {
                    if in_self_type {
                        return Err(compile::Error::new(
                            segment.span(),
                            CompileErrorKind::UnsupportedSuperInSelfType,
                        ));
                    }

                    item.pop()
                        .ok_or_else(compile::Error::unsupported_super(segment.span()))?;
                }
                hir::PathSegmentKind::Generics(arguments) => {
                    if generics.is_some() {
                        return Err(compile::Error::new(
                            segment.span(),
                            CompileErrorKind::UnsupportedGenerics,
                        ));
                    }

                    generics = Some((segment.span(), arguments));
                }
                _ => {
                    return Err(compile::Error::new(
                        segment.span(),
                        CompileErrorKind::ExpectedLeadingPathSegment,
                    ));
                }
            }
        }

        let span = path.span();

        let local = match local {
            Some(local) => Some(local.resolve(resolve_context!(self))?.into()),
            None => None,
        };

        let item = self.pool.alloc_item(item);

        if let Some(new) = self.import(span, qp.module, item, Used::Used)? {
            return Ok(Named {
                local,
                item: new,
                generics,
            });
        }

        Ok(Named {
            local,
            item,
            generics,
        })
    }

    /// Declare a new import.
    #[tracing::instrument(skip_all)]
    pub(crate) fn insert_import(
        &mut self,
        source_id: SourceId,
        span: Span,
        module: ModId,
        visibility: Visibility,
        at: ItemBuf,
        target: ItemBuf,
        alias: Option<ast::Ident>,
        wildcard: bool,
    ) -> compile::Result<()> {
        tracing::trace!(at = ?at, target = ?target);

        let alias = match alias {
            Some(alias) => Some(alias.resolve(resolve_context!(self))?),
            None => None,
        };

        let last = alias
            .as_ref()
            .map(IntoComponent::as_component_ref)
            .or_else(|| target.last())
            .ok_or_else(|| compile::Error::new(span, QueryErrorKind::LastUseComponent))?;

        let item = self.pool.alloc_item(at.extended(last));
        let target = self.pool.alloc_item(target);
        let location = Location::new(source_id, span);

        let entry = meta::Import {
            location,
            target,
            module,
        };

        let id = self.gen.next();
        let item_meta = self.insert_new_item_with(id, item, location, module, visibility, &[])?;

        // toplevel public uses are re-exported.
        if item_meta.is_public(self.pool) {
            self.inner.queue.push_back(BuildEntry {
                item_meta,
                build: Build::ReExport,
                used: Used::Used,
            });
        }

        self.index(IndexedEntry {
            item_meta,
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
    #[tracing::instrument(skip(self, span, module))]
    pub(crate) fn import(
        &mut self,
        span: Span,
        mut module: ModId,
        item: ItemId,
        used: Used,
    ) -> compile::Result<Option<ItemId>> {
        let mut visited = HashSet::<ItemId>::new();
        let mut path = Vec::new();
        let mut item = self.pool.item(item).to_owned();
        let mut any_matched = false;

        let mut count = 0usize;

        'outer: loop {
            if count > IMPORT_RECURSION_LIMIT {
                return Err(compile::Error::new(
                    span,
                    QueryErrorKind::ImportRecursionLimit { count, path },
                ));
            }

            count += 1;

            let mut cur = ItemBuf::new();
            let mut it = item.iter();

            while let Some(c) = it.next() {
                cur.push(c);
                let cur = self.pool.alloc_item(&cur);

                let update = self.import_step(span, module, cur, used, &mut path)?;

                let update = match update {
                    Some(update) => update,
                    None => continue,
                };

                path.push(ImportStep {
                    location: update.location,
                    item: self.pool.item(update.target).to_owned(),
                });

                if !visited.insert(self.pool.alloc_item(&item)) {
                    return Err(compile::Error::new(
                        span,
                        QueryErrorKind::ImportCycle { path },
                    ));
                }

                module = update.module;
                item = self.pool.item(update.target).join(it);
                any_matched = true;
                continue 'outer;
            }

            break;
        }

        if any_matched {
            return Ok(Some(self.pool.alloc_item(item)));
        }

        Ok(None)
    }

    /// Inner import implementation that doesn't walk the imported name.
    #[tracing::instrument(skip(self, span, module, path))]
    fn import_step(
        &mut self,
        span: Span,
        module: ModId,
        item: ItemId,
        used: Used,
        path: &mut Vec<ImportStep>,
    ) -> compile::Result<Option<meta::Import>> {
        // already resolved query.
        if let Some(meta) = self.inner.meta.get(&item) {
            return Ok(match meta.kind {
                meta::Kind::Import(import) => Some(import),
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
            module,
            item,
            entry.item_meta.module,
            entry.item_meta.location,
            entry.item_meta.visibility,
            path,
        )?;

        let import = match entry.indexed {
            Indexed::Import(import) => import.entry,
            indexed => {
                self.import_indexed(span, entry.item_meta, indexed, used)?;
                return Ok(None);
            }
        };

        let meta = meta::Meta {
            context: false,
            hash: self.pool.item_type_hash(entry.item_meta.item),
            associated_container: None,
            item_meta: entry.item_meta,
            kind: meta::Kind::Import(import),
            source: None,
        };

        self.insert_meta(span, meta)?;
        Ok(Some(import))
    }

    /// Build a single, indexed entry and return its metadata.
    fn build_indexed_entry(
        &mut self,
        span: Span,
        entry: IndexedEntry,
        used: Used,
    ) -> compile::Result<meta::Meta> {
        let IndexedEntry { item_meta, indexed } = entry;

        let (hash, container, kind) = match indexed {
            Indexed::Enum => {
                let hash = self.pool.item_type_hash(item_meta.item);
                (hash, None, meta::Kind::Enum)
            }
            Indexed::Variant(variant) => {
                let enum_item = self.item_for((item_meta.location.span, variant.enum_id))?;

                // Assert that everything is built for the enum.
                self.query_meta(span, enum_item.item, Default::default())?;
                let enum_hash = self.pool.item_type_hash(enum_item.item);

                let (hash, kind) = variant_into_item_decl(
                    self.pool.item(item_meta.item),
                    variant.ast.body,
                    Some((enum_hash, variant.index)),
                    resolve_context!(self),
                )?;

                (hash, Some(enum_hash), kind)
            }
            Indexed::Struct(st) => {
                let (hash, kind) = struct_into_item_decl(
                    self.pool.item(item_meta.item),
                    st.ast.body,
                    resolve_context!(self),
                )?;

                (hash, None, kind)
            }
            Indexed::Function(f) => {
                let hash = self.pool.item_type_hash(item_meta.item);

                let kind = meta::Kind::Function {
                    is_async: f.function.ast.async_token.is_some(),
                    args: Some(f.function.ast.args.len()),
                    is_test: f.is_test,
                    is_bench: f.is_bench,
                    instance_function: false,
                };

                self.inner.queue.push_back(BuildEntry {
                    item_meta,
                    build: Build::Function(f.function),
                    used,
                });

                (hash, None, kind)
            }
            Indexed::InstanceFunction(f) => {
                let hash = self.pool.item_type_hash(item_meta.item);
                let container = self.pool.item_type_hash(f.impl_item);

                let kind = meta::Kind::Function {
                    is_async: f.function.ast.async_token.is_some(),
                    args: Some(f.function.ast.args.len()),
                    is_test: false,
                    is_bench: false,
                    instance_function: true,
                };

                self.inner.queue.push_back(BuildEntry {
                    item_meta,
                    build: Build::InstanceFunction(f),
                    used,
                });

                (hash, Some(container), kind)
            }
            Indexed::Closure(c) => {
                let captures = c.captures.clone();
                let do_move = c.do_move;

                self.inner.queue.push_back(BuildEntry {
                    item_meta,
                    build: Build::Closure(c),
                    used,
                });

                let hash = self.pool.item_type_hash(item_meta.item);

                let kind = meta::Kind::Closure { captures, do_move };

                (hash, None, kind)
            }
            Indexed::AsyncBlock(b) => {
                let captures = b.captures.clone();
                let do_move = b.do_move;

                self.inner.queue.push_back(BuildEntry {
                    item_meta,
                    build: Build::AsyncBlock(b),
                    used,
                });

                let hash = self.pool.item_type_hash(item_meta.item);

                let kind = meta::Kind::AsyncBlock { captures, do_move };

                (hash, None, kind)
            }
            Indexed::Const(c) => {
                let mut const_compiler = IrInterpreter {
                    budget: IrBudget::new(1_000_000),
                    scopes: Default::default(),
                    module: c.module,
                    item: item_meta.item,
                    q: self.borrow(),
                };

                let const_value = const_compiler.eval_const(&c.ir, used)?;

                if used.is_unused() {
                    self.inner.queue.push_back(BuildEntry {
                        item_meta,
                        build: Build::Unused,
                        used,
                    });
                }

                let hash = self.pool.item_type_hash(item_meta.item);
                (hash, None, meta::Kind::Const { const_value })
            }
            Indexed::ConstFn(c) => {
                let ir_fn = {
                    // TODO: avoid this arena?
                    let arena = crate::hir::Arena::new();
                    let ctx = crate::hir::lowering::Ctx::new(&arena, self.borrow());
                    let hir = crate::hir::lowering::item_fn(&ctx, &c.item_fn)?;
                    let mut c = IrCompiler {
                        source_id: c.location.source_id,
                        q: self.borrow(),
                    };
                    ir::IrFn::compile_ast(&hir, &mut c)?
                };

                let id = self.insert_const_fn(item_meta, ir_fn);

                if used.is_unused() {
                    self.inner.queue.push_back(BuildEntry {
                        item_meta,
                        build: Build::Unused,
                        used,
                    });
                }

                let hash = self.pool.item_type_hash(item_meta.item);
                (hash, None, meta::Kind::ConstFn { id: Id::new(id) })
            }
            Indexed::Import(import) => {
                if !import.wildcard {
                    self.inner.queue.push_back(BuildEntry {
                        item_meta,
                        build: Build::Import(import),
                        used,
                    });
                }

                let hash = self.pool.item_type_hash(item_meta.item);
                let kind = meta::Kind::Import(import.entry);
                (hash, None, kind)
            }
            Indexed::Module => {
                let hash = self.pool.item_type_hash(item_meta.item);
                (hash, None, meta::Kind::Module)
            }
        };

        let source = SourceMeta {
            location: item_meta.location,
            path: self
                .sources
                .path(item_meta.location.source_id)
                .map(Into::into),
        };

        Ok(meta::Meta {
            context: false,
            hash,
            associated_container: container,
            item_meta,
            kind,
            source: Some(source),
        })
    }

    /// Insert the given name into the unit.
    fn insert_name(&mut self, item: ItemId) {
        let item = self.pool.item(item);
        self.inner.names.insert(item);
    }

    /// Handle an imported indexed entry.
    fn import_indexed(
        &mut self,
        span: Span,
        item_meta: ItemMeta,
        indexed: Indexed,
        used: Used,
    ) -> compile::Result<()> {
        // NB: if we find another indexed entry, queue it up for
        // building and clone its built meta to the other
        // results.
        let entry = IndexedEntry { item_meta, indexed };

        let meta = self.build_indexed_entry(span, entry, used)?;
        self.unit.insert_meta(span, &meta, self.pool)?;
        self.insert_meta(span, meta)?;
        Ok(())
    }

    /// Remove the indexed entry corresponding to the given item..
    fn remove_indexed(
        &mut self,
        span: Span,
        item: ItemId,
    ) -> compile::Result<Option<IndexedEntry>> {
        // See if there's an index entry we can construct and insert.
        let entries = match self.inner.indexed.remove(&item) {
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

        let mut locations = vec![(cur.item_meta.location, cur.item().to_owned())];

        while let Some(oth) = it.next() {
            locations.push((oth.item_meta.location, oth.item().to_owned()));

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
                locations.push((oth.item_meta.location, oth.item().to_owned()));
            }

            return Err(compile::Error::new(
                span,
                QueryErrorKind::AmbiguousItem {
                    item: self.pool.item(cur.item_meta.item).to_owned(),
                    locations: locations
                        .into_iter()
                        .map(|(loc, item)| (loc, self.pool.item(item).to_owned()))
                        .collect(),
                },
            ));
        }

        if let Indexed::Import(Import { wildcard: true, .. }) = &cur.indexed {
            return Err(compile::Error::new(
                span,
                QueryErrorKind::AmbiguousItem {
                    item: self.pool.item(cur.item_meta.item).to_owned(),
                    locations: locations
                        .into_iter()
                        .map(|(loc, item)| (loc, self.pool.item(item).to_owned()))
                        .collect(),
                },
            ));
        }

        Ok(Some(cur))
    }

    /// Walk the names to find the first one that is contained in the unit.
    #[tracing::instrument(skip_all, fields(module = ?self.pool.module_item(module), base = ?self.pool.item(base)))]
    fn convert_initial_path(
        &mut self,
        context: &Context,
        module: ModId,
        base: ItemId,
        local: &ast::Ident,
    ) -> compile::Result<ItemId> {
        let span = local.span;
        let mut base = self.pool.item(base).to_owned();
        debug_assert!(base.starts_with(self.pool.module_item(module)));

        let local = local.resolve(resolve_context!(self))?.to_owned();

        while base.starts_with(self.pool.module_item(module)) {
            base.push(&local);

            tracing::trace!(?base, "testing");

            if self.inner.names.contains(&base) {
                let item = self.pool.alloc_item(&base);

                // TODO: We probably should not engage the whole query meta
                // machinery here.
                if let Some(meta) = self.query_meta(span, item, Used::Used)? {
                    if !matches!(
                        meta.kind,
                        meta::Kind::Function {
                            instance_function: true,
                            ..
                        }
                    ) {
                        return Ok(self.pool.alloc_item(base));
                    }
                }
            }

            let c = base.pop();
            debug_assert!(c.is_some());

            if base.pop().is_none() {
                break;
            }
        }

        if let Some(item) = self.prelude.get(&local) {
            return Ok(self.pool.alloc_item(item));
        }

        if context.contains_crate(&local) {
            return Ok(self.pool.alloc_item(ItemBuf::with_crate(&local)));
        }

        let new_module = self.pool.module_item(module).extended(&local);
        Ok(self.pool.alloc_item(new_module))
    }

    /// Check that the given item is accessible from the given module.
    fn check_access_to(
        &mut self,
        span: Span,
        from: ModId,
        item: ItemId,
        module: ModId,
        location: Location,
        visibility: Visibility,
        chain: &mut Vec<ImportStep>,
    ) -> compile::Result<()> {
        let (common, tree) = self
            .pool
            .module_item(from)
            .ancestry(self.pool.module_item(module));
        let mut current_module = common.clone();

        // Check each module from the common ancestrly to the module.
        for c in &tree {
            current_module.push(c);
            let current_module_id = self.pool.alloc_item(&current_module);

            let m = self.pool.module_by_item(current_module_id).ok_or_else(|| {
                compile::Error::new(
                    span,
                    QueryErrorKind::MissingMod {
                        item: current_module.clone(),
                    },
                )
            })?;

            if !m.visibility.is_visible(&common, &current_module) {
                return Err(compile::Error::new(
                    span,
                    QueryErrorKind::NotVisibleMod {
                        chain: into_chain(take(chain)),
                        location: m.location,
                        visibility: m.visibility,
                        item: current_module,
                        from: self.pool.module_item(from).to_owned(),
                    },
                ));
            }
        }

        if !visibility.is_visible_inside(&common, self.pool.module_item(module)) {
            return Err(compile::Error::new(
                span,
                QueryErrorKind::NotVisible {
                    chain: into_chain(take(chain)),
                    location,
                    visibility,
                    item: self.pool.item(item).to_owned(),
                    from: self.pool.module_item(from).to_owned(),
                },
            ));
        }

        Ok(())
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

#[derive(Debug, Clone)]
pub(crate) enum Indexed {
    /// An enum.
    Enum,
    /// A struct.
    Struct(Struct),
    /// A variant.
    Variant(Variant),
    /// A function.
    Function(IndexedFunction),
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
    /// The index of the variant in its source.
    index: usize,
}

impl Variant {
    /// Construct a new variant.
    pub(crate) fn new(enum_id: Id, ast: ast::ItemVariant, index: usize) -> Self {
        Self {
            enum_id,
            ast,
            index,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Function {
    /// Ast for declaration.
    pub(crate) ast: Box<ast::ItemFn>,
    /// The calling convention of the function.
    pub(crate) call: Call,
}

#[derive(Debug, Clone)]
pub(crate) struct IndexedFunction {
    /// The underlying indexed function.
    pub(crate) function: Function,
    /// If this is a test function.
    pub(crate) is_test: bool,
    /// If this is a bench function.
    pub(crate) is_bench: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct InstanceFunction {
    /// Ast for the instance function.
    pub(crate) function: Function,
    /// The item of the instance function.
    pub(crate) impl_item: ItemId,
    /// The span of the instance function.
    pub(crate) instance_span: Span,
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
    /// A build which simply queries for the item.
    Query,
}

/// An entry in the build queue.
#[derive(Debug, Clone)]
pub(crate) struct BuildEntry {
    /// The item of the build entry.
    pub(crate) item_meta: ItemMeta,
    /// If the queued up entry was unused or not.
    pub(crate) used: Used,
    /// The build entry.
    pub(crate) build: Build,
}

#[derive(Debug, Clone)]
pub(crate) struct IndexedEntry {
    /// The query item this indexed entry belongs to.
    pub(crate) item_meta: ItemMeta,
    /// The entry data.
    pub(crate) indexed: Indexed,
}

impl IndexedEntry {
    /// The item that best describes this indexed entry.
    pub(crate) fn item(&self) -> ItemId {
        match &self.indexed {
            Indexed::Import(Import { entry, .. }) => entry.target,
            _ => self.item_meta.item,
        }
    }
}

/// Query information for a path.
#[derive(Debug, Clone, Copy)]
pub(crate) struct QueryPath {
    pub(crate) module: ModId,
    pub(crate) impl_item: Option<ItemId>,
    pub(crate) item: ItemId,
}

/// An indexed constant function.
#[derive(Debug)]
pub(crate) struct QueryConstFn {
    /// The item of the const fn.
    pub(crate) item_meta: ItemMeta,
    /// The compiled constant function.
    pub(crate) ir_fn: ir::IrFn,
}

/// The result of calling [Query::convert_path].
#[derive(Debug)]
pub(crate) struct Named<'hir> {
    /// If the resolved value is local.
    pub(crate) local: Option<Box<str>>,
    /// The path resolved to the given item.
    pub(crate) item: ItemId,
    /// Generic arguments if any.
    pub(crate) generics: Option<(Span, &'hir [hir::Expr<'hir>])>,
}

impl Named<'_> {
    /// Get the local identifier of this named.
    pub(crate) fn as_local(&self) -> Option<&str> {
        if self.generics.is_none() {
            self.local.as_deref()
        } else {
            None
        }
    }

    /// Assert that this named type is not generic.
    pub(crate) fn assert_not_generic(&self) -> compile::Result<()> {
        if let Some((span, _)) = self.generics {
            return Err(compile::Error::new(
                span,
                CompileErrorKind::UnsupportedGenerics,
            ));
        }

        Ok(())
    }
}

impl fmt::Display for Named<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.item, f)
    }
}

/// Construct metadata for an empty body.
fn unit_body_meta(item: &Item, enum_item: Option<(Hash, usize)>) -> (Hash, meta::Kind) {
    let type_hash = Hash::type_hash(item);

    let kind = match enum_item {
        Some((enum_hash, index)) => meta::Kind::Variant {
            enum_hash,
            index,
            fields: meta::Fields::Unit,
        },
        None => meta::Kind::Struct {
            fields: meta::Fields::Unit,
        },
    };

    (type_hash, kind)
}

/// Construct metadata for an empty body.
fn tuple_body_meta(
    item: &Item,
    enum_: Option<(Hash, usize)>,
    tuple: ast::Parenthesized<ast::Field, T![,]>,
) -> (Hash, meta::Kind) {
    let type_hash = Hash::type_hash(item);

    let tuple = meta::Tuple {
        args: tuple.len(),
        hash: Hash::type_hash(item),
    };

    let kind = match enum_ {
        Some((enum_hash, index)) => meta::Kind::Variant {
            enum_hash,
            index,
            fields: meta::Fields::Tuple(tuple),
        },
        None => meta::Kind::Struct {
            fields: meta::Fields::Tuple(tuple),
        },
    };

    (type_hash, kind)
}

/// Construct metadata for a struct body.
fn struct_body_meta(
    item: &Item,
    enum_: Option<(Hash, usize)>,
    ctx: ResolveContext<'_>,
    st: ast::Braced<ast::Field, T![,]>,
) -> compile::Result<(Hash, meta::Kind)> {
    let type_hash = Hash::type_hash(item);

    let mut fields = HashSet::new();

    for (ast::Field { name, .. }, _) in st {
        let name = name.resolve(ctx)?;
        fields.insert(name.into());
    }

    let st = meta::Struct { fields };

    let kind = match enum_ {
        Some((enum_hash, index)) => meta::Kind::Variant {
            enum_hash,
            index,
            fields: meta::Fields::Struct(st),
        },
        None => meta::Kind::Struct {
            fields: meta::Fields::Struct(st),
        },
    };

    Ok((type_hash, kind))
}

/// Convert an ast declaration into a struct.
fn variant_into_item_decl(
    item: &Item,
    body: ast::ItemVariantBody,
    enum_: Option<(Hash, usize)>,
    ctx: ResolveContext<'_>,
) -> compile::Result<(Hash, meta::Kind)> {
    Ok(match body {
        ast::ItemVariantBody::UnitBody => unit_body_meta(item, enum_),
        ast::ItemVariantBody::TupleBody(tuple) => tuple_body_meta(item, enum_, tuple),
        ast::ItemVariantBody::StructBody(st) => struct_body_meta(item, enum_, ctx, st)?,
    })
}

/// Convert an ast declaration into a struct.
fn struct_into_item_decl(
    item: &Item,
    body: ast::ItemStructBody,
    ctx: ResolveContext<'_>,
) -> compile::Result<(Hash, meta::Kind)> {
    Ok(match body {
        ast::ItemStructBody::UnitBody => unit_body_meta(item, None),
        ast::ItemStructBody::TupleBody(tuple) => tuple_body_meta(item, None, tuple),
        ast::ItemStructBody::StructBody(st) => struct_body_meta(item, None, ctx, st)?,
    })
}

fn into_chain(chain: Vec<ImportStep>) -> Vec<Location> {
    chain.into_iter().map(|c| c.location).collect()
}
