use crate::alloc::prelude::*;
use crate::alloc::{self, try_vec, Box, Vec};
use crate::ast;
use crate::ast::{Span, Spanned};
use crate::compile::v1;
use crate::compile::{
    self, Assembly, CompileVisitor, Context, ErrorKind, Location, Options, Pool, Prelude,
    SourceLoader, UnitBuilder,
};
use crate::hir;
use crate::indexing::FunctionAst;
use crate::macros::Storage;
use crate::parse::Resolve;
use crate::query::{Build, BuildEntry, GenericsParameters, Query, Used};
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
) -> alloc::Result<()> {
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
                worker.q.diagnostics.error(source_id, error)?;
                continue;
            }
        };

        let result = worker.queue.try_push_back(Task::LoadFile {
            kind: LoadFileKind::Root,
            source_id,
            mod_item,
            mod_item_id: root_item_id,
        });

        if let Err(error) = result {
            worker
                .q
                .diagnostics
                .error(source_id, compile::Error::from(error))?;
        }
    }

    worker.index()?;

    if worker.q.diagnostics.has_error() {
        return Ok(());
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
                worker.q.diagnostics.error(source_id, error)?;
            }
        }

        let mut errors = Vec::new();

        if worker.q.queue_unused_entries(&mut errors)? {
            break;
        }

        for (source_id, error) in errors {
            worker.q.diagnostics.error(source_id, error)?;
        }
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
    ) -> alloc::Result<v1::Ctxt<'a, 'hir, 'arena>> {
        Ok(v1::Ctxt {
            source_id: location.source_id,
            q: self.q.borrow(),
            asm,
            scopes: self::v1::Scopes::new(location.source_id)?,
            contexts: try_vec![span.span()],
            loops: self::v1::Loops::new(),
            options: self.options,
        })
    }

    #[tracing::instrument(skip_all)]
    fn compile(
        mut self,
        entry: BuildEntry,
        unit_storage: &mut dyn UnitEncoder,
    ) -> compile::Result<()> {
        let BuildEntry { item_meta, build } = entry;

        let location = item_meta.location;

        let mut asm = self.q.unit.new_assembly(location);

        match build {
            Build::Query => {
                tracing::trace!("query: {}", self.q.pool.item(item_meta.item));

                let used = if self.q.is_used(&item_meta) {
                    Used::Used
                } else {
                    Used::Unused
                };

                if self
                    .q
                    .query_meta(&item_meta.location, item_meta.item, used)?
                    .is_none()
                {
                    return Err(compile::Error::new(
                        item_meta.location.span,
                        ErrorKind::MissingItem {
                            item: self.q.pool.item(item_meta.item).try_to_owned()?,
                        },
                    ));
                }
            }
            Build::Function(f) => {
                tracing::trace!("function: {}", self.q.pool.item(item_meta.item));

                use self::v1::assemble;

                // For instance functions, we are required to know the type hash
                // of the type it is associated with to perform the proper
                // naming of the function.
                let type_hash = if f.is_instance {
                    let Some(impl_item) =
                        f.impl_item.and_then(|item| self.q.inner.items.get(&item))
                    else {
                        return Err(compile::Error::msg(
                            &f.ast,
                            "Impl item has not been expanded",
                        ));
                    };

                    let meta = self.q.lookup_meta(
                        &location,
                        impl_item.item,
                        GenericsParameters::default(),
                    )?;

                    let Some(type_hash) = meta.type_hash_of() else {
                        return Err(compile::Error::expected_meta(
                            &f.ast,
                            meta.info(self.q.pool)?,
                            "type for associated function",
                        ));
                    };

                    Some(type_hash)
                } else {
                    None
                };

                let (args, span): (_, &dyn Spanned) = match &f.ast {
                    FunctionAst::Item(ast) => {
                        let args = format_fn_args(
                            self.q.sources,
                            location,
                            ast.args.iter().map(|(a, _)| a),
                        )?;
                        (args, ast)
                    }
                    FunctionAst::Empty(.., span) => (Box::default(), span),
                };

                let arena = hir::Arena::new();

                let mut cx = hir::lowering::Ctxt::with_query(
                    &arena,
                    self.q.borrow(),
                    item_meta.location.source_id,
                )?;

                let hir = match &f.ast {
                    FunctionAst::Item(ast) => hir::lowering::item_fn(&mut cx, ast)?,
                    FunctionAst::Empty(ast, span) => hir::lowering::empty_fn(&mut cx, ast, &span)?,
                };

                let count = hir.args.len();

                let mut c = self.compiler1(location, span, &mut asm)?;
                assemble::fn_from_item_fn(&mut c, &hir, f.is_instance)?;

                if !self.q.is_used(&item_meta) {
                    self.q
                        .diagnostics
                        .not_used(location.source_id, span, None)?;
                } else {
                    let instance = match (type_hash, &f.ast) {
                        (Some(type_hash), FunctionAst::Item(ast)) => {
                            let name = ast.name.resolve(resolve_context!(self.q))?;
                            Some((type_hash, name))
                        }
                        _ => None,
                    };

                    self.q.unit.new_function(
                        location,
                        self.q.pool.item(item_meta.item),
                        instance,
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
                )?;
                let hir = hir::lowering::expr_closure_secondary(&mut cx, &closure.ast, captures)?;
                let mut c = self.compiler1(location, &closure.ast, &mut asm)?;
                assemble::expr_closure_secondary(&mut c, &hir, &closure.ast)?;

                if !c.q.is_used(&item_meta) {
                    c.q.diagnostics
                        .not_used(location.source_id, &location.span, None)?;
                } else {
                    self.q.unit.new_function(
                        location,
                        self.q.pool.item(item_meta.item),
                        None,
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
                )?;
                let hir = hir::lowering::async_block_secondary(&mut cx, &b.ast, captures)?;
                let mut c = self.compiler1(location, &b.ast, &mut asm)?;
                assemble::async_block_secondary(&mut c, &hir)?;

                if !self.q.is_used(&item_meta) {
                    self.q
                        .diagnostics
                        .not_used(location.source_id, &location.span, None)?;
                } else {
                    let args = hir.captures.len();

                    self.q.unit.new_function(
                        location,
                        self.q.pool.item(item_meta.item),
                        None,
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
                        .not_used(location.source_id, &location.span, None)?;
                }
            }
            Build::Import(import) => {
                tracing::trace!("import: {}", self.q.pool.item(item_meta.item));

                let used = if self.q.is_used(&item_meta) {
                    Used::Used
                } else {
                    Used::Unused
                };

                // Issue the import to check access.
                let result =
                    self.q
                        .import(&location, item_meta.module, item_meta.item, used, used)?;

                if !self.q.is_used(&item_meta) {
                    self.q
                        .diagnostics
                        .not_used(location.source_id, &location.span, None)?;
                }

                let missing = match result {
                    Some(item_id) => {
                        let item = self.q.pool.item(item_id);

                        if self.q.context.contains_prefix(item)? || self.q.contains_prefix(item)? {
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
                            item: self.q.pool.item(item).try_to_owned()?,
                        },
                    ));
                }
            }
            Build::ReExport => {
                tracing::trace!("re-export: {}", self.q.pool.item(item_meta.item));

                let used = if self.q.is_used(&item_meta) {
                    Used::Used
                } else {
                    Used::Unused
                };

                let Some(import) =
                    self.q
                        .import(&location, item_meta.module, item_meta.item, used, used)?
                else {
                    return Err(compile::Error::new(
                        location.span,
                        ErrorKind::MissingItem {
                            item: self.q.pool.item(item_meta.item).try_to_owned()?,
                        },
                    ));
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
                args.try_push(Box::try_from("self")?)?;
            }
            ast::FnArg::Pat(pat) => {
                let span = pat.span();

                if let Some(s) = sources.source(location.source_id, span) {
                    args.try_push(Box::try_from(s)?)?;
                } else {
                    args.try_push(Box::try_from("*")?)?;
                }
            }
        }
    }

    Ok(args.try_into_boxed_slice()?)
}
