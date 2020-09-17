use crate::ast;
use crate::collections::HashMap;
use crate::compile_visitor::NoopCompileVisitor;
use crate::items::Items;
use crate::loops::Loops;
use crate::query::{Build, BuildEntry, Query};
use crate::scopes::{Scope, ScopeGuard, Scopes};
use crate::traits::Compile as _;
use crate::worker::{Expanded, LoadFileKind, Task, Worker};
use crate::CompileResult;
use crate::{
    Assembly, CompileError, CompileErrorKind, CompileVisitor, Errors, FileSourceLoader, LoadError,
    Options, Resolve as _, SourceLoader, Sources, Spanned as _, Storage, UnitBuilder, Warnings,
};
use runestick::{
    CompileMeta, CompileMetaKind, Context, Inst, InstValue, Item, Label, Source, Span, TypeCheck,
};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::Arc;

/// A needs hint for an expression.
/// This is used to contextually determine what an expression is expected to
/// produce.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Needs {
    Type,
    Value,
    None,
}

impl Needs {
    /// Test if any sort of value is needed.
    pub(crate) fn value(self) -> bool {
        matches!(self, Self::Type | Self::Value)
    }
}

/// Compile the given source with default options.
pub fn compile(
    context: &Context,
    sources: &mut Sources,
    unit: &Rc<RefCell<UnitBuilder>>,
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
    unit: &Rc<RefCell<UnitBuilder>>,
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

    verify_imports(worker.errors, context, &mut *unit.borrow_mut())?;

    loop {
        while let Some(entry) = worker.query.queue.pop_front() {
            let source_id = entry.source_id;

            if let Err(error) = compile_entry(CompileEntryArgs {
                context,
                options,
                storage: &storage,
                unit,
                errors: worker.errors,
                warnings: worker.warnings,
                query: &mut worker.query,
                entry,
                expanded: &worker.expanded,
                visitor: worker.visitor,
            }) {
                worker.errors.push(LoadError::new(source_id, error));
            }
        }

        match worker.query.queue_unused_entries(worker.visitor) {
            Ok(true) => (),
            Ok(false) => break,
            Err((source_id, error)) => {
                worker.errors.push(LoadError::new(source_id, error));
            }
        }
    }

    if !worker.errors.is_empty() {
        return Err(());
    }

    Ok(())
}

struct CompileEntryArgs<'a> {
    context: &'a Context,
    options: &'a Options,
    storage: &'a Storage,
    unit: &'a Rc<RefCell<UnitBuilder>>,
    errors: &'a mut Errors,
    warnings: &'a mut Warnings,
    query: &'a mut Query,
    entry: BuildEntry,
    expanded: &'a HashMap<Item, Expanded>,
    visitor: &'a mut dyn CompileVisitor,
}

