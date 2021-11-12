//! Worker used by compiler.

use crate::ast;
use crate::collections::HashMap;
use crate::compile::{CompileVisitor, SourceLoader, UnitBuilder};
use crate::indexing::{Index, IndexScopes, Indexer};
use crate::query::Query;
use crate::shared::{Gen, Items};
use crate::{Context, Diagnostics, Item, Options, SourceId, Sources, Span};
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
    options: &'a Options,
    pub(crate) diagnostics: &'a mut Diagnostics,
    pub(crate) visitor: Rc<dyn CompileVisitor>,
    pub(crate) source_loader: Rc<dyn SourceLoader + 'a>,
    /// Worker queue.
    pub(crate) queue: VecDeque<Task>,
    /// Query engine.
    pub(crate) q: Query<'a>,
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
        unit: &'a mut UnitBuilder,
        diagnostics: &'a mut Diagnostics,
        visitor: Rc<dyn CompileVisitor>,
        source_loader: Rc<dyn SourceLoader + 'a>,
        gen: Gen,
    ) -> Self {
        Self {
            context,
            options,
            diagnostics,
            visitor: visitor.clone(),
            source_loader,
            queue: VecDeque::new(),
            q: Query::new(unit, sources, visitor, gen.clone()),
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

                    let source = match self.q.sources.get(source_id) {
                        Some(source) => source,
                        None => {
                            self.diagnostics
                                .internal(source_id, "missing queued source by id");
                            continue;
                        }
                    };

                    let mut file =
                        match crate::parse::parse_all::<ast::File>(source.as_str(), source_id) {
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
                        loaded: &mut self.loaded,
                        q: &mut self.q,
                        queue: &mut self.queue,
                        context: self.context,
                        options: self.options,
                        source_id,
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

                    let result = import.process(self.context, &mut self.q, &mut |task| {
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
