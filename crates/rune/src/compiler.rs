use crate::ast;
use crate::collections::HashMap;
use crate::error::CompileError;
use crate::traits::{Compile as _, Resolve as _};
use runestick::unit::{Assembly, Label};
use runestick::{Component, Context, ImportKey, Inst, Item, Meta, Source, Span, TypeCheck, Unit};
use std::cell::RefCell;
use std::rc::Rc;

use crate::error::CompileResult;
use crate::index::{Index, Indexer};
use crate::items::Items;
use crate::loops::Loops;
use crate::options::Options;
use crate::query::{Build, Query};
use crate::scopes::{Scope, ScopeGuard, Scopes};
use crate::warning::Warnings;

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

/// Helper function to compile the given source.
pub fn compile(
    context: &Context,
    source: &Source,
    unit: &Rc<RefCell<runestick::Unit>>,
    warnings: &mut Warnings,
) -> CompileResult<()> {
    compile_with_options(context, source, &Default::default(), unit, warnings)?;
    Ok(())
}

/// Encode the given object into a collection of asm.
pub fn compile_with_options(
    context: &Context,
    source: &Source,
    options: &Options,
    unit: &Rc<RefCell<runestick::Unit>>,
    warnings: &mut Warnings,
) -> CompileResult<()> {
    let source_id = unit
        .borrow_mut()
        .debug_info_mut()
        .insert_source(source.clone());

    let mut query = Query::new(source, unit.clone());
    let mut indexer = Indexer::new(source_id, source, &mut query, warnings);
    let file = crate::parse_all::<ast::DeclFile>(source.as_str())?;
    indexer.index(&file)?;

    process_imports(&indexer, context, &mut *unit.borrow_mut())?;

    while let Some((item, build)) = query.queue.pop_front() {
        let mut asm = unit.borrow().new_assembly();

        let mut compiler = Compiler {
            source_id,
            context,
            query: &mut query,
            asm: &mut asm,
            items: Items::new(item.as_vec()),
            unit: unit.clone(),
            scopes: Scopes::new(),
            contexts: vec![],
            source,
            loops: Loops::new(),
            options,
            warnings,
        };

        match build {
            Build::Function(f) => {
                let span = f.ast.span();
                let count = f.ast.args.items.len();
                compiler.contexts.push(span);
                compiler.compile((f.ast, false))?;
                unit.borrow_mut()
                    .new_function(source_id, item, count, asm, f.call)?;
            }
            Build::InstanceFunction(f) => {
                let span = f.ast.span();
                let count = f.ast.args.items.len();
                compiler.contexts.push(span);

                let name = f.ast.name.resolve(&source)?;

                let meta = compiler
                    .lookup_meta(&f.impl_item, f.instance_span)?
                    .ok_or_else(|| CompileError::MissingType {
                        span: f.instance_span,
                        item: f.impl_item.clone(),
                    })?;

                let value_type =
                    meta.value_type()
                        .ok_or_else(|| CompileError::UnsupportedInstanceFunction {
                            meta: meta.clone(),
                            span,
                        })?;

                compiler.compile((f.ast, true))?;
                unit.borrow_mut()
                    .new_instance_function(source_id, item, value_type, name, count, asm, f.call)?;
            }
            Build::Closure(c) => {
                let span = c.ast.span();
                let count = c.ast.args.len();
                compiler.contexts.push(span);
                compiler.compile((c.ast, &c.captures[..]))?;
                unit.borrow_mut()
                    .new_function(source_id, item, count, asm, c.call)?;
            }
            Build::AsyncBlock(async_block) => {
                let span = async_block.ast.span();
                let args = async_block.captures.len();
                compiler.contexts.push(span);
                compiler.compile((async_block.ast, &async_block.captures[..]))?;
                unit.borrow_mut()
                    .new_function(source_id, item, args, asm, async_block.call)?;
            }
        }
    }

    Ok(())
}

