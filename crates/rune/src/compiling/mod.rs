use crate::ast;
use crate::load::{FileSourceLoader, SourceLoader, Sources};
use crate::query::{Build, BuildEntry, Query};
#[cfg(compiler_v2)]
use crate::shared::ResultExt as _;
use crate::shared::{Consts, Gen};
use crate::worker::{LoadFileKind, Task, Worker};
use crate::{Diagnostics, Options, Spanned as _, Storage};
use runestick::{Context, Location, Source, Span};
use std::rc::Rc;
use std::sync::Arc;

mod assembly;
mod compile_error;
mod compile_visitor;
mod unit_builder;
mod v1;
#[cfg(compiler_v2)]
mod v2;

pub use self::compile_error::{CompileError, CompileErrorKind, CompileResult, ImportEntryStep};
pub use self::compile_visitor::{CompileVisitor, NoopCompileVisitor};
pub use self::unit_builder::{BuildError, InsertMetaError, LinkerError, UnitBuilder};
use crate::parsing::Resolve as _;

pub(crate) use self::assembly::{Assembly, AssemblyInst};

/// Compile the given source with default options.
#[allow(clippy::result_unit_err)]
pub fn compile(
    context: &Context,
    sources: &mut Sources,
    unit: &UnitBuilder,
    diagnostics: &mut Diagnostics,
) -> Result<(), ()> {
    let visitor = Rc::new(NoopCompileVisitor::new());
    let source_loader = Rc::new(FileSourceLoader::new());

    compile_with_options(
        context,
        sources,
        unit,
        diagnostics,
        &Default::default(),
        visitor,
        source_loader,
    )?;

    Ok(())
}

/// Encode the given object into a collection of asm.
pub fn compile_with_options<'a>(
    context: &Context,
    sources: &mut Sources,
    unit: &UnitBuilder,
    diagnostics: &mut Diagnostics,
    options: &Options,
    visitor: Rc<dyn CompileVisitor>,
    source_loader: Rc<dyn SourceLoader + 'a>,
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
        diagnostics,
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
        while let Some(entry) = worker.query.next_build_entry() {
            let source_id = entry.location.source_id;

            let task = CompileBuildEntry {
                visitor: &visitor,
                context,
                options,
                storage: &storage,
                unit,
                diagnostics: worker.diagnostics,
                consts: &worker.consts,
                query: &mut worker.query,
            };

            if let Err(error) = task.compile(entry) {
                worker.diagnostics.error(source_id, error);
            }
        }

        match worker.query.queue_unused_entries() {
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

struct CompileBuildEntry<'a> {
    visitor: &'a Rc<dyn CompileVisitor>,
    context: &'a Context,
    options: &'a Options,
    storage: &'a Storage,
    unit: &'a UnitBuilder,
    diagnostics: &'a mut Diagnostics,
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
            diagnostics: self.diagnostics,
        }
    }

    /// Construct an instance of the next version of the compiler.
    #[cfg(compiler_v2)]
    fn compiler2<'a>(
        &'a mut self,
        location: Location,
        source: &'a Arc<Source>,
        span: Span,
        program: &'a mut rune_ssa::Program,
    ) -> self::v2::Compiler<'a> {
        self::v2::Compiler {
            program,
            location,
            contexts: vec![span],
            source,
            scope: self::v2::scope::Stack::new(location.source_id, self.visitor.clone()),
            storage: self.storage,
            context: self.context,
            consts: self.consts,
            query: self.query,
            unit: self.unit.clone(),
            options: self.options,
            diagnostics: self.diagnostics,
            visitor: self.visitor.clone(),
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

                // NB: experimental compiler that is work-in-progress
                #[cfg(compiler_v2)]
                if self.options.v2 {
                    let mut program = rune_ssa::Program::new();
                    let mut c2 = self.compiler2(location, &source, span, &mut program);
                    self::v2::AssembleFn::assemble_fn(f.ast.as_ref(), &mut c2, true)?;
                    program.seal().with_span(span)?;
                    println!("{}", program.dump());
                }

                if used.is_unused() {
                    self.diagnostics.not_used(location.source_id, span, None);
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
                    c.diagnostics.not_used(location.source_id, span, None);
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
                    c.diagnostics
                        .not_used(location.source_id, location.span, None);
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
                    self.diagnostics
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
                self.diagnostics
                    .not_used(location.source_id, location.span, None);
            }
            Build::Import(import) => {
                // Issue the import to check access.
                let result = self
                    .query
                    .import(location.span, &item.module, &item.item, used)?;

                if used.is_unused() {
                    self.diagnostics
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
