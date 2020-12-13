use crate::ast;
use crate::load::{FileSourceLoader, SourceLoader, Sources};
use crate::query::{Build, BuildEntry, Query};
use crate::shared::{Consts, Gen};
use crate::worker::{LoadFileKind, Task, Worker};
use crate::{Error, Errors, Options, Spanned as _, Storage, Warnings};
use runestick::{Context, Location, Source, Span};
use std::rc::Rc;
use std::sync::Arc;

mod assembly;
mod compile_error;
mod compile_visitor;
mod unit_builder;
mod v1;

pub use self::compile_error::{CompileError, CompileErrorKind, CompileResult, ImportEntryStep};
pub use self::compile_visitor::{CompileVisitor, NoopCompileVisitor};
pub use self::unit_builder::{BuildError, InsertMetaError, LinkerError, UnitBuilder};
use crate::parsing::Resolve as _;

pub(crate) use self::assembly::{Assembly, AssemblyInst};

/// Compile the given source with default options.
pub fn compile(
    context: &Context,
    sources: &mut Sources,
    unit: &UnitBuilder,
    errors: &mut Errors,
    warnings: &mut Warnings,
) -> Result<(), ()> {
    let visitor = Rc::new(NoopCompileVisitor::new());
    let mut source_loader = FileSourceLoader::new();

    compile_with_options(
        context,
        sources,
        unit,
        errors,
        warnings,
        &Default::default(),
        visitor,
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
    visitor: Rc<dyn CompileVisitor>,
    source_loader: &mut dyn SourceLoader,
) -> Result<(), ()> {
    // Global storage.
    let storage = Storage::new();
    // Shared id generator.
    let gen = Gen::new();
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
        visitor.clone(),
        source_loader,
        storage.clone(),
        gen,
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
                visitor: &visitor,
                context,
                options,
                storage: &storage,
                unit,
                warnings: worker.warnings,
                consts: &worker.consts,
                query: &mut worker.query,
            };

            if let Err(error) = task.compile(entry) {
                worker.errors.push(Error::new(source_id, error));
            }
        }

        match worker.query.queue_unused_entries() {
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
    visitor: &'a Rc<dyn CompileVisitor>,
    context: &'a Context,
    options: &'a Options,
    storage: &'a Storage,
    unit: &'a UnitBuilder,
    warnings: &'a mut Warnings,
    consts: &'a Consts,
    query: &'a mut Query,
}

impl CompileBuildEntry<'_> {
    fn compiler1<'a>(
        &'a mut self,
        location: Location,
        source: &Arc<Source>,
        span: Span,
        asm: &'a mut Assembly,
    ) -> self::v1::Compiler<'a> {
        self::v1::Compiler {
            visitor: self.visitor.clone(),
            storage: self.storage,
            source_id: location.source_id,
            source: source.clone(),
            context: self.context,
            consts: self.consts,
            query: self.query,
            asm,
            unit: self.unit.clone(),
            scopes: self::v1::Scopes::new(self.visitor.clone()),
            contexts: vec![span],
            loops: self::v1::Loops::new(),
            options: self.options,
            warnings: self.warnings,
        }
    }

    fn compile(mut self, entry: BuildEntry) -> Result<(), CompileError> {
        let BuildEntry {
            item,
            location,
            build,
            source,
            used,
        } = entry;

        let mut asm = self.unit.new_assembly(location);

        match build {
            Build::Function(f) => {
                use self::v1::AssembleFn as _;

                let args = format_fn_args(&*source, f.ast.args.iter().map(|(a, _)| a))?;

                let span = f.ast.span();
                let count = f.ast.args.len();

                let mut c = self.compiler1(location, &source, span, &mut asm);
                f.ast.assemble_fn(&mut c, false)?;

                if used.is_unused() {
                    self.warnings.not_used(location.source_id, span, None);
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
                use self::v1::AssembleFn as _;

                let args = format_fn_args(&*source, f.ast.args.iter().map(|(a, _)| a))?;

                let span = f.ast.span();
                let count = f.ast.args.len();
                let name = f.ast.name.resolve(self.storage, &*source)?;

                let mut c = self.compiler1(location, &source, span, &mut asm);
                let meta = c.lookup_meta(f.instance_span, &f.impl_item)?;

                let type_hash = meta
                    .type_hash_of()
                    .ok_or_else(|| CompileError::expected_meta(span, meta, "instance function"))?;

                f.ast.assemble_fn(&mut c, true)?;

                if used.is_unused() {
                    c.warnings.not_used(location.source_id, span, None);
                } else {
                    self.unit.new_instance_function(
                        location,
                        item.item.clone(),
                        type_hash,
                        name.as_ref(),
                        count,
                        asm,
                        f.call,
                        args,
                    )?;
                }
            }
            Build::Closure(closure) => {
                use self::v1::AssembleClosure as _;

                let span = closure.ast.span();
                let args =
                    format_fn_args(&*source, closure.ast.args.as_slice().iter().map(|(a, _)| a))?;

                let mut c = self.compiler1(location, &source, span, &mut asm);
                closure.ast.assemble_closure(&mut c, &closure.captures)?;

                if used.is_unused() {
                    c.warnings.not_used(location.source_id, location.span, None);
                } else {
                    self.unit.new_function(
                        location,
                        item.item.clone(),
                        closure.ast.args.len(),
                        asm,
                        closure.call,
                        args,
                    )?;
                }
            }
            Build::AsyncBlock(b) => {
                use self::v1::AssembleClosure as _;

                let args = b.captures.len();
                let span = b.ast.span();

                let mut c = self.compiler1(location, &source, span, &mut asm);
                b.ast.assemble_closure(&mut c, &b.captures)?;

                if used.is_unused() {
                    self.warnings
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
                let result = self
                    .query
                    .import(location.span, &item.module, &item.item, used)?;

                if used.is_unused() {
                    self.warnings
                        .not_used(location.source_id, location.span, None);
                }

                let missing = match &result {
                    Some(item) => {
                        if self.context.contains_prefix(item) || self.query.contains_prefix(item) {
                            None
                        } else {
                            Some(item)
                        }
                    }
                    None => Some(&import.entry.target),
                };

                if let Some(item) = missing {
                    return Err(CompileError::new(
                        location.span,
                        CompileErrorKind::MissingItem { item: item.clone() },
                    ));
                }
            }
            Build::ReExport => {
                let import =
                    match self
                        .query
                        .import(location.span, &item.module, &item.item, used)?
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

fn format_fn_args<'a, I>(source: &Source, arguments: I) -> Result<Vec<String>, CompileError>
where
    I: IntoIterator<Item = &'a ast::FnArg>,
{
    let mut args = Vec::new();

    for arg in arguments {
        match arg {
            ast::FnArg::SelfValue(..) => {
                args.push(String::from("self"));
            }
            ast::FnArg::Pat(pat) => {
                let span = pat.span();

                if let Some(s) = source.source(span) {
                    args.push(s.to_owned());
                } else {
                    args.push(String::from("*"));
                }
            }
        }
    }

    Ok(args)
}