fn process_imports(
    indexer: &Indexer<'_, '_>,
    context: &Context,
    unit: &mut Unit,
) -> Result<(), CompileError> {
    for (item, decl_use) in &indexer.imports {
        let span = decl_use.span();

        let mut name = Item::empty();
        let first = decl_use.first.resolve(indexer.source)?;
        name.push(first);

        let mut it = decl_use.rest.iter();
        let last = it.next_back();

        for (_, c) in it {
            match c {
                ast::DeclUseComponent::Wildcard(t) => {
                    return Err(CompileError::UnsupportedWildcard { span: t.span() });
                }
                ast::DeclUseComponent::Ident(ident) => {
                    name.push(ident.resolve(indexer.source)?);
                }
            }
        }

        if let Some((_, c)) = last {
            match c {
                ast::DeclUseComponent::Wildcard(..) => {
                    let mut new_names = Vec::new();

                    if !context.contains_prefix(&name) && !unit.contains_prefix(&name) {
                        return Err(CompileError::MissingModule { span, item: name });
                    }

                    let iter = context
                        .iter_components(&name)
                        .chain(unit.iter_components(&name));

                    for c in iter {
                        let mut name = name.clone();
                        name.push(c);
                        new_names.push(name);
                    }

                    for name in new_names {
                        unit.new_import(item.clone(), &name, span)?;
                    }
                }
                ast::DeclUseComponent::Ident(ident) => {
                    name.push(ident.resolve(indexer.source)?);
                    unit.new_import(item.clone(), &name, span)?;
                }
            }
        }
    }

    for (_, entry) in unit.iter_imports() {
        if context.contains_prefix(&entry.item) || unit.contains_prefix(&entry.item) {
            continue;
        }

        if let Some(span) = entry.span {
            return Err(CompileError::MissingModule {
                span,
                item: entry.item.clone(),
            });
        } else {
            return Err(CompileError::MissingPreludeModule {
                item: entry.item.clone(),
            });
        }
    }

    Ok(())
}

pub(crate) struct Compiler<'a, 'source> {
    pub(crate) source_id: usize,
    /// The context we are compiling for.
    context: &'a Context,
    /// Query system to compile required items.
    pub(crate) query: &'a mut Query<'source>,
    /// The assembly we are generating.
    pub(crate) asm: &'a mut Assembly,
    /// Item builder.
    pub(crate) items: Items,
    /// The compilation unit we are compiling for.
    pub(crate) unit: Rc<RefCell<runestick::Unit>>,
    /// Scopes defined in the compiler.
    pub(crate) scopes: Scopes,
    /// Context for which to emit warnings.
    pub(crate) contexts: Vec<Span>,
    /// The source we are compiling for.
    pub(crate) source: &'source Source,
    /// The nesting of loop we are currently in.
    pub(crate) loops: Loops,
    /// Enabled optimizations.
    pub(crate) options: &'a Options,
    /// Compilation warnings.
    pub(crate) warnings: &'a mut Warnings,
}

