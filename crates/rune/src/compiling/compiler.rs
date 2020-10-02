use crate::ast;
use crate::collections::HashMap;
use crate::compiling::{Assembly, Compile as _, CompileVisitor, Loops, Scope, ScopeGuard, Scopes};
use crate::ir::{IrBudget, IrCompiler, IrInterpreter};
use crate::query::{Named, Query, QueryConstFn, QueryItem, Used};
use crate::CompileResult;
use crate::{
    CompileError, CompileErrorKind, Options, Resolve as _, Spanned, Storage, UnitBuilder, Warnings,
};
use runestick::{
    CompileMeta, CompileMetaKind, ConstValue, Context, Inst, InstValue, Item, Label, Source, Span,
    TypeCheck,
};
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

pub(crate) struct Compiler<'a> {
    /// The source id of the source.
    pub(crate) source_id: usize,
    /// The source we are compiling for.
    pub(crate) source: Arc<Source>,
    /// The current macro context.
    pub(crate) storage: &'a Storage,
    /// The context we are compiling for.
    pub(crate) context: &'a Context,
    /// Query system to compile required items.
    pub(crate) query: &'a mut Query,
    /// The assembly we are generating.
    pub(crate) asm: &'a mut Assembly,
    /// The compilation unit we are compiling for.
    pub(crate) unit: UnitBuilder,
    /// Scopes defined in the compiler.
    pub(crate) scopes: Scopes,
    /// Context for which to emit warnings.
    pub(crate) contexts: Vec<Span>,
    /// The nesting of loop we are currently in.
    pub(crate) loops: Loops,
    /// Enabled optimizations.
    pub(crate) options: &'a Options,
    /// Compilation warnings.
    pub(crate) warnings: &'a mut Warnings,
    /// Compiler visitor.
    pub(crate) visitor: &'a mut dyn CompileVisitor,
}

impl<'a> Compiler<'a> {
    /// Access the meta for the given language item.
    pub fn lookup_meta(
        &mut self,
        spanned: Span,
        named: &Named,
    ) -> CompileResult<Option<CompileMeta>> {
        log::trace!("lookup meta: {:?}", named.item);

        if let Some(meta) = self
            .query
            .query_meta(spanned, &named.item, Default::default())?
        {
            log::trace!("found in query: {:?}", meta);
            self.visitor.visit_meta(self.source_id, &meta, spanned);
            return Ok(Some(meta));
        }

        if let Some(meta) = self.context.lookup_meta(&named.item) {
            log::trace!("found in context: {:?}", meta);
            self.visitor.visit_meta(self.source_id, &meta, spanned);
            return Ok(Some(meta));
        }

        Ok(None)
    }

