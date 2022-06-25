//! The Rune compiler.
//!
//! The main entry to compiling rune source is [prepare][crate::prepare] which
//! uses this compiler. In here you'll just find compiler-specific types.

use crate::ast;
use crate::ast::{Span, Spanned};
use crate::hir;
use crate::macros::Storage;
use crate::parse::Resolve;
use crate::query::{Build, BuildEntry, Query};
use crate::shared::{Consts, Gen};
use crate::worker::{LoadFileKind, Task, Worker};
use crate::{Diagnostics, Sources};

mod assembly;
pub(crate) use self::assembly::{Assembly, AssemblyInst};

pub(crate) mod attrs;

mod compile_error;
pub use self::compile_error::{CompileError, CompileErrorKind, ImportStep};

mod compile_visitor;
pub use self::compile_visitor::CompileVisitor;
pub(crate) use self::compile_visitor::NoopCompileVisitor;

pub(crate) mod context;
pub use self::context::{Context, ContextError, ContextSignature, ContextTypeInfo};

pub(crate) mod ir;
pub(crate) use self::ir::{IrBudget, IrCompiler, IrEvalContext, IrEvalOutcome, IrInterpreter};
pub use self::ir::{IrError, IrErrorKind, IrEval, IrValue};

pub(crate) mod item;
pub use self::item::{Component, ComponentRef, IntoComponent, Item};

mod source_loader;
pub use self::source_loader::{FileSourceLoader, SourceLoader};

mod unit_builder;
pub use self::unit_builder::LinkerError;
pub(crate) use self::unit_builder::UnitBuilder;

mod v1;

mod options;
pub use self::options::{Options, ParseOptionError};

mod location;
pub use self::location::Location;

mod meta;
pub(crate) use self::meta::{
    CaptureMeta, ItemMeta, ModMeta, PrivMeta, PrivMetaKind, PrivStructMeta, PrivTupleMeta,
    PrivVariantMeta,
};
pub use self::meta::{Meta, MetaKind, MetaRef, SourceMeta};

mod module;
pub use self::module::{AssocType, InstallWith, Module, Variant};

mod named;
pub use self::named::Named;

mod names;
pub(crate) use self::names::Names;

mod visibility;
pub(crate) use self::visibility::Visibility;

/// A compile result alias.
pub(crate) type CompileResult<T> = ::std::result::Result<T, CompileError>;

/// Encode the given object into a collection of asm.
pub(crate) fn compile(
    unit: &mut UnitBuilder,
    sources: &mut Sources,
    context: &Context,
    diagnostics: &mut Diagnostics,
    options: &Options,
    visitor: &mut dyn CompileVisitor,
    source_loader: &mut dyn SourceLoader,
) -> Result<(), ()> {
    // Shared id generator.
    let gen = Gen::new();
    let mut consts = Consts::default();
    let mut storage = Storage::default();
    let mut inner = Default::default();

    // The worker queue.
    let mut worker = Worker::new(
        context,
        &mut consts,
        &mut storage,
        sources,
        options,
        unit,
        diagnostics,
        visitor,
        source_loader,
        &gen,
        &mut inner,
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
            tracing::trace!("next build entry: {}", entry.item.item);

            let source_id = entry.location.source_id;

            let task = CompileBuildEntry {
                context,
                options,
                diagnostics: worker.diagnostics,
                q: worker.q.borrow(),
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

struct CompileBuildEntry<'a> {
    context: &'a Context,
    options: &'a Options,
    diagnostics: &'a mut Diagnostics,
    q: Query<'a>,
}

impl CompileBuildEntry<'_> {
    fn compiler1<'a>(
        &'a mut self,
        location: Location,
        span: Span,
        asm: &'a mut Assembly,
    ) -> self::v1::Assembler<'a> {
        self::v1::Assembler {
            source_id: location.source_id,
            context: self.context,
            q: self.q.borrow(),
            asm,
            scopes: self::v1::Scopes::new(),
            contexts: vec![span],
            loops: self::v1::Loops::new(),
            options: self.options,
            diagnostics: self.diagnostics,
        }
    }

    #[tracing::instrument(skip(self, entry))]
    fn compile(mut self, entry: BuildEntry) -> Result<(), CompileError> {
        let BuildEntry {
            item,
            location,
            build,
            used,
        } = entry;

        let mut asm = self.q.unit.new_assembly(location);

        match build {
            Build::Function(f) => {
                tracing::trace!("function: {}", item.item);

                use self::v1::assemble;

                let args =
                    format_fn_args(self.q.sources, location, f.ast.args.iter().map(|(a, _)| a))?;

                let span = f.ast.span();
                let count = f.ast.args.len();

                let mut c = self.compiler1(location, span, &mut asm);
                let arena = hir::Arena::new();
                let ctx = hir::lowering::Ctx::new(&arena);
                let hir = hir::lowering::item_fn(&ctx, &f.ast)?;
                assemble::fn_from_item_fn(&f.ast, &mut c, false)?;

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
                tracing::trace!("instance function: {}", item.item);

                use self::v1::assemble;

                let args =
                    format_fn_args(self.q.sources, location, f.ast.args.iter().map(|(a, _)| a))?;

                let span = f.ast.span();
                let count = f.ast.args.len();

                let mut c = self.compiler1(location, span, &mut asm);
                let meta = c.lookup_meta(f.instance_span, &f.impl_item)?;

                let type_hash = meta.type_hash_of().ok_or_else(|| {
                    CompileError::expected_meta(span, meta.info(), "instance function")
                })?;

                assemble::fn_from_item_fn(&f.ast, &mut c, true)?;

                if used.is_unused() {
                    c.diagnostics.not_used(location.source_id, span, None);
                } else {
                    let name = f.ast.name.resolve(resolve_context!(self.q))?;

                    self.q.unit.new_instance_function(
                        location,
                        item.item.clone(),
                        type_hash,
                        name,
                        count,
                        asm,
                        f.call,
                        args,
                    )?;
                }
            }
            Build::Closure(closure) => {
                tracing::trace!("closure: {}", item.item);

                use self::v1::assemble;

                let span = closure.ast.span();
                let args = format_fn_args(
                    self.q.sources,
                    location,
                    closure.ast.args.as_slice().iter().map(|(a, _)| a),
                )?;

                let mut c = self.compiler1(location, span, &mut asm);
                assemble::closure_from_expr_closure(&closure.ast, &mut c, &closure.captures)?;

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
                tracing::trace!("async block: {}", item.item);

                use self::v1::assemble;

                let args = b.captures.len();
                let span = b.ast.span();

                let mut c = self.compiler1(location, span, &mut asm);
                assemble::closure_from_block(&b.ast, &mut c, &b.captures)?;

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
                tracing::trace!("unused: {}", item.item);

                if !item.visibility.is_public() {
                    self.diagnostics
                        .not_used(location.source_id, location.span, None);
                }
            }
            Build::Import(import) => {
                tracing::trace!("import: {}", item.item);

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
                tracing::trace!("re-export: {}", item.item);

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

fn format_fn_args<'a, I>(
    sources: &Sources,
    location: Location,
    arguments: I,
) -> Result<Box<[Box<str>]>, CompileError>
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

                if let Some(s) = sources.source(location.source_id, span) {
                    args.push(s.into());
                } else {
                    args.push("*".into());
                }
            }
        }
    }

    Ok(args.into())
}
