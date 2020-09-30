use crate::ast;
use crate::load::{FileSourceLoader, SourceLoader, Sources};
use crate::query::{Build, BuildEntry, Query};
use crate::shared::Consts;
use crate::worker::{LoadFileKind, Task, Worker};
use crate::{Error, Errors, Options, Spanned as _, Storage, Warnings};
use runestick::{Context, Item, Source, Span};
use std::collections::VecDeque;

mod assembly;
mod compile;
mod compile_error;
mod compile_visitor;
mod compiler;
mod loops;
mod scopes;
mod unit_builder;

pub use self::compile_error::{CompileError, CompileErrorKind, CompileResult};
pub use self::compile_visitor::{CompileVisitor, NoopCompileVisitor};
pub use self::scopes::Var;
pub use self::unit_builder::{
    ImportEntry, ImportKey, InsertMetaError, LinkerError, Named, UnitBuilder,
};
use crate::parsing::Resolve as _;

pub(crate) use self::assembly::{Assembly, AssemblyInst};
pub(crate) use self::compile::Compile;
pub(crate) use self::compiler::{Compiler, Needs};
pub(crate) use self::loops::{Loop, Loops};
pub(crate) use self::scopes::{Scope, ScopeGuard, Scopes};

/// Compile the given source with default options.
pub fn compile(
    context: &Context,
    sources: &mut Sources,
    unit: &UnitBuilder,
    errors: &mut Errors,
    warnings: &mut Warnings,
) -> Result<(), ()> {
    let mut visitor = NoopCompileVisitor::new();
    let mut source_loader = FileSourceLoader::new();

    compile_with_options(
        context,
        sources,
        unit,
        errors,
        warnings,
        &Default::default(),
        &mut visitor,
        &mut source_loader,
    )?;

    Ok(())
}

/// Encode the given object into a collection of asm.
pub fn compile_with_options(
    context: &Context,
    sources: &mut Sources,
    unit: &UnitBuilder,
    errors: &mut Errors,
    warnings: &mut Warnings,
    options: &Options,
    visitor: &mut dyn CompileVisitor,
    source_loader: &mut dyn SourceLoader,
) -> Result<(), ()> {
    // Global storage.
    let storage = Storage::new();
    // Worker queue.
    let mut queue = VecDeque::new();
    // Constants storage.
    let consts = Consts::default();

    // Queue up the initial sources to be loaded.
    for source_id in sources.source_ids() {
        queue.push_back(Task::LoadFile {
            kind: LoadFileKind::Root,
            item: Item::new(),
            source_id,
        });
    }

    // The worker queue.
    let mut worker = Worker::new(
        queue,
        context,
        sources,
        options,
        unit.clone(),
        consts,
        errors,
        warnings,
        visitor,
        source_loader,
        storage.clone(),
    );

    worker.run();

    if !worker.errors.is_empty() {
        return Err(());
    }

    verify_imports(worker.errors, context, unit)?;

    loop {
        while let Some(entry) = worker.query.queue.pop_front() {
            let source_id = entry.source_id;

            let task = CompileBuildEntry {
                context,
                options,
                storage: &storage,
                unit,
                warnings: worker.warnings,
                query: &mut worker.query,
                entry,
                visitor: worker.visitor,
            };

            if let Err(error) = task.compile() {
                worker.errors.push(Error::new(source_id, error));
            }
        }

        match worker.query.queue_unused_entries(worker.visitor) {
            Ok(true) => (),
            Ok(false) => break,
            Err((source_id, error)) => {
                worker
                    .errors
                    .push(Error::new(source_id, CompileError::from(error)));
            }
        }
    }

    if !worker.errors.is_empty() {
        return Err(());
    }

    Ok(())
}

struct CompileBuildEntry<'a> {
    context: &'a Context,
    options: &'a Options,
    storage: &'a Storage,
    unit: &'a UnitBuilder,
    warnings: &'a mut Warnings,
    query: &'a mut Query,
    entry: BuildEntry,
    visitor: &'a mut dyn CompileVisitor,
}

