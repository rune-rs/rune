//! Worker used by compiler.

mod import;
mod task;
mod wildcard_import;

use rust_alloc::rc::Rc;

use crate::alloc::prelude::*;
use crate::alloc::{self, HashMap, Vec, VecDeque};
use crate::ast::{self, Kind, Span, Spanned};
use crate::compile::{self, ItemId, ModId, WithSpan};
use crate::grammar::{Node, Stream};
use crate::indexing::{index, index2};
use crate::macros::{MacroContext, TokenStream};
use crate::parse::Resolve;
use crate::query::{
    BuiltInLiteral, BuiltInMacro2, DeferEntry, ExpandMacroBuiltin, ExpandedMacro,
    GenericsParameters, ImplItem, ImplItemKind, Query, Used,
};
use crate::SourceId;

pub(crate) use self::import::{Import, ImportState};
pub(crate) use self::task::{LoadFileKind, Task};
pub(crate) use self::wildcard_import::WildcardImport;

pub(crate) struct Worker<'a, 'arena> {
    /// Query engine.
    pub(crate) q: Query<'a, 'arena>,
    /// Files that have been loaded.
    pub(crate) loaded: HashMap<ModId, (SourceId, Span)>,
    /// Worker queue.
    pub(crate) queue: VecDeque<Task>,
}

impl<'a, 'arena> Worker<'a, 'arena> {
    /// Construct a new worker.
    pub(crate) fn new(q: Query<'a, 'arena>) -> Self {
        Self {
            q,
            loaded: HashMap::new(),
            queue: VecDeque::new(),
        }
    }

