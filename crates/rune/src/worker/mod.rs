//! Worker used by compiler.

use crate::ast;
use crate::collections::HashMap;
use crate::indexing::{Index as _, IndexScopes, Indexer};
use crate::query::Query;
use crate::shared::{Consts, Gen, Items};
use crate::{CompileVisitor, Diagnostics, Options, SourceLoader, Sources, Storage, UnitBuilder};
use runestick::{Context, Item, SourceId, Span};
use std::collections::VecDeque;
use std::rc::Rc;

mod import;
mod task;
mod wildcard_import;

pub(crate) use self::import::Import;
pub(crate) use self::task::{LoadFileKind, Task};
pub(crate) use self::wildcard_import::WildcardImport;

pub(crate) struct Worker<'a> {
    context: &'a Context,
    pub(crate) sources: &'a mut Sources,
    options: &'a Options,
    pub(crate) diagnostics: &'a mut Diagnostics,
    pub(crate) visitor: Rc<dyn CompileVisitor>,
    pub(crate) source_loader: Rc<dyn SourceLoader + 'a>,
    /// Constants storage.
    pub(crate) consts: Consts,
    /// Worker queue.
    pub(crate) queue: VecDeque<Task>,
    /// Query engine.
    pub(crate) query: Query,
    /// Macro storage.
    pub(crate) storage: Storage,
    /// Id generator.
    pub(crate) gen: Gen,
    /// Files that have been loaded.
    pub(crate) loaded: HashMap<Item, (SourceId, Span)>,
}

impl<'a> Worker<'a> {
    /// Construct a new worker.
    pub(crate) fn new(
        context: &'a Context,
        sources: &'a mut Sources,
        options: &'a Options,
        unit: UnitBuilder,
        consts: Consts,
        diagnostics: &'a mut Diagnostics,
        visitor: Rc<dyn CompileVisitor>,
        source_loader: Rc<dyn SourceLoader + 'a>,
        storage: Storage,
        gen: Gen,
    ) -> Self {
        Self {
            context,
            sources,
            options,
            diagnostics,
            visitor: visitor.clone(),
            source_loader,
            consts: consts.clone(),
            queue: VecDeque::new(),
            query: Query::new(visitor, storage.clone(), unit, consts, gen.clone()),
            storage,
            gen,
            loaded: HashMap::new(),
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
                    log::trace!("load file: {}", mod_item.item);

                    let source = match self.sources.get(source_id).cloned() {
                        Some(source) => source,
                        None => {
                            self.diagnostics
                                .internal(source_id, "missing queued source by id");
                            continue;
                        }
                    };

                    let mut file = match crate::parse_all::<ast::File>(source.as_str()) {
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

                    log::trace!("index: {}", mod_item.item);
                    let items = Items::new(mod_item.item.clone(), self.gen.clone());

                    let mut indexer = Indexer {
                        root,
                        storage: self.query.storage(),
                        loaded: &mut self.loaded,
                        consts: self.consts.clone(),
                        query: self.query.clone(),
                        queue: &mut self.queue,
                        sources: self.sources,
                        context: self.context,
                        options: self.options,
                        source_id,
                        source,
                        diagnostics: self.diagnostics,
                        items,
                        scopes: IndexScopes::new(),
                        mod_item,
                        impl_item: Default::default(),
                        visitor: self.visitor.clone(),
                        source_loader: self.source_loader.clone(),
                        nested_item: None,
                    };

                    if let Err(error) = file.index(&mut indexer) {
                        indexer.diagnostics.error(source_id, error);
                    }
                }
                Task::ExpandImport(import) => {
                    let source_id = import.source_id;
                    let queue = &mut self.queue;

                    let result =
                        import.process(self.context, &self.storage, &self.query, &mut |task| {
                            queue.push_back(task);
                        });

                    if let Err(error) = result {
                        self.diagnostics.error(source_id, error);
                    }
                }
                Task::ExpandWildcardImport(wildcard_import) => {
                    wildcard_imports.push(wildcard_import);
                }
            }
        }

        for wildcard_import in wildcard_imports {
            let source_id = wildcard_import.source_id;

            if let Err(error) = wildcard_import.process_local(&self.query) {
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