impl CompileBuildEntry<'_> {
    fn compile(self) -> Result<(), CompileError> {
        let BuildEntry {
            item,
            build,
            span,
            source,
            source_id,
            used,
        } = self.entry;

        let mut asm = self.unit.new_assembly(span, source_id);

        let mut compiler = Compiler {
            storage: self.storage,
            source_id,
            source: source.clone(),
            context: self.context,
            query: self.query,
            asm: &mut asm,
            unit: self.unit.clone(),
            scopes: Scopes::new(),
            contexts: vec![],
            loops: Loops::new(),
            options: self.options,
            warnings: self.warnings,
            visitor: self.visitor,
        };

        match build {
            Build::Function(f) => {
                let args =
                    format_fn_args(self.storage, &*source, f.ast.args.iter().map(|(a, _)| a))?;

                let span = f.ast.span();
                let count = f.ast.args.len();
                compiler.contexts.push(span);
                compiler.compile((f.ast, false))?;

                if used.is_unused() {
                    compiler.warnings.not_used(source_id, span, None);
                } else {
                    self.unit
                        .new_function(span, source_id, item, count, asm, f.call, args)?;
                }
            }
            Build::InstanceFunction(f) => {
                let args =
                    format_fn_args(self.storage, &*source, f.ast.args.iter().map(|(a, _)| a))?;

                let span = f.ast.span();
                let count = f.ast.args.len();
                compiler.contexts.push(span);

                let source = compiler.source.clone();
                let name = f.ast.name.resolve(self.storage, &*source)?;

                let meta = compiler
                    .lookup_exact_meta(f.instance_span, &f.impl_item)?
                    .ok_or_else(|| {
                        CompileError::new(
                            &f.instance_span,
                            CompileErrorKind::MissingType {
                                item: (*f.impl_item).clone(),
                            },
                        )
                    })?;

                let type_of = meta.type_of().ok_or_else(|| {
                    CompileError::new(
                        span,
                        CompileErrorKind::UnsupportedInstanceFunction { meta: meta.clone() },
                    )
                })?;

                compiler.compile((f.ast, true))?;

                if used.is_unused() {
                    compiler.warnings.not_used(source_id, span, None);
                } else {
                    self.unit.new_instance_function(
                        span,
                        source_id,
                        item,
                        type_of,
                        name.as_ref(),
                        count,
                        asm,
                        f.call,
                        args,
                    )?;
                }
            }
            Build::Closure(c) => {
                let args = format_fn_args(
                    self.storage,
                    &*source,
                    c.ast.args.as_slice().iter().map(|(a, _)| a),
                )?;

                let count = c.ast.args.len();
                let span = c.ast.span();
                compiler.contexts.push(span);
                compiler.compile((c.ast, &c.captures[..]))?;

                if used.is_unused() {
                    compiler.warnings.not_used(source_id, span, None);
                } else {
                    self.unit
                        .new_function(span, source_id, item, count, asm, c.call, args)?;
                }
            }
            Build::AsyncBlock(b) => {
                let args = b.captures.len();
                let span = b.ast.span();
                compiler.contexts.push(span);
                compiler.compile((&b.ast, &b.captures[..]))?;

                if used.is_unused() {
                    compiler.warnings.not_used(source_id, span, None);
                } else {
                    self.unit
                        .new_function(span, source_id, item, args, asm, b.call, Vec::new())?;
                }
            }
            Build::UnusedConst(c) => {
                self.warnings.not_used(source_id, &c.ir, None);
            }
            Build::UnusedConstFn(c) => {
                self.warnings.not_used(source_id, &c.item_fn, None);
            }
        }

        Ok(())
    }
}

fn format_fn_args<'a, I>(
    storage: &Storage,
    source: &Source,
    arguments: I,
) -> Result<Vec<String>, CompileError>
where
    I: IntoIterator<Item = &'a ast::FnArg>,
{
    let mut args = Vec::new();

    for arg in arguments {
        match arg {
            ast::FnArg::SelfValue(..) => {
                args.push(String::from("self"));
            }
            ast::FnArg::Ignore(..) => {
                args.push(String::from("_"));
            }
            ast::FnArg::Ident(ident) => {
                args.push(ident.resolve(storage, source)?.to_string());
            }
        }
    }

    Ok(args)
}

fn verify_imports(errors: &mut Errors, context: &Context, unit: &UnitBuilder) -> Result<(), ()> {
    for (_, entry) in &*unit.imports() {
        if context.contains_prefix(&entry.item) || unit.contains_prefix(&entry.item) {
            continue;
        }

        if let Some((span, source_id)) = entry.span {
            errors.push(Error::new(
                source_id,
                CompileError::new(
                    span,
                    CompileErrorKind::MissingItem {
                        item: entry.item.clone(),
                    },
                ),
            ));
        } else {
            errors.push(Error::new(
                0,
                CompileError::new(
                    &Span::empty(),
                    CompileErrorKind::MissingPreludeModule {
                        item: entry.item.clone(),
                    },
                ),
            ));
        }

        return Err(());
    }

    Ok(())
}
