//! Worker used by compiler.

use crate::ast;
use crate::collections::HashMap;
use crate::indexing::{Index as _, IndexScopes, Indexer};
use crate::query::{IndexedEntry, Query};
use crate::shared::{Consts, Items};
use crate::CompileResult;
use crate::{
    CompileError, CompileErrorKind, CompileVisitor, Error, Errors, Options, Resolve as _,
    SourceLoader, Sources, Spanned as _, Storage, UnitBuilder, Warnings,
};
use runestick::{Context, Item, Source, SourceId, Span};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

/// A single task that can be fed to the worker.
#[derive(Debug)]
pub(crate) enum Task {
    /// Load a file.
    LoadFile {
        /// The kind of loaded file.
        kind: LoadFileKind,
        /// The item of the file to load.
        item: Item,
        /// The source id of the item being loaded.
        source_id: SourceId,
    },
}

/// The kind of the loaded module.
#[derive(Debug)]
pub(crate) enum LoadFileKind {
    /// A root file, which determined a URL root.
    Root,
    /// A loaded module, which inherits its root from the file it was loaded
    /// from.
    Module { root: Option<PathBuf> },
}

pub(crate) struct Worker<'a> {
    pub(crate) queue: VecDeque<Task>,
    context: &'a Context,
    pub(crate) sources: &'a mut Sources,
    options: &'a Options,
    pub(crate) errors: &'a mut Errors,
    pub(crate) warnings: &'a mut Warnings,
    pub(crate) visitor: &'a mut dyn CompileVisitor,
    pub(crate) source_loader: &'a mut dyn SourceLoader,
    pub(crate) query: Query,
    pub(crate) loaded: HashMap<Item, (SourceId, Span)>,
}

impl<'a> Worker<'a> {
    /// Construct a new worker.
    pub(crate) fn new(
        queue: VecDeque<Task>,
        context: &'a Context,
        sources: &'a mut Sources,
        options: &'a Options,
        unit: UnitBuilder,
        consts: Consts,
        errors: &'a mut Errors,
        warnings: &'a mut Warnings,
        visitor: &'a mut dyn CompileVisitor,
        source_loader: &'a mut dyn SourceLoader,
        storage: Storage,
    ) -> Self {
        Self {
            queue,
            context,
            sources,
            options,
            errors,
            warnings,
            visitor,
            source_loader,
            query: Query::new(storage, unit, consts),
            loaded: HashMap::new(),
        }
    }

    /// Run the worker until the task queue is empty.
    pub(crate) fn run(&mut self) {
        while let Some(task) = self.queue.pop_front() {
            match task {
                Task::LoadFile {
                    kind,
                    item,
                    source_id,
                } => {
                    log::trace!("load file: {}", item);

                    let source = match self.sources.get(source_id).cloned() {
                        Some(source) => source,
                        None => {
                            self.errors
                                .push(Error::internal(source_id, "missing queued source by id"));

                            continue;
                        }
                    };

                    let mut file = match crate::parse_all::<ast::File>(source.as_str()) {
                        Ok(file) => file,
                        Err(error) => {
                            self.errors.push(Error::new(source_id, error));

                            continue;
                        }
                    };

                    let root = match kind {
                        LoadFileKind::Root => source.path().map(ToOwned::to_owned),
                        LoadFileKind::Module { root } => root,
                    };

                    let items = Items::new(item.clone().into_vec());

                    log::trace!("index: {}", item);

                    let mut indexer = Indexer {
                        root,
                        storage: self.query.storage.clone(),
                        loaded: &mut self.loaded,
                        query: &mut self.query,
                        queue: &mut self.queue,
                        sources: self.sources,
                        context: self.context,
                        options: self.options,
                        source_id,
                        source,
                        warnings: self.warnings,
                        items,
                        scopes: IndexScopes::new(),
                        impl_items: Default::default(),
                        visitor: self.visitor,
                        source_loader: self.source_loader,
                    };

                    if let Err(error) = indexer.index(&mut file) {
                        self.errors.push(Error::new(source_id, error));
                        continue;
                    }
                }
            }
        }
    }
}

/// Import to process.
#[derive(Debug)]
pub(crate) struct Import {
    pub(crate) item: Item,
    pub(crate) ast: ast::ItemUse,
    pub(crate) source: Arc<Source>,
    pub(crate) source_id: usize,
}

impl Import {
    /// Process the import, populating the unit.
    pub(crate) fn process(
        self,
        context: &Context,
        storage: &Storage,
        unit: &UnitBuilder,
    ) -> CompileResult<()> {
        let Self {
            item,
            ast: decl_use,
            source,
            source_id,
        } = self;

        let span = decl_use.span();

        let name = unit.convert_path(&item, &decl_use.path, storage, &*source)?;

        if decl_use.is_wildcard() {
            if !context.contains_prefix(&name) && !unit.contains_prefix(&name) {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::MissingModule { item: name },
                ));
            }

            let iter = context
                .iter_components(name.iter())
                .chain(unit.iter_components(&name));

            for c in iter {
                let name = name.iter().chain(std::iter::once(c));
                unit.new_import(item.clone(), name, span, source_id)?;
            }
        } else {
            unit.new_import(item, &name, span, source_id)?;
        }

        Ok(())
    }
}
