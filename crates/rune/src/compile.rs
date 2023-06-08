//! The Rune compiler.
//!
//! The main entry to compiling rune source is [prepare][crate::prepare] which
//! uses this compiler. In here you'll just find compiler-specific types.

mod assembly;
pub(crate) use self::assembly::{Assembly, AssemblyInst};

pub(crate) mod attrs;

pub(crate) mod error;
pub(crate) use self::error::{
    CompileErrorKind, HirErrorKind, IrErrorKind, ParseErrorKind, QueryErrorKind, ResolveErrorKind,
};
pub use self::error::{Error, ImportStep};

mod compile_visitor;
pub use self::compile_visitor::CompileVisitor;
pub(crate) use self::compile_visitor::NoopCompileVisitor;

pub(crate) mod context;
pub use self::context::Context;

pub(crate) mod context_error;
pub use self::context_error::ContextError;

pub(crate) mod meta_info;
pub use meta_info::MetaInfo;

mod docs;
pub(crate) use self::docs::Docs;

mod prelude;
pub(crate) use self::prelude::Prelude;

pub(crate) mod ir;
pub(crate) use self::ir::{IrBudget, IrCompiler, IrEvalContext, IrEvalOutcome, IrInterpreter};
pub use self::ir::{IrEval, IrValue};

pub use rune_core::{Component, ComponentRef, IntoComponent, Item, ItemBuf};

mod source_loader;
pub use self::source_loader::{FileSourceLoader, SourceLoader};

mod unit_builder;
pub use self::unit_builder::LinkerError;
pub(crate) use self::unit_builder::UnitBuilder;

pub(crate) mod v1;

mod options;
pub use self::options::{Options, ParseOptionError};

mod location;
pub use self::location::Location;

pub mod meta;
pub(crate) use self::meta::{Doc, ItemMeta};
pub use self::meta::{MetaRef, SourceMeta};

mod pool;
pub(crate) use self::pool::{ItemId, ModId, ModMeta, Pool};

mod named;
pub use self::named::Named;

mod names;
pub(crate) use self::names::Names;

mod visibility;
pub(crate) use self::visibility::Visibility;

mod with_span;
pub use self::with_span::{HasSpan, WithSpan};

/// Helper alias for compile results.
pub type Result<T, E = Error> = ::core::result::Result<T, E>;

use crate::no_std::prelude::*;

use crate::ast;
use crate::ast::{Span, Spanned};
use crate::hir;
use crate::macros::Storage;
use crate::parse::Resolve;
use crate::query::{Build, BuildEntry, Query};
use crate::runtime::unit::UnitEncoder;
use crate::shared::{Consts, Gen};
use crate::worker::{LoadFileKind, Task, Worker};
use crate::{Diagnostics, Sources};

/// Encode the given object into a collection of asm.
pub(crate) fn compile(
    unit: &mut UnitBuilder,
    prelude: &Prelude,
    sources: &mut Sources,
    pool: &mut Pool,
    context: &Context,
    options: &Options,
    visitor: &mut dyn CompileVisitor,
    diagnostics: &mut Diagnostics,
    source_loader: &mut dyn SourceLoader,
    unit_storage: &mut dyn UnitEncoder,
) -> Result<(), ()> {
    // Shared id generator.
    let gen = Gen::new();
    let mut consts = Consts::default();
    let mut storage = Storage::default();
    let mut inner = Default::default();

    let q = Query::new(
        unit,
        prelude,
        &mut consts,
        &mut storage,
        sources,
        pool,
        visitor,
        diagnostics,
        &gen,
        context,
        &mut inner,
    );

    // The worker queue.
    let mut worker = Worker::new(options, source_loader, q);

    // Queue up the initial sources to be loaded.
    for source_id in worker.q.sources.source_ids() {
        // Unique identifier for the root module in this source context.
        let root_item_id = worker.q.gen.next();

        let mod_item = match worker
            .q
            .insert_root_mod(root_item_id, source_id, Span::empty())
        {
            Ok(result) => result,
            Err(error) => {
                worker.q.diagnostics.error(source_id, error);
                return Err(());
            }
        };

        worker.queue.push_back(Task::LoadFile {
            kind: LoadFileKind::Root,
            source_id,
            mod_item,
            mod_item_id: root_item_id,
        });
    }

    worker.run();

    if worker.q.diagnostics.has_error() {
        return Err(());
    }

    loop {
        while let Some(entry) = worker.q.next_build_entry() {
            tracing::trace!("next build entry: {}", entry.item_meta.item);
            let source_id = entry.item_meta.location.source_id;

            let task = CompileBuildEntry {
                options,
                q: worker.q.borrow(),
            };

            if let Err(error) = task.compile(entry, unit_storage) {
                worker.q.diagnostics.error(source_id, error);
            }
        }

        match worker.q.queue_unused_entries() {
            Ok(true) => (),
            Ok(false) => break,
            Err((source_id, error)) => {
                worker.q.diagnostics.error(source_id, error);
            }
        }
    }

    if worker.q.diagnostics.has_error() {
        return Err(());
    }

    Ok(())
}

