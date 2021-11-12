use crate::ast;
use crate::load::{SourceLoader, Sources};
use crate::query::{Build, BuildEntry, Query};
use crate::shared::Gen;
use crate::worker::{LoadFileKind, Task, Worker};
use crate::{Context, Diagnostics, Location, Options, Source, Span, Spanned};
use std::rc::Rc;

mod assembly;
mod compile_error;
mod compile_visitor;
mod unit_builder;
mod v1;

pub(crate) use self::assembly::{Assembly, AssemblyInst};
pub use self::compile_error::{CompileError, CompileErrorKind, CompileResult, ImportEntryStep};
pub use self::compile_visitor::{CompileVisitor, NoopCompileVisitor};
pub(crate) use self::unit_builder::UnitBuilder;
pub use self::unit_builder::{BuildError, InsertMetaError, LinkerError};
use crate::parsing::Resolve;

/// Encode the given object into a collection of asm.
pub(crate) fn compile_with_options<'a>(
    context: &Context,
    sources: &mut Sources,
    unit: &mut UnitBuilder,
    diagnostics: &mut Diagnostics,
    options: &Options,
    visitor: Rc<dyn CompileVisitor>,
    source_loader: Rc<dyn SourceLoader + 'a>,
) -> Result<(), ()> {
    // Shared id generator.
    let gen = Gen::new();

    // The worker queue.
    let mut worker = Worker::new(
        context,
        sources,
        options,
        unit,
        diagnostics,
        visitor.clone(),
        source_loader,
        gen,
    );

    // Queue up the initial sources to be loaded.
    for source_id in worker.q.sources.source_ids() {
        let mod_item = match worker.q.insert_root_mod(source_id, Span::empty()) {
            Ok(result) => result,
            Err(error) => {
                worker.diagnostics.error(source_id, error);
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

    if worker.diagnostics.has_error() {
        return Err(());
    }

    loop {
        while let Some(entry) = worker.q.next_build_entry() {
            let source_id = entry.location.source_id;

            let task = CompileBuildEntry {
                visitor: &visitor,
                context,
                options,
                diagnostics: worker.diagnostics,
                q: &mut worker.q,
            };

            if let Err(error) = task.compile(entry) {
                worker.diagnostics.error(source_id, error);
            }
        }

        match worker.q.queue_unused_entries() {
            Ok(true) => (),
            Ok(false) => break,
            Err((source_id, error)) => {
                worker.diagnostics.error(source_id, error);
            }
        }
    }

    if worker.diagnostics.has_error() {
        return Err(());
    }

    Ok(())
}

struct CompileBuildEntry<'a, 'q> {
    visitor: &'a Rc<dyn CompileVisitor>,
    context: &'a Context,
    options: &'a Options,
    diagnostics: &'a mut Diagnostics,
    q: &'a mut Query<'q>,
}

impl<'q> CompileBuildEntry<'_, 'q> {
    fn compiler1<'a>(
        &'a mut self,
        location: Location,
        span: Span,
        asm: &'a mut Assembly,
    ) -> self::v1::Compiler<'a, 'q> {
        self::v1::Compiler {
            visitor: self.visitor.clone(),
            source_id: location.source_id,
            context: self.context,
            q: self.q,
            asm,
            scopes: self::v1::Scopes::new(self.visitor.clone()),
            contexts: vec![span],
            loops: self::v1::Loops::new(),
            options: self.options,
            diagnostics: self.diagnostics,
        }
    }

    fn compile(mut self, entry: BuildEntry) -> Result<(), CompileError> {
        let BuildEntry {
            item,
            location,
            build,
            used,
        } = entry;

        let source = self.q.sources.get(location.source_id).ok_or_else(|| {
            CompileError::new(
                location.span,
                CompileErrorKind::MissingSourceId {
                    source_id: location.source_id,
                },
            )
        })?;

        let mut asm = self.q.unit.new_assembly(location);

        match build {
            Build::Function(f) => {
                use self::v1::AssembleFn;

                let args = format_fn_args(source, f.ast.args.iter().map(|(a, _)| a))?;

                let span = f.ast.span();
                let count = f.ast.args.len();

                let mut c = self.compiler1(location, span, &mut asm);
                f.ast.assemble_fn(&mut c, false)?;

                if used.is_unused() {
                    self.diagnostics.not_used(location.source_id, span, None);
                } else {
                    self.q.unit.new_function(
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
                use self::v1::AssembleFn;

                let args = format_fn_args(&*source, f.ast.args.iter().map(|(a, _)| a))?;

                let span = f.ast.span();
                let count = f.ast.args.len();

                let mut c = self.compiler1(location, span, &mut asm);
                let meta = c.lookup_meta(f.instance_span, &f.impl_item)?;

                let type_hash = meta
                    .type_hash_of()
                    .ok_or_else(|| CompileError::expected_meta(span, meta, "instance function"))?;

                f.ast.assemble_fn(&mut c, true)?;

                if used.is_unused() {
                    c.diagnostics.not_used(location.source_id, span, None);
                } else {
                    let name = f.ast.name.resolve(&self.q.storage, self.q.sources)?;

                    self.q.unit.new_instance_function(
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
                use self::v1::AssembleClosure;

                let span = closure.ast.span();
                let args =
                    format_fn_args(&*source, closure.ast.args.as_slice().iter().map(|(a, _)| a))?;

                let mut c = self.compiler1(location, span, &mut asm);
                closure.ast.assemble_closure(&mut c, &closure.captures)?;

                if used.is_unused() {
                    c.diagnostics
                        .not_used(location.source_id, location.span, None);
                } else {
                    self.q.unit.new_function(
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
                use self::v1::AssembleClosure;

                let args = b.captures.len();
                let span = b.ast.span();

                let mut c = self.compiler1(location, span, &mut asm);
                b.ast.assemble_closure(&mut c, &b.captures)?;

                if used.is_unused() {
                    self.diagnostics
                        .not_used(location.source_id, location.span, None);
                } else {
                    self.q.unit.new_function(
                        location,
                        item.item.clone(),
                        args,
                        asm,
                        b.call,
                        Default::default(),
                    )?;
                }
            }
            Build::Unused => {
                self.diagnostics
                    .not_used(location.source_id, location.span, None);
            }
            Build::Import(import) => {
                // Issue the import to check access.
                let result = self
                    .q
                    .import(location.span, &item.module, &item.item, used)?;

                if used.is_unused() {
                    self.diagnostics
                        .not_used(location.source_id, location.span, None);
                }

                let missing = match &result {
                    Some(item) => {
                        if self.context.contains_prefix(item) || self.q.contains_prefix(item) {
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
                let import = match self
                    .q
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

                self.q
                    .unit
                    .new_function_reexport(location, &item.item, &import)?;
            }
        }

        Ok(())
    }
}

fn format_fn_args<'a, I>(source: &Source, arguments: I) -> Result<Box<[Box<str>]>, CompileError>
where
    I: IntoIterator<Item = &'a ast::FnArg>,
{
    let mut args = Vec::new();

    for arg in arguments {
        match arg {
            ast::FnArg::SelfValue(..) => {
                args.push("self".into());
            }
            ast::FnArg::Pat(pat) => {
                let span = pat.span();

                if let Some(s) = source.source(span) {
                    args.push(s.into());
                } else {
                    args.push("*".into());
                }
            }
        }
    }

    Ok(args.into())
}