impl<'a, 'source> Compiler<'a, 'source> {
    /// Access the meta for the given language item.
    pub fn lookup_meta(&mut self, name: &Item, span: Span) -> CompileResult<Option<Meta>> {
        log::trace!("lookup meta: {}", name);

        if let Some(meta) = self.context.lookup_meta(name) {
            log::trace!("found in context: {:?}", meta);
            return Ok(Some(meta));
        }

        let mut base = self.items.item();

        loop {
            let current = base.join(name);
            log::trace!("lookup meta (query): {}", current);

            if let Some(meta) = self.query.query_meta(&current, span)? {
                log::trace!("found in query: {:?}", meta);
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
        meta: &Meta,
        span: Span,
        needs: Needs,
    ) -> CompileResult<()> {
        log::trace!("Meta => {:?} {:?}", meta, needs);

        while let Needs::Value = needs {
            match meta {
                Meta::MetaTuple { tuple, .. } if tuple.args == 0 => {
                    self.asm.push_with_comment(
                        Inst::Call {
                            hash: tuple.hash,
                            args: 0,
                        },
                        span,
                        format!("tuple `{}`", tuple.item),
                    );
                }
                Meta::MetaVariantTuple {
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
                Meta::MetaTuple { tuple, .. } => {
                    self.asm.push_with_comment(
                        Inst::Fn { hash: tuple.hash },
                        span,
                        format!("tuple `{}`", tuple.item),
                    );
                }
                Meta::MetaVariantTuple {
                    enum_item, tuple, ..
                } => {
                    self.asm.push_with_comment(
                        Inst::Fn { hash: tuple.hash },
                        span,
                        format!("tuple variant `{}::{}`", enum_item, tuple.item),
                    );
                }
                Meta::MetaFunction {
                    value_type, item, ..
                } => {
                    let hash = value_type.as_type_hash();
                    self.asm
                        .push_with_comment(Inst::Fn { hash }, span, format!("fn `{}`", item));
                }
                meta => {
                    return Err(CompileError::UnsupportedValue {
                        span,
                        meta: meta.clone(),
                    });
                }
            }

            return Ok(());
        }

        let value_type = meta
            .value_type()
            .ok_or_else(|| CompileError::UnsupportedType {
                span,
                meta: meta.clone(),
            })?;

        let hash = value_type.as_type_hash();
        self.asm.push(Inst::Type { hash }, span);
        Ok(())
    }

    /// Lookup the given local name.
    fn lookup_import_by_name(&self, local: &Component) -> Option<Item> {
        let unit = self.unit.borrow();

        let mut base = self.items.item();

        loop {
            let key = ImportKey::new(base.clone(), local.clone());

            if let Some(entry) = unit.lookup_import(&key) {
                return Some(entry.item.clone());
            }

            if base.pop().is_none() {
                break;
            }
        }

        None
    }

    /// Convert a path to an item.
    pub(crate) fn convert_path_to_item(&self, path: &ast::Path) -> CompileResult<Item> {
        let local = Component::from(path.first.resolve(self.source)?);

        let imported = match self.lookup_import_by_name(&local) {
            Some(path) => path,
            None => Item::of(&[local]),
        };

        let mut rest = Vec::new();

        for (_, part) in &path.rest {
            rest.push(Component::String(part.resolve(self.source)?.to_owned()));
        }

        let it = imported.into_iter().chain(rest.into_iter());
        Ok(Item::of(it))
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

                let mut scope = self.scopes.child(span)?;
                self.compile((&*expr_let.expr, Needs::Value))?;

                let load = |_: &mut Assembly| {};

                if self.compile_pat(&mut scope, &expr_let.pat, false_label, &load)? {
                    self.asm.jump(then_label, span);
                    self.asm.label(false_label)?;
                } else {
                    self.asm.jump(then_label, span);
                };

                Ok(scope)
            }
        }
    }

    /// Encode a vector pattern match.
    pub(crate) fn compile_pat_vec(
        &mut self,
        scope: &mut Scope,
        pat_vec: &ast::PatVec,
        false_label: Label,
        load: &dyn Fn(&mut Assembly),
    ) -> CompileResult<()> {
        let span = pat_vec.span();
        log::trace!("PatVec => {:?}", self.source.source(span));

        // Assign the yet-to-be-verified tuple to an anonymous slot, so we can
        // interact with it multiple times.
        load(&mut self.asm);
        let offset = scope.decl_anon(span);

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
            .pop_and_jump_if_not(scope.local_var_count, false_label, span);

        for (index, (pat, _)) in pat_vec.items.iter().enumerate() {
            let span = pat.span();

            let load = move |asm: &mut Assembly| {
                asm.push(Inst::TupleIndexGetAt { offset, index }, span);
            };

            self.compile_pat(scope, &*pat, false_label, &load)?;
        }

        Ok(())
    }

    /// Encode a vector pattern match.
    pub(crate) fn compile_pat_tuple(
        &mut self,
        scope: &mut Scope,
        pat_tuple: &ast::PatTuple,
        false_label: Label,
        load: &dyn Fn(&mut Assembly),
    ) -> CompileResult<()> {
        let span = pat_tuple.span();
        log::trace!("PatTuple => {:?}", self.source.source(span));

        // Assign the yet-to-be-verified tuple to an anonymous slot, so we can
        // interact with it multiple times.
        load(&mut self.asm);
        let offset = scope.decl_anon(span);

        let type_check = if let Some(path) = &pat_tuple.path {
            let item = self.convert_path_to_item(path)?;

            let (tuple, meta, type_check) =
                if let Some(meta) = self.lookup_meta(&item, path.span())? {
                    match &meta {
                        Meta::MetaTuple {
                            tuple, value_type, ..
                        } => {
                            let type_check = TypeCheck::Type(value_type.as_type_hash());
                            (tuple.clone(), meta, type_check)
                        }
                        Meta::MetaVariantTuple {
                            tuple, value_type, ..
                        } => {
                            let type_check = TypeCheck::Variant(value_type.as_type_hash());
                            (tuple.clone(), meta, type_check)
                        }
                        _ => return Err(CompileError::UnsupportedMetaPattern { meta, span }),
                    }
                } else {
                    return Err(CompileError::UnsupportedPattern { span });
                };

            let count = pat_tuple.items.len();
            let is_open = pat_tuple.open_pattern.is_some();

            if !(tuple.args == count || count < tuple.args && is_open) {
                return Err(CompileError::UnsupportedArgumentCount {
                    span,
                    meta,
                    expected: tuple.args,
                    actual: count,
                });
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
            .pop_and_jump_if_not(scope.local_var_count, false_label, span);

        for (index, (pat, _)) in pat_tuple.items.iter().enumerate() {
            let span = pat.span();

            let load = move |asm: &mut Assembly| {
                asm.push(Inst::TupleIndexGetAt { offset, index }, span);
            };

            self.compile_pat(scope, &*pat, false_label, &load)?;
        }

        Ok(())
    }

    /// Encode an object pattern match.
    pub(crate) fn compile_pat_object(
        &mut self,
        scope: &mut Scope,
        pat_object: &ast::PatObject,
        false_label: Label,
        load: &dyn Fn(&mut Assembly),
    ) -> CompileResult<()> {
        let span = pat_object.span();
        log::trace!("PatObject => {:?}", self.source.source(span));

        // NB: bind the loaded variable (once) to an anonymous var.
        // We reduce the number of copy operations by having specialized
        // operations perform the load from the given offset.
        load(&mut self.asm);
        let offset = scope.decl_anon(span);

        let mut string_slots = Vec::new();

        let mut keys_dup = HashMap::new();
        let mut keys = Vec::new();

        for (item, _) in &pat_object.fields {
            let span = item.span();

            let key = item.key.resolve(self.source)?;
            string_slots.push(self.unit.borrow_mut().new_static_string(&*key)?);
            keys.push(key.to_string());

            if let Some(existing) = keys_dup.insert(key, span) {
                return Err(CompileError::DuplicateObjectKey {
                    span,
                    existing,
                    object: pat_object.span(),
                });
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
                        return Err(CompileError::MissingType { span, item });
                    }
                };

                let (object, type_check) = match &meta {
                    Meta::MetaStruct {
                        object, value_type, ..
                    } => {
                        let type_check = TypeCheck::Type(value_type.as_type_hash());
                        (object, type_check)
                    }
                    Meta::MetaVariantStruct {
                        object, value_type, ..
                    } => {
                        let type_check = TypeCheck::Variant(value_type.as_type_hash());
                        (object, type_check)
                    }
                    _ => {
                        return Err(CompileError::UnsupportedMetaPattern { meta, span });
                    }
                };

                let fields = match &object.fields {
                    Some(fields) => fields,
                    None => {
                        // NB: might want to describe that field composition is unknown because it is an external meta item.
                        return Err(CompileError::UnsupportedMetaPattern { meta, span });
                    }
                };

                for (field, _) in &pat_object.fields {
                    let span = field.key.span();
                    let key = field.key.resolve(self.source)?;

                    if !fields.contains(&*key) {
                        return Err(CompileError::LitObjectNotField {
                            span,
                            field: key.to_string(),
                            item: object.item.clone(),
                        });
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
            .pop_and_jump_if_not(scope.local_var_count, false_label, span);

        for ((item, _), slot) in pat_object.fields.iter().zip(string_slots) {
            let span = item.span();

            let load = move |asm: &mut Assembly| {
                asm.push(Inst::ObjectSlotIndexGetAt { offset, slot }, span);
            };

            if let Some((_, pat)) = &item.binding {
                // load the given vector index and declare it as a local variable.
                self.compile_pat(scope, &*pat, false_label, &load)?;
                continue;
            }

            // NB: only raw identifiers are supported as anonymous bindings
            let ident = match &item.key {
                ast::LitObjectKey::Ident(ident) => ident,
                _ => return Err(CompileError::UnsupportedBinding { span }),
            };

            load(&mut self.asm);
            let name = ident.resolve(self.source)?;
            scope.decl_var(name, span);
        }

        Ok(())
    }

    /// Compile a binding name that matches a known meta type.
    ///
    /// Returns `true` if the binding was used.
    pub(crate) fn compile_pat_meta_binding(
        &mut self,
        scope: &mut Scope,
        span: Span,
        meta: &Meta,
        false_label: Label,
        load: &dyn Fn(&mut Assembly),
    ) -> CompileResult<bool> {
        let (tuple, type_check) = match meta {
            Meta::MetaTuple {
                tuple, value_type, ..
            } if tuple.args == 0 => (tuple, TypeCheck::Type(value_type.as_type_hash())),
            Meta::MetaVariantTuple {
                tuple, value_type, ..
            } if tuple.args == 0 => (tuple, TypeCheck::Variant(value_type.as_type_hash())),
            _ => return Ok(false),
        };

        let type_check = match self.context.type_check_for(&tuple.item) {
            Some(type_check) => type_check,
            None => type_check,
        };

        load(&mut self.asm);
        self.asm.push(
            Inst::MatchSequence {
                type_check,
                len: tuple.args,
                exact: true,
            },
            span,
        );
        self.asm
            .pop_and_jump_if_not(scope.local_var_count, false_label, span);
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
        scope: &mut Scope,
        pat: &ast::Pat,
        false_label: Label,
        load: &dyn Fn(&mut Assembly),
    ) -> CompileResult<bool> {
        let span = pat.span();
        log::trace!("Pat => {:?}", self.source.source(span));

        match pat {
            ast::Pat::PatPath(path) => {
                let span = path.span();

                let item = self.convert_path_to_item(&path.path)?;

                if let Some(meta) = self.lookup_meta(&item, span)? {
                    if self.compile_pat_meta_binding(scope, span, &meta, false_label, load)? {
                        return Ok(true);
                    }
                }

                let ident = match item.as_local() {
                    Some(ident) => ident,
                    None => {
                        return Err(CompileError::UnsupportedBinding { span });
                    }
                };

                load(&mut self.asm);
                scope.decl_var(&ident, span);
                return Ok(false);
            }
            ast::Pat::PatIgnore(..) => {
                return Ok(false);
            }
            ast::Pat::PatUnit(unit) => {
                load(&mut self.asm);
                self.asm.push(Inst::IsUnit, unit.span());
            }
            ast::Pat::PatByte(lit_byte) => {
                let byte = lit_byte.resolve(self.source)?;
                load(&mut self.asm);
                self.asm.push(Inst::EqByte { byte }, lit_byte.span());
            }
            ast::Pat::PatChar(lit_char) => {
                let character = lit_char.resolve(self.source)?;
                load(&mut self.asm);
                self.asm
                    .push(Inst::EqCharacter { character }, lit_char.span());
            }
            ast::Pat::PatNumber(number_literal) => {
                let span = number_literal.span();
                let number = number_literal.resolve(self.source)?;

                let integer = match number {
                    ast::Number::Integer(integer) => integer,
                    ast::Number::Float(..) => {
                        return Err(CompileError::MatchFloatInPattern { span });
                    }
                };

                load(&mut self.asm);
                self.asm.push(Inst::EqInteger { integer }, span);
            }
            ast::Pat::PatString(pat_string) => {
                let span = pat_string.span();
                let string = pat_string.resolve(self.source)?;
                let slot = self.unit.borrow_mut().new_static_string(&*string)?;
                load(&mut self.asm);
                self.asm.push(Inst::EqStaticString { slot }, span);
            }
            ast::Pat::PatVec(pat_vec) => {
                self.compile_pat_vec(scope, pat_vec, false_label, &load)?;
                return Ok(true);
            }
            ast::Pat::PatTuple(pat_tuple) => {
                self.compile_pat_tuple(scope, pat_tuple, false_label, &load)?;
                return Ok(true);
            }
            ast::Pat::PatObject(object) => {
                self.compile_pat_object(scope, object, false_label, &load)?;
                return Ok(true);
            }
        }

        self.asm
            .pop_and_jump_if_not(scope.local_var_count, false_label, span);
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
