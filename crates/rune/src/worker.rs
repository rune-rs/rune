//! Worker used by compiler.

use runestick::{Component, Context, Item, Source, Span};

use crate::ast;
use crate::collections::HashMap;
use crate::error::CompileResult;
use crate::index::{Index, Indexer};
use crate::index_scopes::IndexScopes;
use crate::items::Items;
use crate::macros::MacroCompiler;
use crate::query::Query;
use crate::{
    CompileError, LoadError, LoadErrorKind, MacroContext, Options, Parse, Resolve as _, SourceId,
    Sources, Storage, UnitBuilder, Warnings,
};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Arc;

/// A single task that can be fed to the worker.
#[derive(Debug)]
pub(crate) enum Task {
    /// An indexing task, which will index the specified item.
    Index {
        /// Item being built.
        item: Item,
        /// Path to index.
        items: Items,
        /// The source id where the item came from.
        source_id: SourceId,
        /// The source where the item came from.
        source: Arc<Source>,
        scopes: IndexScopes,
        impl_items: Vec<Item>,
        ast: IndexAst,
    },
    /// Task to process an import.
    Import(Import),
    /// Task to expand a macro. This might produce additional indexing tasks.
    ExpandMacro(Macro),
}

#[derive(Debug)]
pub(crate) enum IndexAst {
    /// Index the root of a file with the given item.
    File(ast::File),
    /// Index an item.
    Item(ast::Item),
    /// Index a new expression.
    Expr(ast::Expr),
}

pub(crate) struct Worker<'a> {
    pub(crate) queue: VecDeque<Task>,
    context: &'a Context,
    pub(crate) sources: &'a mut Sources,
    options: &'a Options,
    pub(crate) warnings: &'a mut Warnings,
    pub(crate) query: Query,
    pub(crate) loaded: HashMap<Item, (SourceId, Span)>,
    pub(crate) expanded: HashMap<Item, Expanded>,
}

impl<'a> Worker<'a> {
    /// Construct a new worker.
    pub(crate) fn new(
        queue: VecDeque<Task>,
        context: &'a Context,
        sources: &'a mut Sources,
        options: &'a Options,
        unit: Rc<RefCell<UnitBuilder>>,
        warnings: &'a mut Warnings,
        storage: Storage,
    ) -> Self {
        Self {
            queue,
            context,
            sources,
            options,
            warnings,
            query: Query::new(storage, unit),
            loaded: HashMap::new(),
            expanded: HashMap::new(),
        }
    }

    /// Run the worker until the task queue is empty.
    pub(crate) fn run(&mut self) -> Result<(), LoadError> {
        while let Some(task) = self.queue.pop_front() {
            match task {
                Task::Index {
                    item,
                    items,
                    source_id,
                    source,
                    scopes,
                    impl_items,
                    ast,
                } => {
                    log::trace!("index: {}", item);

                    let mut indexer = Indexer {
                        storage: self.query.storage.clone(),
                        loaded: &mut self.loaded,
                        query: &mut self.query,
                        queue: &mut self.queue,
                        sources: self.sources,
                        source_id,
                        source,
                        warnings: self.warnings,
                        items,
                        scopes,
                        impl_items,
                    };

                    let result = match ast {
                        IndexAst::File(ast) => match indexer.index(&ast) {
                            Ok(()) => Ok(None),
                            Err(error) => Err(error),
                        },
                        IndexAst::Item(ast) => match indexer.index(&ast) {
                            Ok(()) => Ok(None),
                            Err(error) => Err(error),
                        },
                        IndexAst::Expr(ast) => match indexer.index(&ast) {
                            Ok(()) => Ok(Some(Expanded::Expr(ast))),
                            Err(error) => Err(error),
                        },
                    };

                    match result {
                        Ok(expanded) => {
                            if let Some(expanded) = expanded {
                                self.expanded.insert(item, expanded);
                            }
                        }
                        Err(error) => {
                            return Err(LoadError::from(LoadErrorKind::CompileError {
                                source_id,
                                error,
                            }));
                        }
                    }
                }
                Task::Import(import) => {
                    log::trace!("import: {}", import.item);

                    let source_id = import.source_id;

                    if let Err(error) = import.process(
                        self.context,
                        &self.query.storage,
                        &mut *self.query.unit.borrow_mut(),
                    ) {
                        return Err(LoadError::from(LoadErrorKind::CompileError {
                            error,
                            source_id,
                        }));
                    }
                }
                Task::ExpandMacro(m) => {
                    let Macro {
                        items,
                        ast,
                        source,
                        source_id,
                        scopes,
                        impl_items,
                        kind,
                    } = m;

                    let item = items.item();
                    let span = ast.span();
                    log::trace!("expanding macro: {}", item);

                    match kind {
                        MacroKind::Expr => (),
                        MacroKind::Item => {
                            // NB: item macros are not expanded into the second
                            // compiler phase (only indexed), so we need to
                            // restore their item position so that indexing is
                            // done on the correct item.
                            match items.pop() {
                                Some(Component::Macro(..)) => (),
                                _ => return Err(LoadError::from(LoadErrorKind::CompileError {
                                    source_id,
                                    error: CompileError::internal(
                                        "expected macro item as last component of macro expansion",
                                        span,
                                    ),
                                })),
                            }
                        }
                    }

                    let mut macro_context =
                        MacroContext::new(self.query.storage.clone(), source.clone());

                    let compiler = MacroCompiler {
                        storage: self.query.storage.clone(),
                        item: item.clone(),
                        macro_context: &mut macro_context,
                        options: self.options,
                        context: self.context,
                        unit: self.query.unit.clone(),
                        source: source.clone(),
                    };

                    let ast = match kind {
                        MacroKind::Expr => {
                            IndexAst::Expr(compile_macro::<ast::Expr>(source_id, compiler, ast)?)
                        }
                        MacroKind::Item => {
                            IndexAst::Item(compile_macro::<ast::Item>(source_id, compiler, ast)?)
                        }
                    };

                    self.queue.push_back(Task::Index {
                        item,
                        items,
                        source_id,
                        source,
                        scopes,
                        impl_items,
                        ast,
                    });
                }
            }
        }

        Ok(())
    }
}

