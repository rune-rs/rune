//! Worker used by compiler.

use crate::no_std::prelude::*;

use crate::no_std::collections::HashMap;
use crate::no_std::collections::VecDeque;

use crate::ast;
use crate::ast::Span;
use crate::compile::ModId;
use crate::indexing::index;
use crate::indexing::items::Items;
use crate::indexing::{Indexer, Scopes};
use crate::query::Query;
use crate::SourceId;

mod import;
mod task;
mod wildcard_import;

pub(crate) use self::import::Import;
pub(crate) use self::task::{LoadFileKind, Task};
pub(crate) use self::wildcard_import::WildcardImport;

pub(crate) struct Worker<'a> {
    /// Query engine.
    pub(crate) q: Query<'a>,
    /// Files that have been loaded.
    pub(crate) loaded: HashMap<ModId, (SourceId, Span)>,
    /// Worker queue.
    pub(crate) queue: VecDeque<Task>,
}

impl<'a> Worker<'a> {
    /// Construct a new worker.
    pub(crate) fn new(q: Query<'a>) -> Self {
        Self {
            q,
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
                    mod_item_id,
                } => {
                    let item = self.q.pool.module_item(mod_item);
                    tracing::trace!("load file: {}", item);

                    let source = match self.q.sources.get(source_id) {
                        Some(source) => source,
                        None => {
                            self.q
                                .diagnostics
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
                            self.q.diagnostics.error(source_id, error);
                            continue;
                        }
                    };

                    let root = match kind {
                        LoadFileKind::Root => source.path().map(ToOwned::to_owned),
                        LoadFileKind::Module { root } => root,
                    };

                    let items = Items::new(item, mod_item_id, self.q.gen);

                    let mut idx = Indexer {
                        q: self.q.borrow(),
                        root,
                        source_id,
                        items,
                        scopes: Scopes::default(),
                        mod_item,
                        impl_item: Default::default(),
                        nested_item: None,
                        macro_depth: 0,
                        loaded: Some(&mut self.loaded),
                        queue: Some(&mut self.queue),
                    };

                    if let Err(error) = index::file(&mut idx, &mut file) {
                        idx.q.diagnostics.error(source_id, error);
                    }
                }
                Task::ExpandImport(import) => {
                    tracing::trace!("expand import");

                    let source_id = import.source_id;
                    let queue = &mut self.queue;

                    let result = import.process(&mut self.q, &mut |task| {
                        queue.push_back(task);
                    });

                    if let Err(error) = result {
                        self.q.diagnostics.error(source_id, error);
                    }
                }
                Task::ExpandWildcardImport(wildcard_import) => {
                    tracing::trace!("expand wildcard import");

                    wildcard_imports.push(wildcard_import);
                }
            }
        }

        for mut wildcard_import in wildcard_imports {
            if let Err(error) = wildcard_import.process_local(&mut self.q) {
                self.q
                    .diagnostics
                    .error(wildcard_import.location.source_id, error);
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
