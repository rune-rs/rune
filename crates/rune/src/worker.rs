//! Worker used by compiler.

use crate::ast;
use crate::collections::HashMap;
use crate::indexing::{Index as _, IndexScopes, Indexer, Visibility};
use crate::query::{Query, QueryMod};
use crate::shared::{Consts, Items};
use crate::CompileResult;
use crate::{
    CompileError, CompileErrorKind, CompileVisitor, Error, Errors, Options, Resolve as _,
    SourceLoader, Sources, Spanned as _, Storage, UnitBuilder, Warnings,
};
use runestick::{Context, Item, Source, SourceId, Span};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

/// A single task that can be fed to the worker.
#[derive(Debug)]
pub(crate) enum Task {
    /// Load a file.
    LoadFile {
        /// The kind of loaded file.
        kind: LoadFileKind,
        /// The source id of the item being loaded.
        source_id: SourceId,
        /// The item of the file to load.
        mod_item: Rc<QueryMod>,
    },
    /// Deferred action, since it requires all modules to be loaded to be able
    /// to discover all modules.
    ExpandUnitWildcard(ExpandUnitWildcard),
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
    context: &'a Context,
    pub(crate) sources: &'a mut Sources,
    options: &'a Options,
    pub(crate) errors: &'a mut Errors,
    pub(crate) warnings: &'a mut Warnings,
    pub(crate) visitor: &'a mut dyn CompileVisitor,
    pub(crate) source_loader: &'a mut dyn SourceLoader,
    /// Constants storage.
    pub(crate) consts: Consts,
    /// Worker queue.
    pub(crate) queue: VecDeque<Task>,
    /// Query engine.
    pub(crate) query: Query,
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
        errors: &'a mut Errors,
        warnings: &'a mut Warnings,
        visitor: &'a mut dyn CompileVisitor,
        source_loader: &'a mut dyn SourceLoader,
        storage: Storage,
    ) -> Self {
        Self {
            context,
            sources,
            options,
            errors,
            warnings,
            visitor,
            source_loader,
            consts: consts.clone(),
            queue: VecDeque::new(),
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
                    source_id,
                    mod_item,
                } => {
                    log::trace!("load file: {}", mod_item.item);

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

                    log::trace!("index: {}", mod_item.item);
                    let items = Items::new(mod_item.item.clone());

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
                        warnings: self.warnings,
                        items,
                        scopes: IndexScopes::new(),
                        mod_item,
                        impl_item: Default::default(),
                        visitor: self.visitor,
                        source_loader: self.source_loader,
                    };

                    if let Err(error) = indexer.index(&mut file) {
                        self.errors.push(Error::new(source_id, error));
                    }
                }
                Task::ExpandUnitWildcard(expander) => {
                    let source_id = expander.source_id;

                    if let Err(error) = expander.expand(&mut self.query) {
                        self.errors.push(Error::new(source_id, error));
                    }
                }
            }
        }
    }
}

/// Import to process.
#[derive(Debug)]
pub(crate) struct Import<'a> {
    pub(crate) visibility: Visibility,
    pub(crate) item: &'a Item,
    pub(crate) source: &'a Arc<Source>,
    pub(crate) source_id: usize,
    pub(crate) ast: Box<ast::ItemUse>,
}