struct CompileBuildEntry<'a> {
    options: &'a Options,
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
            q: self.q.borrow(),
            asm,
            scopes: self::v1::Scopes::new(),
            contexts: vec![span],
            loops: self::v1::Loops::new(),
            options: self.options,
        }
    }

    fn compile(mut self, entry: BuildEntry, unit_storage: &mut dyn UnitEncoder) -> Result<()> {
        let BuildEntry {
            item_meta,
            build,
            used,
        } = entry;

        let location = item_meta.location;

        let mut asm = self.q.unit.new_assembly(location);

        match build {
            Build::Query => {
                tracing::trace!("query: {}", self.q.pool.item(item_meta.item));

                if self
                    .q
                    .query_meta(item_meta.location.span, item_meta.item, used)?
                    .is_none()
                {
                    return Err(Error::new(
                        item_meta.location.span,
                        CompileErrorKind::MissingItem {
                            item: self.q.pool.item(item_meta.item).to_owned(),
                        },
                    ));
                }
            }
            Build::Function(f) => {
                tracing::trace!("function: {}", self.q.pool.item(item_meta.item));

                use self::v1::assemble;

                let args =
                    format_fn_args(self.q.sources, location, f.ast.args.iter().map(|(a, _)| a))?;

                let span = f.ast.span();
                let count = f.ast.args.len();

                let arena = hir::Arena::new();
                let mut ctx =
                    hir::lowering::Ctx::new(&arena, self.q.borrow(), item_meta.location.source_id);
                let hir = hir::lowering::item_fn(&mut ctx, &f.ast)?;
                let mut c = self.compiler1(location, span, &mut asm);
                assemble::fn_from_item_fn(&hir, &mut c, false)?;

                if used.is_unused() {
                    self.q.diagnostics.not_used(location.source_id, span, None);
                } else {
                    self.q.unit.new_function(
                        location,
                        self.q.pool.item(item_meta.item),
                        count,
                        asm,
                        f.call,
                        args,
                        unit_storage,
                    )?;
                }
            }
            Build::InstanceFunction(f) => {
                tracing::trace!("instance function: {}", self.q.pool.item(item_meta.item));

                use self::v1::assemble;

                let args =
                    format_fn_args(self.q.sources, location, f.ast.args.iter().map(|(a, _)| a))?;

                let span = f.ast.span();
                let count = f.ast.args.len();

                let mut c = self.compiler1(location, span, &mut asm);
                let meta = c.lookup_meta(
                    f.instance_span,
                    f.impl_item,
                    self::v1::GenericsParameters::default(),
                )?;

                let type_hash = meta.type_hash_of().ok_or_else(|| {
                    Error::expected_meta(span, meta.info(c.q.pool), "instance function")
                })?;

                let arena = hir::Arena::new();
                let mut ctx =
                    hir::lowering::Ctx::new(&arena, c.q.borrow(), item_meta.location.source_id);
                let hir = hir::lowering::item_fn(&mut ctx, &f.ast)?;
                assemble::fn_from_item_fn(&hir, &mut c, true)?;

                if used.is_unused() {
                    c.q.diagnostics.not_used(location.source_id, span, None);
                } else {
                    let name = f.ast.name.resolve(resolve_context!(self.q))?;

                    self.q.unit.new_instance_function(
                        location,
                        self.q.pool.item(item_meta.item),
                        type_hash,
                        name,
                        count,
                        asm,
                        f.call,
                        args,
                        unit_storage,
                    )?;
                }
            }
            Build::Closure(closure) => {
                tracing::trace!("closure: {}", self.q.pool.item(item_meta.item));

                use self::v1::assemble;

                let span = closure.ast.span();
                let args = format_fn_args(
                    self.q.sources,
                    location,
                    closure.ast.args.as_slice().iter().map(|(a, _)| a),
                )?;

                let arena = hir::Arena::new();
                let mut ctx =
                    hir::lowering::Ctx::new(&arena, self.q.borrow(), item_meta.location.source_id);
                let hir = hir::lowering::expr_closure(&mut ctx, &closure.ast)?;
                let mut c = self.compiler1(location, span, &mut asm);
                assemble::closure_from_expr_closure(span, &mut c, &hir, &closure.captures)?;

                if used.is_unused() {
                    c.q.diagnostics
                        .not_used(location.source_id, location.span, None);
                } else {
                    self.q.unit.new_function(
                        location,
                        self.q.pool.item(item_meta.item),
                        closure.ast.args.len(),
                        asm,
                        closure.call,
                        args,
                        unit_storage,
                    )?;
                }
            }
            Build::AsyncBlock(b) => {
                tracing::trace!("async block: {}", self.q.pool.item(item_meta.item));

                use self::v1::assemble;

                let args = b.captures.len();
                let span = b.ast.span();

                let arena = hir::Arena::new();
                let mut ctx =
                    hir::lowering::Ctx::new(&arena, self.q.borrow(), item_meta.location.source_id);
                let hir = hir::lowering::block(&mut ctx, &b.ast)?;

                let mut c = self.compiler1(location, span, &mut asm);
                assemble::closure_from_block(&hir, &mut c, &b.captures)?;

                if used.is_unused() {
                    self.q
                        .diagnostics
                        .not_used(location.source_id, location.span, None);
                } else {
                    self.q.unit.new_function(
                        location,
                        self.q.pool.item(item_meta.item),
                        args,
                        asm,
                        b.call,
                        Default::default(),
                        unit_storage,
                    )?;
                }
            }
            Build::Unused => {
                tracing::trace!("unused: {}", self.q.pool.item(item_meta.item));

                if !item_meta.visibility.is_public() {
                    self.q
                        .diagnostics
                        .not_used(location.source_id, location.span, None);
                }
            }
            Build::Import(import) => {
                tracing::trace!("import: {}", self.q.pool.item(item_meta.item));

                // Issue the import to check access.
                let result =
                    self.q
                        .import(location.span, item_meta.module, item_meta.item, used)?;

                if used.is_unused() {
                    self.q
                        .diagnostics
                        .not_used(location.source_id, location.span, None);
                }

                let missing = match result {
                    Some(item_id) => {
                        let item = self.q.pool.item(item_id);

                        if self.q.context.contains_prefix(item) || self.q.contains_prefix(item) {
                            None
                        } else {
                            Some(item_id)
                        }
                    }
                    None => Some(import.entry.target),
                };

                if let Some(item) = missing {
                    return Err(Error::new(
                        location.span,
                        CompileErrorKind::MissingItem {
                            item: self.q.pool.item(item).to_owned(),
                        },
                    ));
                }
            }
            Build::ReExport => {
                tracing::trace!("re-export: {}", self.q.pool.item(item_meta.item));

                let import =
                    match self
                        .q
                        .import(location.span, item_meta.module, item_meta.item, used)?
                    {
                        Some(item) => item,
                        None => {
                            return Err(Error::new(
                                location.span,
                                CompileErrorKind::MissingItem {
                                    item: self.q.pool.item(item_meta.item).to_owned(),
                                },
                            ))
                        }
                    };

                self.q.unit.new_function_reexport(
                    location,
                    self.q.pool.item(item_meta.item),
                    self.q.pool.item(import),
                )?;
            }
        }

        Ok(())
    }
}

fn format_fn_args<'a, I>(
    sources: &Sources,
    location: Location,
    arguments: I,
) -> Result<Box<[Box<str>]>>
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