    /// Perform indexing in the worker.
    #[tracing::instrument(skip_all)]
    pub(crate) fn index(&mut self) -> alloc::Result<()> {
        // NB: defer wildcard expansion until all other imports have been
        // indexed.
        let mut wildcard_imports = Vec::new();

        while !self.queue.is_empty() {
            // Prioritise processing the indexing queue. This ensures that files
            // and imports are loaded which might be used by subsequent steps.
            // We only advance wildcard imports and impl items once this is
            // empty.
            //
            // Language semantics also ensures that once this queue is drained,
            // every item which might affect the behavior of imports has been
            // indexed.
            while let Some(task) = self.queue.pop_front() {
                match task {
                    Task::LoadFile {
                        kind,
                        source_id,
                        mod_item,
                        mod_item_id,
                    } => {
                        let result = self.load_file(kind, source_id, mod_item, mod_item_id);

                        if let Err(error) = result {
                            self.q.diagnostics.error(source_id, error)?;
                        }
                    }
                    Task::ExpandImport(import) => {
                        tracing::trace!("expand import");

                        let source_id = import.source_id;
                        let queue = &mut self.queue;

                        let result = import.process(&mut self.q, &mut |task| {
                            queue.try_push_back(task)?;
                            Ok(())
                        });

                        if let Err(error) = result {
                            self.q.diagnostics.error(source_id, error)?;
                        }
                    }
                    Task::ExpandWildcardImport(wildcard_import) => {
                        tracing::trace!("expand wildcard import");

                        let source_id = wildcard_import.location.source_id;

                        if let Err(error) = wildcard_imports.try_push(wildcard_import) {
                            self.q
                                .diagnostics
                                .error(source_id, compile::Error::from(error))?;
                        }
                    }
                }
            }

            // Process discovered wildcard imports, since they might be used
            // during impl items below.
            for mut wildcard_import in wildcard_imports.drain(..) {
                if let Err(error) = wildcard_import.process_local(&mut self.q) {
                    self.q
                        .diagnostics
                        .error(wildcard_import.location.source_id, error)?;
                }
            }

            // Expand impl items since they might be non-local. We need to look up the metadata associated with the item.
            while let Some(entry) = self.q.next_defer_entry() {
                match entry {
                    DeferEntry::ImplItem(item) => {
                        let source_id = item.location.source_id;

                        if let Err(error) = self.impl_item(item) {
                            self.q.diagnostics.error(source_id, error)?;
                        }
                    }
                    DeferEntry::ExpandMacroBuiltin(this) => {
                        let source_id = this.location.source_id;

                        if let Err(error) = self.expand_macro_builtin(this) {
                            self.q.diagnostics.error(source_id, error)?;
                        }
                    }
                    DeferEntry::ExpandMacroCall(this) => {
                        let source_id = this.location.source_id;

                        if let Err(error) = self.expand_macro_call(this) {
                            self.q.diagnostics.error(source_id, error)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn load_file(
        &mut self,
        kind: LoadFileKind,
        source_id: SourceId,
        mod_item: ModId,
        mod_item_id: ItemId,
    ) -> compile::Result<()> {
        let Some(source) = self.q.sources.get(source_id) else {
            self.q
                .diagnostics
                .internal(source_id, "Missing queued source by id")?;
            return Ok(());
        };

        let (root, is_module) = match kind {
            LoadFileKind::Root => (source.path().map(|p| p.try_to_owned()).transpose()?, false),
            LoadFileKind::Module { root } => (root, true),
        };

        macro_rules! indexer {
            ($tree:expr) => {{
                let item = self.q.pool.module_item(mod_item);
                let items = $crate::indexing::Items::new(item)?;

                tracing::trace!(?item, "load file: {}", item);

                $crate::indexing::Indexer {
                    q: self.q.borrow(),
                    root: root.as_deref(),
                    source_id,
                    items,
                    scopes: $crate::indexing::Scopes::new()?,
                    item: $crate::indexing::IndexItem::new(mod_item, mod_item_id),
                    nested_item: None,
                    macro_depth: 0,
                    loaded: Some(&mut self.loaded),
                    queue: Some(&mut self.queue),
                    tree: $tree,
                }
            }};
        }

        let as_function_body = self.q.options.script && !is_module;

        #[allow(clippy::collapsible_else_if)]
        if self.q.options.v2 {
            let tree = crate::grammar::text(source_id, source.as_str()).root()?;

            let tree = Rc::new(tree);

            #[cfg(feature = "std")]
            if self.q.options.print_tree {
                tree.print_with_source(
                    &Span::empty(),
                    format_args!("Loading file (source: {source_id})"),
                    source.as_str(),
                )?;
            }

            if as_function_body {
                let mut idx = indexer!(&tree);

                tree.parse_all(|p: &mut crate::grammar::Stream| index2::bare(&mut idx, p))?;
            } else {
                let mut idx = indexer!(&tree);
                tree.parse_all(|p| index2::file(&mut idx, p))?;
            }
        } else {
            if as_function_body {
                let ast =
                    crate::parse::parse_all::<ast::EmptyBlock>(source.as_str(), source_id, true)?;

                let span = Span::new(0, source.len());

                let empty = Rc::default();
                let mut idx = indexer!(&empty);

                index::empty_block_fn(&mut idx, ast, &span)?;
            } else {
                let mut ast =
                    crate::parse::parse_all::<ast::File>(source.as_str(), source_id, true)?;

                let empty = Rc::default();
                let mut idx = indexer!(&empty);
                index::file(&mut idx, &mut ast)?;
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn impl_item(&mut self, this: ImplItem) -> compile::Result<()> {
        macro_rules! indexer {
            ($tree:expr, $named:expr, $meta:expr) => {{
                let items =
                    $crate::indexing::Items::new({ self.q.pool.item($meta.item_meta.item) })?;

                $crate::indexing::Indexer {
                    q: self.q.borrow(),
                    root: this.root.as_deref(),
                    source_id: this.location.source_id,
                    items,
                    scopes: $crate::indexing::Scopes::new()?,
                    item: $crate::indexing::IndexItem::with_impl_item(
                        $named.module,
                        $named.item,
                        $meta.item_meta.item,
                    ),
                    nested_item: this.nested_item,
                    macro_depth: this.macro_depth,
                    loaded: Some(&mut self.loaded),
                    queue: Some(&mut self.queue),
                    tree: $tree,
                }
            }};
        }

        // When converting a path, we conservatively deny `Self` impl
        // since that is what Rust does, and at some point in the future
        // we might introduce bounds which would not be communicated
        // through `Self`.
        match this.kind {
            ImplItemKind::Ast { path, functions } => {
                let named = self
                    .q
                    .convert_path_with(&path, true, Used::Used, Used::Unused)?;

                if let Some((spanned, _)) = named.parameters.into_iter().flatten().next() {
                    return Err(compile::Error::new(
                        spanned.span(),
                        compile::ErrorKind::UnsupportedGenerics,
                    ));
                }

                tracing::trace!(item = ?self.q.pool.item(named.item), "next impl item entry");

                let meta = self.q.lookup_meta(
                    &this.location,
                    named.item,
                    GenericsParameters::default(),
                )?;

                let empty = Rc::default();
                let mut idx = indexer!(&empty, named, meta);

                for f in functions {
                    index::item_fn(&mut idx, f)?;
                }
            }
            ImplItemKind::Node { path, functions } => {
                let named =
                    path.parse(|p| self.q.convert_path2_with(p, true, Used::Used, Used::Unused))?;

                if let Some(spanned) = named.parameters.into_iter().flatten().next() {
                    return Err(compile::Error::new(
                        spanned.span(),
                        compile::ErrorKind::UnsupportedGenerics,
                    ));
                }

                tracing::trace!(item = ?self.q.pool.item(named.item), "next impl item entry");

                let meta = self.q.lookup_meta(
                    &this.location,
                    named.item,
                    GenericsParameters::default(),
                )?;

                let mut idx = indexer!(path.tree(), named, meta);

                for (id, attrs) in functions {
                    path.parse_id(id, |p| index2::item(&mut idx, p, attrs))?;
                }
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn expand_macro_builtin(&mut self, mut this: ExpandMacroBuiltin) -> compile::Result<()> {
        let (name, stream) = this.node.parse(|p| {
            let Some([ident]) = p.pump()?.nodes::<1>() else {
                return Err(compile::Error::msg(&*p, "missing macro identifier"));
            };

            let ident = ident.ast::<ast::Ident>()?;
            let name = ident.resolve(resolve_context!(self.q))?;

            p.expect(K![!])?;

            let close = match p.peek() {
                K!['{'] => K!['}'],
                K!['('] => K![')'],
                token => {
                    return Err(compile::Error::msg(
                        p.peek_span(),
                        try_format!("expected `{{` or `(`, found {token}"),
                    ));
                }
            };

            p.pump()?;
            let stream = p.expect(Kind::TokenStream)?;
            p.expect(close)?;

            Ok((name, stream))
        })?;

        let expanded = match name {
            "file" => stream.parse(|p| self.expand_file_macro(&this, p))?,
            "line" => stream.parse(|p| self.expand_line_macro(&this, p))?,
            "format" => self.expand_format_macro(&this, stream)?,
            "template" => {
                let literal = this.literal.take();
                self.expand_template_macro(&this, literal, stream)?
            }
            name => {
                return Err(compile::Error::msg(
                    &this.node,
                    try_format!("no internal macro named `{name}`"),
                ));
            }
        };

        let id = this.finish()?;

        self.q
            .insert_expanded_macro(id, ExpandedMacro::Builtin(expanded))?;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn expand_file_macro(
        &mut self,
        this: &ExpandMacroBuiltin,
        p: &mut Stream<'_>,
    ) -> compile::Result<BuiltInMacro2> {
        let name = self
            .q
            .sources
            .name(this.location.source_id)
            .ok_or_else(|| {
                compile::Error::new(
                    &*p,
                    compile::ErrorKind::MissingSourceId {
                        source_id: this.location.source_id,
                    },
                )
            })?;

        let id = self.q.storage.insert_str(name)?;

        let value = ast::LitStr {
            span: p.span(),
            source: ast::StrSource::Synthetic(id),
        };

        Ok(BuiltInMacro2::File(value))
    }

    fn expand_line_macro(
        &mut self,
        this: &ExpandMacroBuiltin,
        p: &mut Stream<'_>,
    ) -> compile::Result<BuiltInMacro2> {
        let (l, _) = self
            .q
            .sources
            .get(this.location.source_id)
            .map(|s| s.pos_to_utf8_linecol(p.span().start.into_usize()))
            .unwrap_or_default();

        Ok(BuiltInMacro2::Line(l + 1))
    }

    fn expand_format_macro(
        &mut self,
        this: &ExpandMacroBuiltin,
        stream: Node<'_>,
    ) -> compile::Result<BuiltInMacro2> {
        let tree = crate::grammar::node(stream).format()?;
        let tree = Rc::new(tree);

        let items = crate::indexing::Items::new(self.q.pool.item(this.item.id))?;

        let mut idx = crate::indexing::Indexer {
            q: self.q.borrow(),
            root: this.root.as_deref(),
            source_id: this.location.source_id,
            items,
            scopes: crate::indexing::Scopes::new()?,
            item: this.item,
            nested_item: None,
            macro_depth: this.macro_depth + 1,
            loaded: Some(&mut self.loaded),
            queue: Some(&mut self.queue),
            tree: &tree,
        };

        tree.parse_all(|p| index2::any(&mut idx, p))?;

        #[cfg(feature = "std")]
        if self.q.options.print_tree {
            tree.print(
                &this.node,
                format_args!("Expanded format!() macro {}", this.id),
            )?;
        }

        Ok(BuiltInMacro2::Format(tree))
    }

    fn expand_template_macro(
        &mut self,
        this: &ExpandMacroBuiltin,
        literal: BuiltInLiteral,
        stream: Node<'_>,
    ) -> compile::Result<BuiltInMacro2> {
        let tree = crate::grammar::node(stream).exprs(K![,])?;
        let tree = Rc::new(tree);

        let items = crate::indexing::Items::new(self.q.pool.item(this.item.id))?;

        let mut idx = crate::indexing::Indexer {
            q: self.q.borrow(),
            root: this.root.as_deref(),
            source_id: this.location.source_id,
            items,
            scopes: crate::indexing::Scopes::new()?,
            item: this.item,
            nested_item: None,
            macro_depth: this.macro_depth + 1,
            loaded: Some(&mut self.loaded),
            queue: Some(&mut self.queue),
            tree: &tree,
        };

        tree.parse_all(|p| index2::any(&mut idx, p))?;

        #[cfg(feature = "std")]
        if self.q.options.print_tree {
            tree.print(
                &this.node,
                format_args!("Expanded template!() macro {}", this.id),
            )?;
        }

        Ok(BuiltInMacro2::Template(tree, literal))
    }

    #[tracing::instrument(skip_all)]
    fn expand_macro_call(&mut self, this: ExpandMacroBuiltin) -> compile::Result<()> {
        if this.macro_depth >= self.q.options.max_macro_depth {
            return Err(compile::Error::new(
                this.node.span(),
                compile::ErrorKind::MaxMacroRecursion {
                    depth: this.macro_depth,
                    max: self.q.options.max_macro_depth,
                },
            ));
        }

        let item_meta = self
            .q
            .item_for("macro call", this.item.id)
            .with_span(&this.node)?;

        let (named, stream) = this.node.parse(|p| {
            let named = p
                .pump()?
                .parse(|p| self.q.convert_path2_with(p, true, Used::Used, Used::Unused))?;

            p.expect(K![!])?;

            let close = match p.peek() {
                K!['{'] => K!['}'],
                K!['('] => K![')'],
                token => {
                    return Err(compile::Error::msg(
                        p.peek_span(),
                        try_format!("expected `{{` or `(`, found {token}"),
                    ));
                }
            };

            p.pump()?;
            let stream = p.expect(Kind::TokenStream)?;
            p.expect(close)?;

            Ok((named, stream))
        })?;

        if let Some(spanned) = named.parameters.into_iter().flatten().next() {
            return Err(compile::Error::new(
                spanned.span(),
                compile::ErrorKind::UnsupportedGenerics,
            ));
        }

        let hash = self.q.pool.item_type_hash(named.item);

        let Some(handler) = self.q.context.lookup_macro(hash) else {
            return Err(compile::Error::new(
                &this.node,
                compile::ErrorKind::MissingMacro {
                    item: self.q.pool.item(named.item).try_to_owned()?,
                },
            ));
        };

        let items = crate::indexing::Items::new(self.q.pool.item(this.item.id))?;

        let mut idx = crate::indexing::Indexer {
            q: self.q.borrow(),
            root: this.root.as_deref(),
            source_id: this.location.source_id,
            items,
            scopes: crate::indexing::Scopes::new()?,
            item: this.item,
            nested_item: None,
            macro_depth: this.macro_depth + 1,
            loaded: Some(&mut self.loaded),
            queue: Some(&mut self.queue),
            tree: this.node.tree(),
        };

        let mut input_stream = TokenStream::new();

        for node in stream
            .children()
            .flat_map(|c| c.walk())
            .filter(|n| n.is_empty())
        {
            input_stream.push(node.token())?;
        }

        let output_stream = {
            let mut macro_context = MacroContext {
                macro_span: this.node.span(),
                input_span: stream.span(),
                item_meta,
                idx: &mut idx,
            };

            handler(&mut macro_context, &input_stream)?
        };

        let inner_tree = crate::grammar::token_stream(&output_stream).root()?;

        let tree = Rc::new(inner_tree);
        idx.tree = &tree;
        tree.parse_all(|p| index2::any(&mut idx, p))?;

        #[cfg(feature = "std")]
        if self.q.options.print_tree {
            tree.print(
                &this.node,
                format_args!("Expanded macro {} from output stream", this.id),
            )?;
        }

        let id = this.finish()?;
        self.q
            .insert_expanded_macro(id, ExpandedMacro::Tree(tree))?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ImportKind {
    /// The import is in-place.
    Local,
    /// The import is deferred.
    Global,
}