impl Import<'_> {
    /// Process the import, populating the unit.
    pub(crate) fn process(
        self,
        mod_item: &Rc<QueryMod>,
        context: &Context,
        storage: &Storage,
        query: &Query,
        mut wildcard_expand: impl FnMut(ExpandUnitWildcard),
    ) -> CompileResult<()> {
        let mut queue = VecDeque::new();
        queue.push_back((Item::new(), &self.ast.path, true));

        while let Some((mut name, path, mut initial)) = queue.pop_front() {
            if let Some(global) = &path.global {
                return Err(CompileError::internal(
                    global,
                    "global imports are not supported",
                ));
            }

            let span = path.span();

            let mut it = Some(&path.first)
                .into_iter()
                .chain(path.segments.iter().map(|(_, s)| s));

            let complete = loop {
                let segment = match it.next() {
                    Some(segment) => segment,
                    None => break None,
                };

                // Only the first ever segment loaded counts as the initial
                // segment.
                let initial = std::mem::take(&mut initial);

                match segment {
                    ast::ItemUseSegment::PathSegment(segment) => match segment {
                        ast::PathSegment::SelfType(..) => {
                            return Err(CompileError::new(
                                path,
                                CompileErrorKind::ExpectedLeadingPathSegment,
                            ));
                        }
                        ast::PathSegment::SelfValue(self_type) => {
                            if !initial {
                                return Err(CompileError::new(
                                    self_type,
                                    CompileErrorKind::ExpectedLeadingPathSegment,
                                ));
                            }

                            name = mod_item.item.clone();
                        }
                        ast::PathSegment::Ident(ident) => {
                            if initial {
                                let ident = ident.resolve(storage, self.source)?;
                                name = Item::of(&[ident.as_ref()]);
                            } else {
                                name.push(ident.resolve(storage, self.source)?);
                            }
                        }
                        ast::PathSegment::Crate(crate_token) => {
                            if !initial {
                                return Err(CompileError::new(
                                    crate_token,
                                    CompileErrorKind::ExpectedLeadingPathSegment,
                                ));
                            }

                            name = Item::new();
                        }
                        ast::PathSegment::Super(super_token) => {
                            if initial {
                                name = mod_item.item.clone();
                            }

                            name.pop().ok_or_else(|| {
                                CompileError::new(super_token, CompileErrorKind::UnsupportedSuper)
                            })?;
                        }
                    },
                    ast::ItemUseSegment::Wildcard(star_token) => {
                        let was_in_context = if context.contains_prefix(&name) {
                            for c in context.iter_components(&name) {
                                query.insert_import(
                                    self.source_id,
                                    span,
                                    &self.source,
                                    mod_item,
                                    self.visibility,
                                    self.item.clone(),
                                    name.extended(c),
                                    None::<&str>,
                                    true,
                                )?;
                            }

                            true
                        } else {
                            false
                        };

                        let wildcard_expander = ExpandUnitWildcard {
                            visibility: self.visibility,
                            from: self.item.clone(),
                            name: name.clone(),
                            span,
                            source_id: self.source_id,
                            source: self.source.clone(),
                            was_in_context,
                            mod_item: mod_item.clone(),
                        };

                        wildcard_expand(wildcard_expander);
                        break Some(star_token.span());
                    }
                    ast::ItemUseSegment::Group(group) => {
                        for (path, _) in group {
                            queue.push_back((name.clone(), path, initial));
                        }

                        break Some(group.span());
                    }
                }
            };

            if let Some(segment) = it.next() {
                return Err(CompileError::new(
                    segment,
                    CompileErrorKind::IllegalUseSegment,
                ));
            }

            let alias = match path.alias {
                Some((_, ident)) => {
                    if let Some(span) = complete {
                        return Err(CompileError::new(
                            span.join(ident.span()),
                            CompileErrorKind::UseAliasNotSupported,
                        ));
                    }

                    Some(ident.resolve(storage, self.source)?)
                }
                None => None,
            };

            if complete.is_none() {
                query.insert_import(
                    self.source_id,
                    span,
                    &self.source,
                    mod_item,
                    self.visibility,
                    self.item.clone(),
                    name,
                    alias.as_deref(),
                    false,
                )?;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct ExpandUnitWildcard {
    visibility: Visibility,
    from: Item,
    name: Item,
    source_id: SourceId,
    source: Arc<Source>,
    span: Span,
    /// Indicates if any wildcards were expanded from context.
    was_in_context: bool,
    mod_item: Rc<QueryMod>,
}

impl ExpandUnitWildcard {
    pub(crate) fn expand(self, query: &mut Query) -> CompileResult<()> {
        if query.contains_prefix(&self.name) {
            let components = query.iter_components(&self.name);

            for c in components {
                let name = self.name.extended(c);

                query.insert_import(
                    self.source_id,
                    self.span,
                    &self.source,
                    &self.mod_item,
                    self.visibility,
                    self.from.clone(),
                    name,
                    None::<&str>,
                    true,
                )?;
            }

            return Ok(());
        }

        if !self.was_in_context {
            return Err(CompileError::new(
                self.span,
                CompileErrorKind::MissingItem { item: self.name },
            ));
        }

        Ok(())
    }
}