/// An item that has been expanded by a macro.
pub(crate) enum Expanded {
    /// The expansion resulted in an expression.
    Expr(ast::Expr),
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
        unit: &mut UnitBuilder,
    ) -> CompileResult<()> {
        let Self {
            item,
            ast: decl_use,
            source,
            source_id,
        } = self;

        let span = decl_use.span();

        let mut name = Item::empty();
        let first = decl_use.first.resolve(storage, &*source)?;
        name.push(first.as_ref());

        let mut it = decl_use.rest.iter();
        let last = it.next_back();

        for (_, c) in it {
            match c {
                ast::ItemUseComponent::Wildcard(t) => {
                    return Err(CompileError::UnsupportedWildcard { span: t.span() });
                }
                ast::ItemUseComponent::Ident(ident) => {
                    name.push(ident.resolve(storage, &*source)?.as_ref());
                }
            }
        }

        if let Some((_, c)) = last {
            match c {
                ast::ItemUseComponent::Wildcard(..) => {
                    let mut new_names = Vec::new();

                    if !context.contains_prefix(&name) && !unit.contains_prefix(&name) {
                        return Err(CompileError::MissingModule { span, item: name });
                    }

                    let iter = context
                        .iter_components(&name)
                        .chain(unit.iter_components(&name));

                    for c in iter {
                        let mut name = name.clone();
                        name.push(c);
                        new_names.push(name);
                    }

                    for name in new_names {
                        unit.new_import(item.clone(), &name, span, source_id)?;
                    }
                }
                ast::ItemUseComponent::Ident(ident) => {
                    name.push(ident.resolve(storage, &*source)?.as_ref());
                    unit.new_import(item, &name, span, source_id)?;
                }
            }
        } else {
            unit.new_import(item, &name, span, source_id)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum MacroKind {
    Expr,
    Item,
}

#[derive(Debug)]
pub(crate) struct Macro {
    pub(crate) items: Items,
    pub(crate) ast: ast::MacroCall,
    pub(crate) source: Arc<Source>,
    pub(crate) source_id: usize,
    pub(crate) scopes: IndexScopes,
    pub(crate) impl_items: Vec<Item>,
    pub(crate) kind: MacroKind,
}

/// Compile the given macro, return the output from the macro.
fn compile_macro<'a, T>(
    source_id: usize,
    mut compiler: MacroCompiler<'_>,
    ast: ast::MacroCall,
) -> Result<T, LoadError>
where
    T: Parse,
    Indexer<'a>: Index<T>,
{
    let output = match compiler.eval_macro::<T>(ast) {
        Ok(output) => output,
        Err(error) => {
            return Err(LoadError::from(LoadErrorKind::CompileError {
                source_id,
                error,
            }));
        }
    };

    Ok(output)
}
