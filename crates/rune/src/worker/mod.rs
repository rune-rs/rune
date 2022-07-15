//! Worker used by compiler.

use crate::ast;
use crate::ast::Span;
use crate::collections::HashMap;
use crate::compile::{CompileVisitor, Item, Options, Prelude, SourceLoader, UnitBuilder};
use crate::indexing::index;
use crate::indexing::{IndexScopes, Indexer};
use crate::macros::Storage;
use crate::query::{Query, QueryInner};
use crate::shared::{Consts, Gen, Items};
use crate::{Context, Diagnostics, SourceId, Sources};
use std::collections::VecDeque;

mod import;
mod task;
mod wildcard_import;

pub(crate) use self::import::Import;
pub(crate) use self::task::{LoadFileKind, Task};
pub(crate) use self::wildcard_import::WildcardImport;

pub(crate) struct Worker<'a> {
    context: &'a Context,
    options: &'a Options,
    pub(crate) diagnostics: &'a mut Diagnostics,
    pub(crate) source_loader: &'a mut dyn SourceLoader,
    /// Query engine.
    pub(crate) q: Query<'a>,
    /// Id generator.
    pub(crate) gen: &'a Gen,
    /// Files that have been loaded.
    pub(crate) loaded: HashMap<Item, (SourceId, Span)>,
    /// Worker queue.
    pub(crate) queue: VecDeque<Task>,
}

impl<'a> Worker<'a> {
    /// Construct a new worker.
    pub(crate) fn new(
        context: &'a Context,
        consts: &'a mut Consts,
        storage: &'a mut Storage,
        sources: &'a mut Sources,
        options: &'a Options,
        unit: &'a mut UnitBuilder,
        prelude: &'a Prelude,
        diagnostics: &'a mut Diagnostics,
        visitor: &'a mut dyn CompileVisitor,
        source_loader: &'a mut dyn SourceLoader,
        gen: &'a Gen,
        inner: &'a mut QueryInner,
    ) -> Self {
        Self {
            context,
            options,
            diagnostics,
            source_loader,
            q: Query::new(unit, prelude, consts, storage, sources, visitor, gen, inner),
            gen,
            loaded: HashMap::new(),
            queue: VecDeque::new(),
        }
    }

    /// Run the worker until the task queue is empty.
    pub(crate) fn run(&mut self) {
        // NB: defer wildcard expansion until all other imports have been
        // indexed.
        let mut wildcard_imports = Vec::new();

        while let Some(task) = self.queue.pop_front() {
            match task {
                Task::LoadFile {
                    kind,
                    source_id,
                    mod_item,
                } => {
                    tracing::trace!("load file: {}", mod_item.item);

                    let source = match self.q.sources.get(source_id) {
                        Some(source) => source,
                        None => {
                            self.diagnostics
                                .internal(source_id, "missing queued source by id");
                            continue;
                        }
                    };

                    let mut file = match crate::parse::parse_all::<ast::File>(
                        source.as_str(),
                        source_id,
                        true,
                    ) {
                        Ok(file) => file,
                        Err(error) => {
                            self.diagnostics.error(source_id, error);
                            continue;
                        }
                    };

                    let root = match kind {
                        LoadFileKind::Root => source.path().map(ToOwned::to_owned),
                        LoadFileKind::Module { root } => root,
                    };

                    tracing::trace!("load file: {}", mod_item.item);
                    let items = Items::new(mod_item.item.clone(), self.gen);

                    let mut indexer = Indexer {
                        root,
                        loaded: &mut self.loaded,
                        q: self.q.borrow(),
                        queue: &mut self.queue,
                        context: self.context,
                        options: self.options,
                        source_id,
                        diagnostics: self.diagnostics,
                        items,
                        scopes: IndexScopes::new(),
                        mod_item,
                        impl_item: Default::default(),
                        source_loader: self.source_loader,
                        nested_item: None,
                    };

                    if let Err(error) = index::file(&mut file, &mut indexer) {
                        indexer.diagnostics.error(source_id, error);
                    }
                }
                Task::ExpandImport(import) => {
                    tracing::trace!("expand import");

                    let source_id = import.source_id;
                    let queue = &mut self.queue;

                    let result = import.process(self.context, &mut self.q, &mut |task| {
                        queue.push_back(task);
                    });

                    if let Err(error) = result {
                        self.diagnostics.error(source_id, error);
                    }
                }
                Task::ExpandWildcardImport(wildcard_import) => {
                    tracing::trace!("expand wildcard import");

                    wildcard_imports.push(wildcard_import);
                }
            }
        }

        for wildcard_import in wildcard_imports {
            let source_id = wildcard_import.source_id;

            if let Err(error) = wildcard_import.process_local(&mut self.q) {
                self.diagnostics.error(source_id, error);
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ImportKind {
    /// The import is in-place.
    Local,
    /// The import is deferred.
    Global,
}
