use crate::ast;
use crate::load::{FileSourceLoader, SourceLoader, Sources};
use crate::query::{Build, BuildEntry, Query, Used};
use crate::shared::Consts;
use crate::worker::{LoadFileKind, Task, Worker};
use crate::{Error, Errors, Options, Spanned as _, Storage, Warnings};
use runestick::{Context, Source, Span};

mod assemble;
mod assembly;
mod compile_error;
mod compile_visitor;
mod compiler;
mod loops;
mod scopes;
mod unit_builder;

pub use self::compile_error::{CompileError, CompileErrorKind, CompileResult, ImportEntryStep};
pub use self::compile_visitor::{CompileVisitor, NoopCompileVisitor};
pub use self::scopes::Var;
pub use self::unit_builder::{BuildError, InsertMetaError, LinkerError, UnitBuilder};
use crate::parsing::Resolve as _;

pub(crate) use self::assemble::{Assemble, AssembleClosure, AssembleConst, AssembleFn};
pub(crate) use self::assembly::{Assembly, AssemblyInst};
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
    // Constants storage.
    let consts = Consts::default();

    // The worker queue.
    let mut worker = Worker::new(
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

    // Queue up the initial sources to be loaded.
    for source_id in worker.sources.source_ids() {
        let mod_item = match worker.query.insert_root_mod(source_id, Span::empty()) {
            Ok(result) => result,
            Err(error) => {
                errors.push(Error::new(source_id, error));
                return Err(());
            }
        };

        worker.queue.push_back(Task::LoadFile {
            kind: LoadFileKind::Root,
            source_id,
            mod_item,
        });
    }

    worker.run();

    if !worker.errors.is_empty() {
        return Err(());
    }

    loop {
        while let Some(entry) = worker.query.next_build_entry() {
            let source_id = entry.location.source_id;

            let task = CompileBuildEntry {
                context,
                options,
                storage: &storage,
                unit,
                warnings: worker.warnings,
                consts: &worker.consts,
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
    consts: &'a Consts,
    query: &'a mut Query,
    entry: BuildEntry,
    visitor: &'a mut dyn CompileVisitor,
}

impl CompileBuildEntry<'_> {
    fn compile(self) -> Result<(), CompileError> {
        let BuildEntry {
            item,
            location,
            build,
            source,
            used,
        } = self.entry;

        let mut asm = self.unit.new_assembly(location);

        let mut compiler = Compiler {
            storage: self.storage,
            source_id: location.source_id,
            source: source.clone(),
            context: self.context,
            consts: self.consts,
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
                f.ast.assemble_fn(&mut compiler, false)?;

                if used.is_unused() {
                    compiler.warnings.not_used(location.source_id, span, None);
                } else {
                    self.unit.new_function(
                        location,
                        item.item.clone(),
                        count,
                        asm,
                        f.call,
                        args,
                    )?;
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

                let type_of = meta
                    .base_type_of()
                    .ok_or_else(|| CompileError::expected_meta(span, meta, "instance function"))?;

                f.ast.assemble_fn(&mut compiler, true)?;

                if used.is_unused() {
                    compiler.warnings.not_used(location.source_id, span, None);
                } else {
                    self.unit.new_instance_function(
                        location,
                        item.item.clone(),
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
                c.ast.assemble_closure(&mut compiler, &c.captures)?;

                if used.is_unused() {
                    compiler
                        .warnings
                        .not_used(location.source_id, location.span, None);
                } else {
                    self.unit.new_function(
                        location,
                        item.item.clone(),
                        count,
                        asm,
                        c.call,
                        args,
                    )?;
                }
            }
            Build::AsyncBlock(b) => {
                let args = b.captures.len();
                let span = b.ast.span();
                compiler.contexts.push(span);
                b.ast.assemble_closure(&mut compiler, &b.captures)?;

                if used.is_unused() {
                    compiler
                        .warnings
                        .not_used(location.source_id, location.span, None);
                } else {
                    self.unit.new_function(
                        location,
                        item.item.clone(),
                        args,
                        asm,
                        b.call,
                        Vec::new(),
                    )?;
                }
            }
            Build::Unused => {
                self.warnings
                    .not_used(location.source_id, location.span, None);
            }
            Build::Import(import) => {
                // Issue the import to check access.
                let result = self.query.get_import(
                    &item.module,
                    location.span,
                    &import.entry.imported,
                    Used::Used,
                )?;

                if used.is_unused() {
                    self.warnings
                        .not_used(location.source_id, location.span, None);
                }

                if result.is_none()
                    && !self.context.contains_prefix(&import.entry.imported)
                    && !self.query.contains_module(&import.entry.imported)
                {
                    return Err(CompileError::new(
                        location.span,
                        CompileErrorKind::MissingItem {
                            item: item.item.clone(),
                        },
                    ));
                }
            }
            Build::ReExport => {
                let import =
                    match self
                        .query
                        .get_import(&item.module, location.span, &item.item, used)?
                    {
                        Some(item) => item,
                        None => {
                            return Err(CompileError::new(
                                location.span,
                                CompileErrorKind::MissingItem {
                                    item: item.item.clone(),
                                },
                            ))
                        }
                    };

                self.unit
                    .new_function_reexport(location, &item.item, &import)?;
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