    /// Access the meta for the given language item.
    pub fn lookup_exact_meta(
        &mut self,
        spanned: Span,
        name: &Item,
    ) -> CompileResult<Option<CompileMeta>> {
        log::trace!("lookup meta: {}", name);

        if let Some(meta) = self.context.lookup_meta(name) {
            log::trace!("found in context: {:?}", meta);
            self.visitor.visit_meta(self.source_id, &meta, spanned);
            return Ok(Some(meta));
        }

        if let Some(meta) = self.query.query_meta(spanned, name, Default::default())? {
            log::trace!("found in query: {:?}", meta);
            self.visitor.visit_meta(self.source_id, &meta, spanned);
            return Ok(Some(meta));
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
                CompileMetaKind::UnitStruct { empty, .. } => {
                    self.asm.push_with_comment(
                        Inst::Call {
                            hash: empty.hash,
                            args: 0,
                        },
                        span,
                        meta.to_string(),
                    );
                }
                CompileMetaKind::TupleStruct { tuple, .. } if tuple.args == 0 => {
                    self.asm.push_with_comment(
                        Inst::Call {
                            hash: tuple.hash,
                            args: 0,
                        },
                        span,
                        meta.to_string(),
                    );
                }
                CompileMetaKind::UnitVariant { empty, .. } => {
                    self.asm.push_with_comment(
                        Inst::Call {
                            hash: empty.hash,
                            args: 0,
                        },
                        span,
                        meta.to_string(),
                    );
                }
                CompileMetaKind::TupleVariant { tuple, .. } if tuple.args == 0 => {
                    self.asm.push_with_comment(
                        Inst::Call {
                            hash: tuple.hash,
                            args: 0,
                        },
                        span,
                        meta.to_string(),
                    );
                }
                CompileMetaKind::TupleStruct { tuple, .. } => {
                    self.asm.push_with_comment(
                        Inst::LoadFn { hash: tuple.hash },
                        span,
                        meta.to_string(),
                    );
                }
                CompileMetaKind::TupleVariant { tuple, .. } => {
                    self.asm.push_with_comment(
                        Inst::LoadFn { hash: tuple.hash },
                        span,
                        meta.to_string(),
                    );
                }
                CompileMetaKind::Function { type_of } => {
                    let hash = **type_of;
                    self.asm
                        .push_with_comment(Inst::LoadFn { hash }, span, meta.to_string());
                }
                CompileMetaKind::Const { const_value, .. } => {
                    self.compile((const_value, span))?;
                }
                _ => {
                    return Err(CompileError::expected_meta(
                        span,
                        meta.clone(),
                        "something that can be used as a value",
                    ));
                }
            }

            return Ok(());
        }

        let type_of = meta.type_of().ok_or_else(|| {
            CompileError::expected_meta(span, meta.clone(), "something that has a type")
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
    pub(crate) fn convert_path_to_named(&mut self, path: &ast::Path) -> CompileResult<Named> {
        let named = self
            .query
            .convert_path(path, &self.storage, &*self.source)?;

        Ok(named)
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

        let (is_open, count) = pat_items_count(&pat_vec.items)?;

        self.asm.push(
            Inst::MatchSequence {
                type_check: TypeCheck::Vec,
                len: count,
                exact: !is_open,
            },
            span,
        );

        self.asm
            .pop_and_jump_if_not(self.scopes.local_var_count(span)?, false_label, span);

        for (index, (pat, _)) in pat_vec.items.iter().take(count).enumerate() {
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
            let named = self.convert_path_to_named(path)?;

            let meta = match self.lookup_meta(path.span(), &named)? {
                Some(meta) => meta,
                None => {
                    return Err(CompileError::new(
                        span,
                        CompileErrorKind::UnsupportedPattern,
                    ));
                }
            };

            let (args, type_check) = match &meta.kind {
                CompileMetaKind::UnitStruct { type_of, .. } => {
                    let type_check = TypeCheck::Type(**type_of);
                    (0, type_check)
                }
                CompileMetaKind::TupleStruct { tuple, type_of, .. } => {
                    let type_check = TypeCheck::Type(**type_of);
                    (tuple.args, type_check)
                }
                CompileMetaKind::UnitVariant { type_of, .. } => {
                    let type_check = TypeCheck::Variant(**type_of);
                    (0, type_check)
                }
                CompileMetaKind::TupleVariant { tuple, type_of, .. } => {
                    let type_check = TypeCheck::Variant(**type_of);
                    (tuple.args, type_check)
                }
                _ => {
                    return Err(CompileError::expected_meta(
                        span,
                        meta,
                        "type that can be used in a tuple pattern",
                    ));
                }
            };

            let (has_rest, count) = pat_items_count(&pat_tuple.items)?;

            if !(args == count || count < args && has_rest) {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::UnsupportedArgumentCount {
                        meta,
                        expected: args,
                        actual: count,
                    },
                ));
            }

            match self.context.type_check_for(&meta.item) {
                Some(type_check) => type_check,
                None => type_check,
            }
        } else {
            TypeCheck::Tuple
        };

        let (is_open, count) = pat_items_count(&pat_tuple.items)?;

        self.asm.push(Inst::Copy { offset }, span);
        self.asm.push(
            Inst::MatchSequence {
                type_check,
                len: count,
                exact: !is_open,
            },
            span,
        );
        self.asm
            .pop_and_jump_if_not(self.scopes.local_var_count(span)?, false_label, span);

        for (index, (pat, _)) in pat_tuple.items.iter().take(count).enumerate() {
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

        let mut bindings = Vec::new();
        let (has_rest, count) = pat_items_count(&pat_object.items)?;

        for (pat, _) in pat_object.items.iter().take(count) {
            let span = pat.span();

            let key = match pat {
                ast::Pat::PatBinding(binding) => {
                    let key = binding.key.resolve(&self.storage, &*self.source)?;
                    bindings.push(Binding::Binding(
                        binding.span(),
                        key.as_ref().into(),
                        &*binding.pat,
                    ));
                    key
                }
                ast::Pat::PatPath(path) => {
                    let ident = match path.path.try_as_ident() {
                        Some(ident) => ident,
                        None => {
                            return Err(CompileError::new(
                                span,
                                CompileErrorKind::UnsupportedPattern,
                            ));
                        }
                    };

                    let key = ident.resolve(&self.storage, &*self.source)?;

                    bindings.push(Binding::Ident(path.span(), key.as_ref().into()));
                    key
                }
                _ => {
                    return Err(CompileError::new(
                        span,
                        CompileErrorKind::UnsupportedPattern,
                    ));
                }
            };

            string_slots.push(self.unit.new_static_string(span, &*key)?);

            if let Some(existing) = keys_dup.insert(key.to_string(), span) {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::DuplicateObjectKey {
                        existing,
                        object: pat_object.span(),
                    },
                ));
            }

            keys.push(key.to_string());
        }

