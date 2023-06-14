use crate::no_std::prelude::*;

use crate::ast;
use crate::ast::{Span, Spanned};
use crate::compile::v1;
use crate::compile::{
    self, Assembly, CompileVisitor, Context, ErrorKind, Location, Options, Pool, Prelude,
    SourceLoader, UnitBuilder,
};
use crate::hir;
use crate::macros::Storage;
use crate::parse::Resolve;
use crate::query::{Build, BuildEntry, GenericsParameters, Query};
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
    visitor: &mut dyn CompileVisitor,
    diagnostics: &mut Diagnostics,
    source_loader: &mut dyn SourceLoader,
    options: &Options,
    unit_storage: &mut dyn UnitEncoder,
) -> Result<(), ()> {
    // Shared id generator.
    let gen = Gen::new();
    let const_arena = hir::Arena::new();
    let mut consts = Consts::default();
    let mut storage = Storage::default();
    let mut inner = Default::default();

    let q = Query::new(
        unit,
        prelude,
        &const_arena,
        &mut consts,
        &mut storage,
        sources,
        pool,
        visitor,
        diagnostics,
        source_loader,
        options,
        &gen,
        context,
        &mut inner,
    );

    // The worker queue.
    let mut worker = Worker::new(q);

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
            tracing::trace!(item = ?worker.q.pool.item(entry.item_meta.item), "next build entry");
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

struct CompileBuildEntry<'a, 'arena> {
    options: &'a Options,
    q: Query<'a, 'arena>,
}

impl<'arena> CompileBuildEntry<'_, 'arena> {
    fn compiler1<'a, 'hir>(
        &'a mut self,
        location: Location,
        span: &dyn Spanned,
        asm: &'a mut Assembly,
    ) -> v1::Ctxt<'a, 'hir, 'arena> {
        v1::Ctxt {
            source_id: location.source_id,
            q: self.q.borrow(),
            asm,
            scopes: self::v1::Scopes::new(location.source_id),
            contexts: vec![span.span()],
            loops: self::v1::Loops::new(),
            options: self.options,
        }
    }

    #[tracing::instrument(skip_all)]
    fn compile(
        mut self,
        entry: BuildEntry,
        unit_storage: &mut dyn UnitEncoder,
    ) -> compile::Result<()> {
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
                    .query_meta(&item_meta.location, item_meta.item, used)?
                    .is_none()
                {
                    return Err(compile::Error::new(
                        item_meta.location.span,
                        ErrorKind::MissingItem {
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

                let span = &*f.ast;
                let count = f.ast.args.len();

                let arena = hir::Arena::new();
                let mut cx = hir::lowering::Ctxt::with_query(
                    &arena,
                    self.q.borrow(),
                    item_meta.location.source_id,
                );
                let hir = hir::lowering::item_fn(&mut cx, &f.ast)?;
                let mut c = self.compiler1(location, span, &mut asm);
                assemble::fn_from_item_fn(&mut c, &hir, false)?;

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

                let count = f.ast.args.len();

                let arena = hir::Arena::new();
                let mut c = self.compiler1(location, &f.ast, &mut asm);
                let meta =
                    c.q.lookup_meta(&location, f.impl_item, GenericsParameters::default())?;

                let Some(type_hash) = meta.type_hash_of() else {
                    return Err(compile::Error::expected_meta(&f.ast, meta.info(c.q.pool), "instance function"));
                };

                let mut cx = hir::lowering::Ctxt::with_query(
                    &arena,
                    c.q.borrow(),
                    item_meta.location.source_id,
                );
                let hir = hir::lowering::item_fn(&mut cx, &f.ast)?;
                assemble::fn_from_item_fn(&mut c, &hir, true)?;

                if used.is_unused() {
                    c.q.diagnostics.not_used(location.source_id, &f.ast, None);
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

                let args = format_fn_args(
                    self.q.sources,
                    location,
                    closure.ast.args.as_slice().iter().map(|(a, _)| a),
                )?;

                let captures = self.q.pool.item_type_hash(item_meta.item);

                let arena = hir::Arena::new();
                let mut cx = hir::lowering::Ctxt::with_query(
                    &arena,
                    self.q.borrow(),
                    item_meta.location.source_id,
                );
                let hir = hir::lowering::expr_closure_secondary(&mut cx, &closure.ast, captures)?;
                let mut c = self.compiler1(location, &closure.ast, &mut asm);
                assemble::expr_closure_secondary(&mut c, &hir, &closure.ast)?;

                if used.is_unused() {
                    c.q.diagnostics
                        .not_used(location.source_id, &location.span, None);
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

                let captures = self.q.pool.item_type_hash(item_meta.item);

                let arena = hir::Arena::new();
                let mut cx = hir::lowering::Ctxt::with_query(
                    &arena,
                    self.q.borrow(),
                    item_meta.location.source_id,
                );
                let hir = hir::lowering::async_block_secondary(&mut cx, &b.ast, captures)?;
                let mut c = self.compiler1(location, &b.ast, &mut asm);
                assemble::async_block_secondary(&mut c, &hir)?;

                if used.is_unused() {
                    self.q
                        .diagnostics
                        .not_used(location.source_id, &location.span, None);
                } else {
                    let args = hir.captures.len();

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
                        .not_used(location.source_id, &location.span, None);
                }
            }
            Build::Import(import) => {
                tracing::trace!("import: {}", self.q.pool.item(item_meta.item));

                // Issue the import to check access.
                let result = self
                    .q
                    .import(&location, item_meta.module, item_meta.item, used)?;

                if used.is_unused() {
                    self.q
                        .diagnostics
                        .not_used(location.source_id, &location.span, None);
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
                    return Err(compile::Error::new(
                        location,
                        ErrorKind::MissingItem {
                            item: self.q.pool.item(item).to_owned(),
                        },
                    ));
                }
            }
            Build::ReExport => {
                tracing::trace!("re-export: {}", self.q.pool.item(item_meta.item));

                let Some(import) = self.q.import(&location, item_meta.module, item_meta.item, used)? else {
                    return Err(compile::Error::new(
                        location.span,
                        ErrorKind::MissingItem {
                            item: self.q.pool.item(item_meta.item).to_owned(),
                        },
                    ))
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
) -> compile::Result<Box<[Box<str>]>>
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