fn compile_entry(args: CompileEntryArgs<'_>) -> Result<(), CompileError> {
    let CompileEntryArgs {
        context,
        options,
        storage,
        unit,
        errors,
        warnings,
        query,
        entry,
        expanded,
        visitor,
    } = args;

    let BuildEntry {
        item,
        build,
        source,
        source_id,
        unused,
    } = entry;

    let mut asm = unit.borrow().new_assembly(source_id);

    let mut compiler = Compiler {
        storage,
        source_id,
        source: source.clone(),
        context,
        query,
        asm: &mut asm,
        items: Items::new(item.as_vec()),
        unit: unit.clone(),
        scopes: Scopes::new(),
        contexts: vec![],
        loops: Loops::new(),
        options,
        errors,
        warnings,
        expanded,
        visitor,
    };

    match build {
        Build::Function(f) => {
            let args = format_fn_args(storage, &*source, f.ast.args.items.iter().map(|(a, _)| a))?;

            let span = f.ast.span();
            let count = f.ast.args.items.len();
            compiler.contexts.push(span);
            compiler.compile((f.ast, false))?;

            if unused {
                compiler.warnings.not_used(source_id, span, None);
            } else {
                unit.borrow_mut()
                    .new_function(source_id, item, count, asm, f.call, args)?;
            }
        }
        Build::InstanceFunction(f) => {
            let args = format_fn_args(storage, &*source, f.ast.args.items.iter().map(|(a, _)| a))?;

            let span = f.ast.span();
            let count = f.ast.args.items.len();
            compiler.contexts.push(span);

            let source = compiler.source.clone();
            let name = f.ast.name.resolve(storage, &*source)?;

            let meta = compiler
                .lookup_meta(&f.impl_item, f.instance_span)?
                .ok_or_else(|| {
                    CompileError::new(
                        f.instance_span,
                        CompileErrorKind::MissingType {
                            item: f.impl_item.clone(),
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

            if unused {
                compiler.warnings.not_used(source_id, span, None);
            } else {
                unit.borrow_mut().new_instance_function(
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
                storage,
                &*source,
                c.ast.args.as_slice().iter().map(|(a, _)| a),
            )?;

            let span = c.ast.span();
            let count = c.ast.args.len();
            compiler.contexts.push(span);
            compiler.compile((c.ast, &c.captures[..]))?;

            if unused {
                compiler.warnings.not_used(source_id, span, None);
            } else {
                unit.borrow_mut()
                    .new_function(source_id, item, count, asm, c.call, args)?;
            }
        }
        Build::AsyncBlock(async_block) => {
            let span = async_block.ast.span();
            let args = async_block.captures.len();
            compiler.contexts.push(span);
            compiler.compile((&async_block.ast, &async_block.captures[..]))?;

            if unused {
                compiler.warnings.not_used(source_id, span, None);
            } else {
                unit.borrow_mut().new_function(
                    source_id,
                    item,
                    args,
                    asm,
                    async_block.call,
                    Vec::new(),
                )?;
            }
        }
    }

    Ok(())
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
            ast::FnArg::Self_(..) => {
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

fn verify_imports(
    errors: &mut Errors,
    context: &Context,
    unit: &mut UnitBuilder,
) -> Result<(), ()> {
    for (_, entry) in unit.iter_imports() {
        if context.contains_prefix(&entry.item) || unit.contains_prefix(&entry.item) {
            continue;
        }

        if let Some((span, source_id)) = entry.span {
            errors.push(LoadError::new(
                source_id,
                CompileError::new(
                    span,
                    CompileErrorKind::MissingModule {
                        item: entry.item.clone(),
                    },
                ),
            ));

            return Err(());
        } else {
            errors.push(LoadError::new(
                0,
                CompileError::new(
                    Span::empty(),
                    CompileErrorKind::MissingPreludeModule {
                        item: entry.item.clone(),
                    },
                ),
            ));

            return Err(());
        }
    }

    Ok(())
}

pub(crate) struct Compiler<'a> {
    /// The source id of the source.
    pub(crate) source_id: usize,
    /// The source we are compiling for.
    pub(crate) source: Arc<Source>,
    /// The current macro context.
    pub(crate) storage: &'a Storage,
    /// The context we are compiling for.
    context: &'a Context,
    /// Items expanded by macros.
    pub(crate) expanded: &'a HashMap<Item, Expanded>,
    /// Query system to compile required items.
    pub(crate) query: &'a mut Query,
    /// The assembly we are generating.
    pub(crate) asm: &'a mut Assembly,
    /// Item builder.
    pub(crate) items: Items,
    /// The compilation unit we are compiling for.
    pub(crate) unit: Rc<RefCell<UnitBuilder>>,
    /// Scopes defined in the compiler.
    pub(crate) scopes: Scopes,
    /// Context for which to emit warnings.
    pub(crate) contexts: Vec<Span>,
    /// The nesting of loop we are currently in.
    pub(crate) loops: Loops,
    /// Enabled optimizations.
    pub(crate) options: &'a Options,
    /// Compilation warnings.
    #[allow(unused)]
    pub(crate) errors: &'a mut Errors,
    /// Compilation warnings.
    pub(crate) warnings: &'a mut Warnings,
    /// Compiler visitor.
    pub(crate) visitor: &'a mut dyn CompileVisitor,
}

impl<'a> Compiler<'a> {
    /// Access the meta for the given language item.
    pub fn lookup_meta(&mut self, name: &Item, span: Span) -> CompileResult<Option<CompileMeta>> {
        log::trace!("lookup meta: {}", name);

        if let Some(meta) = self.context.lookup_meta(name) {
            log::trace!("found in context: {:?}", meta);
            self.visitor.visit_meta(self.source_id, &meta, span);
            return Ok(Some(meta));
        }

        let mut base = self.items.item();

        loop {
            let current = base.join(name);
            log::trace!("lookup meta (query): {}", current);

            if let Some(meta) = self.query.query_meta(&current)? {
                log::trace!("found in query: {:?}", meta);
                self.visitor.visit_meta(self.source_id, &meta, span);
                return Ok(Some(meta));
            }

            if base.pop().is_none() {
                break;
            }
        }

        Ok(None)
    }

    /// Pop locals by simply popping them.
    pub(crate) fn locals_pop(&mut self, total_var_count: usize, span: Span) {
        match total_var_count {
            0 => (),
            1 => {
                self.asm.push(Inst::Pop, span);
            }
            count => {
                self.asm.push(Inst::PopN { count }, span);
            }
        }
    }

    /// Clean up local variables by preserving the value that is on top and
    /// popping the rest.
    ///
    /// The clean operation will preserve the value that is on top of the stack,
    /// and pop the values under it.
    pub(crate) fn locals_clean(&mut self, total_var_count: usize, span: Span) {
        match total_var_count {
            0 => (),
            count => {
                self.asm.push(Inst::Clean { count }, span);
            }
        }
    }

    /// Compile an item.
    pub(crate) fn compile_meta(
        &mut self,
        meta: &CompileMeta,
        span: Span,
        needs: Needs,
    ) -> CompileResult<()> {
        log::trace!("CompileMeta => {:?} {:?}", meta, needs);
        if let Needs::Value = needs {
            match &meta.kind {
                CompileMetaKind::Tuple { tuple, .. } if tuple.args == 0 => {
                    self.asm.push_with_comment(
                        Inst::Call {
                            hash: tuple.hash,
                            args: 0,
                        },
                        span,
                        format!("tuple `{}`", tuple.item),
                    );
                }
                CompileMetaKind::TupleVariant {
                    enum_item, tuple, ..
                } if tuple.args == 0 => {
                    self.asm.push_with_comment(
                        Inst::Call {
                            hash: tuple.hash,
                            args: 0,
                        },
                        span,
                        format!("tuple variant `{}::{}`", enum_item, tuple.item),
                    );
                }
                CompileMetaKind::Tuple { tuple, .. } => {
                    self.asm.push_with_comment(
                        Inst::LoadFn { hash: tuple.hash },
                        span,
                        format!("tuple `{}`", tuple.item),
                    );
                }
                CompileMetaKind::TupleVariant {
                    enum_item, tuple, ..
                } => {
                    self.asm.push_with_comment(
                        Inst::LoadFn { hash: tuple.hash },
                        span,
                        format!("tuple variant `{}::{}`", enum_item, tuple.item),
                    );
                }
                CompileMetaKind::Function { type_of, item, .. } => {
                    let hash = **type_of;
                    self.asm.push_with_comment(
                        Inst::LoadFn { hash },
                        span,
                        format!("fn `{}`", item),
                    );
                }
                _ => {
                    return Err(CompileError::new(
                        span,
                        CompileErrorKind::UnsupportedValue { meta: meta.clone() },
                    ));
                }
            }

            return Ok(());
        }

        let type_of = meta.type_of().ok_or_else(|| {
            CompileError::new(
                span,
                CompileErrorKind::UnsupportedType { meta: meta.clone() },
            )
        })?;

        let hash = *type_of;
        self.asm.push(
            Inst::Push {
                value: InstValue::Type(hash),
            },
            span,
        );
        Ok(())
    }

    /// Convert a path to an item.
    pub(crate) fn convert_path_to_item(&self, path: &ast::Path) -> CompileResult<Item> {
        let base = self.items.item();
        self.unit
            .borrow()
            .convert_path(&base, path, &self.storage, &*self.source)
    }

    pub(crate) fn compile_condition(
        &mut self,
        condition: &ast::Condition,
        then_label: Label,
    ) -> CompileResult<Scope> {
        let span = condition.span();
        log::trace!("Condition => {:?}", self.source.source(span));

        match condition {
            ast::Condition::Expr(expr) => {
                let span = expr.span();

                self.compile((&**expr, Needs::Value))?;
                self.asm.jump_if(then_label, span);

                Ok(self.scopes.child(span)?)
            }
            ast::Condition::ExprLet(expr_let) => {
                let span = expr_let.span();

                let false_label = self.asm.new_label("if_condition_false");

                let scope = self.scopes.child(span)?;
                let expected = self.scopes.push(scope);

                let load = |this: &mut Self, needs: Needs| {
                    this.compile((&*expr_let.expr, needs))?;
                    Ok(())
                };

                if self.compile_pat(&expr_let.pat, false_label, &load)? {
                    self.asm.jump(then_label, span);
                    self.asm.label(false_label)?;
                } else {
                    self.asm.jump(then_label, span);
                };

                let scope = self.scopes.pop(expected, span)?;
                Ok(scope)
            }
        }
    }

    /// Encode a vector pattern match.
    pub(crate) fn compile_pat_vec(
        &mut self,
        pat_vec: &ast::PatVec,
        false_label: Label,
        load: &dyn Fn(&mut Self, Needs) -> CompileResult<()>,
    ) -> CompileResult<()> {
        let span = pat_vec.span();
        log::trace!("PatVec => {:?}", self.source.source(span));

        // Assign the yet-to-be-verified tuple to an anonymous slot, so we can
        // interact with it multiple times.
        load(self, Needs::Value)?;
        let offset = self.scopes.decl_anon(span)?;

        // Copy the temporary and check that its length matches the pattern and
        // that it is indeed a vector.
        self.asm.push(Inst::Copy { offset }, span);

        self.asm.push(
            Inst::MatchSequence {
                type_check: TypeCheck::Vec,
                len: pat_vec.items.len(),
                exact: pat_vec.open_pattern.is_none(),
            },
            span,
        );

        self.asm
            .pop_and_jump_if_not(self.scopes.local_var_count(span)?, false_label, span);

        for (index, (pat, _)) in pat_vec.items.iter().enumerate() {
            let span = pat.span();

            let load = move |this: &mut Self, needs: Needs| {
                if needs.value() {
                    this.asm.push(Inst::TupleIndexGetAt { offset, index }, span);
                }

                Ok(())
            };

            self.compile_pat(&*pat, false_label, &load)?;
        }

        Ok(())
    }

    /// Encode a vector pattern match.
    pub(crate) fn compile_pat_tuple(
        &mut self,
        pat_tuple: &ast::PatTuple,
        false_label: Label,
        load: &dyn Fn(&mut Self, Needs) -> CompileResult<()>,
    ) -> CompileResult<()> {
        let span = pat_tuple.span();
        log::trace!("PatTuple => {:?}", self.source.source(span));

        // Assign the yet-to-be-verified tuple to an anonymous slot, so we can
        // interact with it multiple times.
        load(self, Needs::Value)?;
        let offset = self.scopes.decl_anon(span)?;

        let type_check = if let Some(path) = &pat_tuple.path {
            let item = self.convert_path_to_item(path)?;

            let (tuple, meta, type_check) =
                if let Some(meta) = self.lookup_meta(&item, path.span())? {
                    match &meta.kind {
                        CompileMetaKind::Tuple { tuple, type_of, .. } => {
                            let type_check = TypeCheck::Type(**type_of);
                            (tuple.clone(), meta, type_check)
                        }
                        CompileMetaKind::TupleVariant { tuple, type_of, .. } => {
                            let type_check = TypeCheck::Variant(**type_of);
                            (tuple.clone(), meta, type_check)
                        }
                        _ => {
                            return Err(CompileError::new(
                                span,
                                CompileErrorKind::UnsupportedMetaPattern { meta },
                            ))
                        }
                    }
                } else {
                    return Err(CompileError::new(
                        span,
                        CompileErrorKind::UnsupportedPattern,
                    ));
                };

            let count = pat_tuple.items.len();
            let is_open = pat_tuple.open_pattern.is_some();

            if !(tuple.args == count || count < tuple.args && is_open) {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::UnsupportedArgumentCount {
                        meta,
                        expected: tuple.args,
                        actual: count,
                    },
                ));
            }

            match self.context.type_check_for(&tuple.item) {
                Some(type_check) => type_check,
                None => type_check,
            }
        } else {
            TypeCheck::Tuple
        };

        self.asm.push(Inst::Copy { offset }, span);
        self.asm.push(
            Inst::MatchSequence {
                type_check,
                len: pat_tuple.items.len(),
                exact: pat_tuple.open_pattern.is_none(),
            },
            span,
        );
        self.asm
            .pop_and_jump_if_not(self.scopes.local_var_count(span)?, false_label, span);

        for (index, (pat, _)) in pat_tuple.items.iter().enumerate() {
            let span = pat.span();

            let load = move |this: &mut Self, needs: Needs| {
                if needs.value() {
                    this.asm.push(Inst::TupleIndexGetAt { offset, index }, span);
                }

                Ok(())
            };

            self.compile_pat(&*pat, false_label, &load)?;
        }

        Ok(())
    }

    /// Encode an object pattern match.
    pub(crate) fn compile_pat_object(
        &mut self,
        pat_object: &ast::PatObject,
        false_label: Label,
        load: &dyn Fn(&mut Self, Needs) -> CompileResult<()>,
    ) -> CompileResult<()> {
        let span = pat_object.span();
        log::trace!("PatObject => {:?}", self.source.source(span));

        // NB: bind the loaded variable (once) to an anonymous var.
        // We reduce the number of copy operations by having specialized
        // operations perform the load from the given offset.
        load(self, Needs::Value)?;
        let offset = self.scopes.decl_anon(span)?;

        let mut string_slots = Vec::new();

        let mut keys_dup = HashMap::new();
        let mut keys = Vec::new();

        for (item, _) in &pat_object.fields {
            let span = item.span();

            let source = self.source.clone();
            let key = item.key.resolve(&self.storage, &*source)?;
            string_slots.push(self.unit.borrow_mut().new_static_string(&*key)?);
            keys.push(key.to_string());

            if let Some(existing) = keys_dup.insert(key.to_string(), span) {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::DuplicateObjectKey {
                        existing,
                        object: pat_object.span(),
                    },
                ));
            }
        }

        let keys = self.unit.borrow_mut().new_static_object_keys(&keys[..])?;

        let type_check = match &pat_object.ident {
            ast::LitObjectIdent::Named(path) => {
                let span = path.span();
                let item = self.convert_path_to_item(path)?;

                let meta = match self.lookup_meta(&item, span)? {
                    Some(meta) => meta,
                    None => {
                        return Err(CompileError::new(
                            span,
                            CompileErrorKind::MissingType { item },
                        ));
                    }
                };

                let (object, type_check) = match &meta.kind {
                    CompileMetaKind::Struct {
                        object, type_of, ..
                    } => {
                        let type_check = TypeCheck::Type(**type_of);
                        (object, type_check)
                    }
                    CompileMetaKind::StructVariant {
                        object, type_of, ..
                    } => {
                        let type_check = TypeCheck::Variant(**type_of);
                        (object, type_check)
                    }
                    _ => {
                        return Err(CompileError::new(
                            span,
                            CompileErrorKind::UnsupportedMetaPattern { meta },
                        ));
                    }
                };

                let fields = match &object.fields {
                    Some(fields) => fields,
                    None => {
                        // NB: might want to describe that field composition is unknown because it is an external meta item.
                        return Err(CompileError::new(
                            span,
                            CompileErrorKind::UnsupportedMetaPattern { meta },
                        ));
                    }
                };

                for (field, _) in &pat_object.fields {
                    let span = field.key.span();
                    let key = field.key.resolve(&self.storage, &*self.source)?;

                    if !fields.contains(&*key) {
                        return Err(CompileError::new(
                            span,
                            CompileErrorKind::LitObjectNotField {
                                field: key.to_string(),
                                item: object.item.clone(),
                            },
                        ));
                    }
                }

                type_check
            }
            ast::LitObjectIdent::Anonymous(..) => TypeCheck::Object,
        };

        // Copy the temporary and check that its length matches the pattern and
        // that it is indeed a vector.
        self.asm.push(Inst::Copy { offset }, span);
        self.asm.push(
            Inst::MatchObject {
                type_check,
                slot: keys,
                exact: pat_object.open_pattern.is_none(),
            },
            span,
        );

        self.asm
            .pop_and_jump_if_not(self.scopes.local_var_count(span)?, false_label, span);

        for ((item, _), slot) in pat_object.fields.iter().zip(string_slots) {
            let span = item.span();

            let load = move |this: &mut Self, needs: Needs| {
                if needs.value() {
                    this.asm.push(Inst::ObjectIndexGetAt { offset, slot }, span);
                }

                Ok(())
            };

            if let Some((_, pat)) = &item.binding {
                // load the given vector index and declare it as a local variable.
                self.compile_pat(&*pat, false_label, &load)?;
                continue;
            }

            // NB: only raw identifiers are supported as anonymous bindings
            let ident = match &item.key {
                ast::LitObjectKey::Ident(ident) => ident,
                _ => {
                    return Err(CompileError::new(
                        span,
                        CompileErrorKind::UnsupportedBinding,
                    ))
                }
            };

            load(self, Needs::Value)?;
            let name = ident.resolve(&self.storage, &*self.source)?;
            self.scopes.decl_var(name.as_ref(), span)?;
        }

        Ok(())
    }

    /// Compile a binding name that matches a known meta type.
    ///
    /// Returns `true` if the binding was used.
    pub(crate) fn compile_pat_meta_binding(
        &mut self,
        span: Span,
        meta: &CompileMeta,
        false_label: Label,
        load: &dyn Fn(&mut Self, Needs) -> CompileResult<()>,
    ) -> CompileResult<bool> {
        let (tuple, type_check) = match &meta.kind {
            CompileMetaKind::Tuple { tuple, type_of, .. } if tuple.args == 0 => {
                (tuple, TypeCheck::Type(**type_of))
            }
            CompileMetaKind::TupleVariant { tuple, type_of, .. } if tuple.args == 0 => {
                (tuple, TypeCheck::Variant(**type_of))
            }
            _ => return Ok(false),
        };

        let type_check = match self.context.type_check_for(&tuple.item) {
            Some(type_check) => type_check,
            None => type_check,
        };

        load(self, Needs::Value)?;
        self.asm.push(
            Inst::MatchSequence {
                type_check,
                len: tuple.args,
                exact: true,
            },
            span,
        );
        self.asm
            .pop_and_jump_if_not(self.scopes.local_var_count(span)?, false_label, span);
        Ok(true)
    }

    /// Encode a pattern.
    ///
    /// Patterns will clean up their own locals and execute a jump to
    /// `false_label` in case the pattern does not match.
    ///
    /// Returns a boolean indicating if the label was used.
    pub(crate) fn compile_pat(
        &mut self,
        pat: &ast::Pat,
        false_label: Label,
        load: &dyn Fn(&mut Self, Needs) -> CompileResult<()>,
    ) -> CompileResult<bool> {
        let span = pat.span();
        log::trace!("Pat => {:?}", self.source.source(span));

        match pat {
            ast::Pat::PatPath(path) => {
                let span = path.span();

                let item = self.convert_path_to_item(&path.path)?;

                if let Some(meta) = self.lookup_meta(&item, span)? {
                    if self.compile_pat_meta_binding(span, &meta, false_label, load)? {
                        return Ok(true);
                    }
                }

                let ident = match item.as_local() {
                    Some(ident) => ident,
                    None => {
                        return Err(CompileError::new(
                            span,
                            CompileErrorKind::UnsupportedBinding,
                        ));
                    }
                };

                load(self, Needs::Value)?;
                self.scopes.decl_var(&ident, span)?;
                return Ok(false);
            }
            ast::Pat::PatIgnore(..) => {
                // ignore binding, but might still have side effects, so must
                // call the load generator.
                load(self, Needs::None)?;
                return Ok(false);
            }
            ast::Pat::PatUnit(unit) => {
                load(self, Needs::Value)?;
                self.asm.push(Inst::IsUnit, unit.span());
            }
            ast::Pat::PatByte(lit_byte) => {
                let byte = lit_byte.resolve(&self.storage, &*self.source)?;
                load(self, Needs::Value)?;
                self.asm.push(Inst::EqByte { byte }, lit_byte.span());
            }
            ast::Pat::PatChar(lit_char) => {
                let character = lit_char.resolve(&self.storage, &*self.source)?;
                load(self, Needs::Value)?;
                self.asm
                    .push(Inst::EqCharacter { character }, lit_char.span());
            }
            ast::Pat::PatNumber(number_literal) => {
                let span = number_literal.span();
                let number = number_literal.resolve(&self.storage, &*self.source)?;

                let integer = match number {
                    ast::Number::Integer(integer) => integer,
                    ast::Number::Float(..) => {
                        return Err(CompileError::new(
                            span,
                            CompileErrorKind::MatchFloatInPattern,
                        ));
                    }
                };

                load(self, Needs::Value)?;
                self.asm.push(Inst::EqInteger { integer }, span);
            }
            ast::Pat::PatString(pat_string) => {
                let span = pat_string.span();
                let string = pat_string.resolve(&self.storage, &*self.source)?;
                let slot = self.unit.borrow_mut().new_static_string(&*string)?;
                load(self, Needs::Value)?;
                self.asm.push(Inst::EqStaticString { slot }, span);
            }
            ast::Pat::PatVec(pat_vec) => {
                self.compile_pat_vec(pat_vec, false_label, &load)?;
                return Ok(true);
            }
            ast::Pat::PatTuple(pat_tuple) => {
                self.compile_pat_tuple(pat_tuple, false_label, &load)?;
                return Ok(true);
            }
            ast::Pat::PatObject(object) => {
                self.compile_pat_object(object, false_label, &load)?;
                return Ok(true);
            }
        }

        self.asm
            .pop_and_jump_if_not(self.scopes.local_var_count(span)?, false_label, span);
        Ok(true)
    }

    /// Clean the last scope.
    pub(crate) fn clean_last_scope(
        &mut self,
        span: Span,
        expected: ScopeGuard,
        needs: Needs,
    ) -> CompileResult<()> {
        let scope = self.scopes.pop(expected, span)?;

        if needs.value() {
            self.locals_clean(scope.local_var_count, span);
        } else {
            self.locals_pop(scope.local_var_count, span);
        }

        Ok(())
    }

    /// Get the latest relevant warning context.
    pub(crate) fn context(&self) -> Option<Span> {
        self.contexts.last().copied()
    }
}