        let keys = self.unit.new_static_object_keys(span, &keys[..])?;

        let type_check = match &pat_object.ident {
            ast::LitObjectIdent::Named(path) => {
                let span = path.span();

                let named = self.convert_path_to_named(path)?;

                let meta = match self.lookup_meta(span, &named)? {
                    Some(meta) => meta,
                    None => {
                        return Err(CompileError::new(
                            span,
                            CompileErrorKind::MissingType {
                                item: named.item.clone(),
                            },
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
                        return Err(CompileError::expected_meta(
                            span,
                            meta,
                            "type that can be used in an object pattern",
                        ));
                    }
                };

                let fields = match &object.fields {
                    Some(fields) => fields,
                    None => {
                        // NB: might want to describe that field composition is unknown because it is an external meta item.
                        return Err(CompileError::expected_meta(
                            span,
                            meta,
                            "type that can be used in an object pattern",
                        ));
                    }
                };

                for binding in &bindings {
                    if !fields.contains(binding.key()) {
                        return Err(CompileError::new(
                            span,
                            CompileErrorKind::LitObjectNotField {
                                field: binding.key().to_string(),
                                item: meta.item.clone(),
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
                exact: !has_rest,
            },
            span,
        );

        self.asm
            .pop_and_jump_if_not(self.scopes.local_var_count(span)?, false_label, span);

        for (binding, slot) in bindings.iter().zip(string_slots) {
            let span = binding.span();

            match binding {
                Binding::Binding(_, _, pat) => {
                    let load = move |this: &mut Self, needs: Needs| {
                        if needs.value() {
                            this.asm.push(Inst::ObjectIndexGetAt { offset, slot }, span);
                        }

                        Ok(())
                    };

                    self.compile_pat(&*pat, false_label, &load)?;
                }
                Binding::Ident(_, key) => {
                    self.asm.push(Inst::ObjectIndexGetAt { offset, slot }, span);
                    self.scopes.decl_var(key, span)?;
                }
            }
        }

        return Ok(());

        enum Binding<'a> {
            Binding(Span, Box<str>, &'a ast::Pat),
            Ident(Span, Box<str>),
        }

        impl Binding<'_> {
            fn span(&self) -> Span {
                match self {
                    Self::Binding(span, _, _) => *span,
                    Self::Ident(span, _) => *span,
                }
            }

            fn key(&self) -> &str {
                match self {
                    Self::Binding(_, key, _) => key.as_ref(),
                    Self::Ident(_, key) => key.as_ref(),
                }
            }
        }
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
        let type_check = match &meta.kind {
            CompileMetaKind::UnitStruct { type_of, .. } => TypeCheck::Type(**type_of),
            CompileMetaKind::TupleStruct { tuple, type_of, .. } if tuple.args == 0 => {
                TypeCheck::Type(**type_of)
            }
            CompileMetaKind::UnitVariant { type_of, .. } => TypeCheck::Variant(**type_of),
            CompileMetaKind::TupleVariant { tuple, type_of, .. } if tuple.args == 0 => {
                TypeCheck::Variant(**type_of)
            }
            _ => return Ok(false),
        };

        let type_check = match self.context.type_check_for(&meta.item) {
            Some(type_check) => type_check,
            None => type_check,
        };

        load(self, Needs::Value)?;
        self.asm.push(
            Inst::MatchSequence {
                type_check,
                len: 0,
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

                let named = self.convert_path_to_named(&path.path)?;

                if let Some(meta) = self.lookup_meta(span, &named)? {
                    if self.compile_pat_meta_binding(span, &meta, false_label, load)? {
                        return Ok(true);
                    }
                }

                if let Some(ident) = named.as_local() {
                    load(self, Needs::Value)?;
                    self.scopes.decl_var(ident, span)?;
                    return Ok(false);
                }

                Err(CompileError::new(
                    span,
                    CompileErrorKind::UnsupportedBinding,
                ))
            }
            ast::Pat::PatIgnore(..) => {
                // ignore binding, but might still have side effects, so must
                // call the load generator.
                load(self, Needs::None)?;
                Ok(false)
            }
            ast::Pat::PatLit(pat_lit) => Ok(self.compile_pat_lit(pat_lit, false_label, load)?),
            ast::Pat::PatVec(pat_vec) => {
                self.compile_pat_vec(pat_vec, false_label, &load)?;
                Ok(true)
            }
            ast::Pat::PatTuple(pat_tuple) => {
                self.compile_pat_tuple(pat_tuple, false_label, &load)?;
                Ok(true)
            }
            ast::Pat::PatObject(object) => {
                self.compile_pat_object(object, false_label, &load)?;
                Ok(true)
            }
            pat => Err(CompileError::new(pat, CompileErrorKind::UnsupportedPattern)),
        }
    }

    pub(crate) fn compile_pat_lit(
        &mut self,
        pat_lit: &ast::PatLit,
        false_label: Label,
        load: &dyn Fn(&mut Self, Needs) -> CompileResult<()>,
    ) -> CompileResult<bool> {
        loop {
            match &*pat_lit.expr {
                ast::Expr::ExprUnary(expr_unary) => match &*expr_unary.expr {
                    ast::Expr::ExprLit(ast::ExprLit {
                        lit: ast::Lit::Number(lit_number),
                        ..
                    }) => {
                        let span = lit_number.span();
                        let integer = lit_number
                            .resolve(&self.storage, &*self.source)?
                            .as_i64(pat_lit.span(), true)?;
                        load(self, Needs::Value)?;
                        self.asm.push(Inst::EqInteger { integer }, span);
                        break;
                    }
                    _ => (),
                },
                ast::Expr::ExprLit(expr_lit) => match &expr_lit.lit {
                    ast::Lit::Unit(unit) => {
                        load(self, Needs::Value)?;
                        self.asm.push(Inst::IsUnit, unit.span());
                        break;
                    }
                    ast::Lit::Byte(lit_byte) => {
                        let byte = lit_byte.resolve(&self.storage, &*self.source)?;
                        load(self, Needs::Value)?;
                        self.asm.push(Inst::EqByte { byte }, lit_byte.span());
                        break;
                    }
                    ast::Lit::Char(lit_char) => {
                        let character = lit_char.resolve(&self.storage, &*self.source)?;
                        load(self, Needs::Value)?;
                        self.asm
                            .push(Inst::EqCharacter { character }, lit_char.span());
                        break;
                    }
                    ast::Lit::Str(pat_string) => {
                        let span = pat_string.span();
                        let string = pat_string.resolve(&self.storage, &*self.source)?;
                        let slot = self.unit.new_static_string(span, &*string)?;
                        load(self, Needs::Value)?;
                        self.asm.push(Inst::EqStaticString { slot }, span);
                        break;
                    }
                    ast::Lit::Number(lit_number) => {
                        let span = lit_number.span();
                        let integer = lit_number
                            .resolve(&self.storage, &*self.source)?
                            .as_i64(pat_lit.span(), false)?;
                        load(self, Needs::Value)?;
                        self.asm.push(Inst::EqInteger { integer }, span);
                        break;
                    }
                    ast::Lit::Bool(_) => {}
                    ast::Lit::ByteStr(_) => {}
                    ast::Lit::Object(_) => {}
                    ast::Lit::Template(_) => {}
                    ast::Lit::Tuple(_) => {}
                    ast::Lit::Vec(_) => {}
                },
                _ => (),
            }

            return Err(CompileError::new(
                pat_lit,
                CompileErrorKind::UnsupportedPattern,
            ));
        }

        let span = pat_lit.span();
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

    /// Calling a constant function by id and return the resuling value.
    pub(crate) fn call_const_fn<S>(
        &mut self,
        spanned: S,
        meta: &CompileMeta,
        from: &QueryItem,
        query_const_fn: &QueryConstFn,
        args: &[(ast::Expr, Option<ast::Comma>)],
    ) -> Result<ConstValue, CompileError>
    where
        S: Copy + Spanned,
    {
        use crate::ir::IrCompile;

        if query_const_fn.ir_fn.args.len() != args.len() {
            return Err(CompileError::new(
                spanned,
                CompileErrorKind::UnsupportedArgumentCount {
                    meta: meta.clone(),
                    expected: query_const_fn.ir_fn.args.len(),
                    actual: args.len(),
                },
            ));
        }

        let mut compiler = IrCompiler {
            query: self.query,
            source: &*self.source,
            storage: self.storage,
        };

        let mut compiled = Vec::new();

        // TODO: precompile these and fetch using opaque id?
        for ((a, _), name) in args.iter().zip(&query_const_fn.ir_fn.args) {
            compiled.push((compiler.compile(a)?, name));
        }

        let mut interpreter = IrInterpreter {
            budget: IrBudget::new(1_000_000),
            scopes: Default::default(),
            mod_item: from.mod_item.clone(),
            item: from.item.clone(),
            query: self.query,
        };

        for (ir, name) in compiled {
            let value = interpreter.eval_value(&ir, Used::Used)?;
            interpreter.scopes.decl(name, value, spanned)?;
        }

        interpreter.mod_item = query_const_fn.item.mod_item.clone();
        interpreter.item = query_const_fn.item.item.clone();
        let value = interpreter.eval_value(&query_const_fn.ir_fn.ir, Used::Used)?;
        Ok(value.into_const(spanned)?)
    }
}

/// Test if the given pattern is open or not.
fn pat_items_count<'a, I: 'a, U: 'a>(items: I) -> Result<(bool, usize), CompileError>
where
    I: IntoIterator<Item = &'a (ast::Pat, U)>,
    I::IntoIter: DoubleEndedIterator,
{
    let mut it = items.into_iter();

    let (is_open, mut count) = match it.next_back() {
        Some((pat, _)) => {
            if matches!(pat, ast::Pat::PatRest(..)) {
                (true, 0)
            } else {
                (false, 1)
            }
        }
        None => return Ok((false, 0)),
    };

    for (pat, _) in it {
        if let ast::Pat::PatRest(rest) = pat {
            return Err(CompileError::new(
                rest,
                CompileErrorKind::UnsupportedPattern,
            ));
        }

        count += 1;
    }

    Ok((is_open, count))
}
