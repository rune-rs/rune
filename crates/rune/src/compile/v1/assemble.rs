use core::mem::{replace, take};

use crate as rune;
use crate::alloc::prelude::*;
use crate::alloc::{try_format, Vec};
use crate::ast::{self, Span, Spanned};
use crate::compile::ir;
use crate::compile::v1::{Layer, Loop, Loops, ScopeGuard, Scopes, Var};
use crate::compile::{self, Assembly, ErrorKind, ItemId, ModId, Options, WithSpan};
use crate::hir;
use crate::query::{ConstFn, Query, Used};
use crate::runtime::{
    ConstValue, Inst, InstAddress, InstAssignOp, InstOp, InstRange, InstTarget, InstValue,
    InstVariant, Label, PanicReason, Protocol, TypeCheck,
};
use crate::{Hash, SourceId};

use rune_macros::instrument;

/// A needs hint for an expression.
/// This is used to contextually determine what an expression is expected to
/// produce.
#[derive(Debug, TryClone, Clone, Copy)]
#[try_clone(copy)]
pub(crate) enum Needs {
    Value,
    None,
}

impl Needs {
    /// Test if any sort of value is needed.
    #[inline(always)]
    pub(crate) fn value(self) -> bool {
        matches!(self, Self::Value)
    }
}

/// Assemble context.
pub(crate) struct Ctxt<'a, 'hir, 'arena> {
    /// The source id of the source.
    pub(crate) source_id: SourceId,
    /// Query system to compile required items.
    pub(crate) q: Query<'a, 'arena>,
    /// The assembly we are generating.
    pub(crate) asm: &'a mut Assembly,
    /// Scopes defined in the compiler.
    pub(crate) scopes: Scopes<'hir>,
    /// Context for which to emit warnings.
    pub(crate) contexts: Vec<Span>,
    /// The nesting of loop we are currently in.
    pub(crate) loops: Loops<'hir>,
    /// Enabled optimizations.
    pub(crate) options: &'a Options,
}

impl<'a, 'hir, 'arena> Ctxt<'a, 'hir, 'arena> {
    /// Pop locals by simply popping them.
    pub(crate) fn locals_pop(
        &mut self,
        total_var_count: usize,
        span: &dyn Spanned,
    ) -> compile::Result<()> {
        match total_var_count {
            0 => (),
            1 => {
                self.asm.push(Inst::Pop, span)?;
            }
            count => {
                self.asm.push(Inst::PopN { count }, span)?;
            }
        }

        Ok(())
    }

    /// Clean up local variables by preserving the value that is on top and
    /// popping the rest.
    ///
    /// The clean operation will preserve the value that is on top of the stack,
    /// and pop the values under it.
    pub(crate) fn locals_clean(
        &mut self,
        total_var_count: usize,
        span: &dyn Spanned,
    ) -> compile::Result<()> {
        match total_var_count {
            0 => (),
            count => {
                self.asm.push(Inst::Clean { count }, span)?;
            }
        }

        Ok(())
    }

    /// Clean the last scope.
    pub(crate) fn clean_last_scope(
        &mut self,
        span: &dyn Spanned,
        expected: ScopeGuard,
        needs: Needs,
    ) -> compile::Result<()> {
        let scope = self.scopes.pop(expected, span)?;

        if needs.value() {
            self.locals_clean(scope.local, span)?;
        } else {
            self.locals_pop(scope.local, span)?;
        }

        Ok(())
    }

    /// Get the latest relevant warning context.
    pub(crate) fn context(&self) -> Option<Span> {
        self.contexts.last().copied()
    }

    /// Calling a constant function by id and return the resuling value.
    pub(crate) fn call_const_fn(
        &mut self,
        span: &dyn Spanned,
        from_module: ModId,
        from_item: ItemId,
        query_const_fn: &ConstFn,
        args: &[hir::Expr<'_>],
    ) -> compile::Result<ConstValue> {
        if query_const_fn.ir_fn.args.len() != args.len() {
            return Err(compile::Error::new(
                span,
                ErrorKind::UnsupportedArgumentCount {
                    expected: query_const_fn.ir_fn.args.len(),
                    actual: args.len(),
                },
            ));
        }

        let mut compiler = ir::Ctxt {
            source_id: self.source_id,
            q: self.q.borrow(),
        };

        let mut compiled = Vec::new();

        // TODO: precompile these and fetch using opaque id?
        for (hir, name) in args.iter().zip(&query_const_fn.ir_fn.args) {
            compiled.try_push((ir::compiler::expr(hir, &mut compiler)?, name))?;
        }

        let mut interpreter = ir::Interpreter {
            budget: ir::Budget::new(1_000_000),
            scopes: ir::Scopes::new()?,
            module: from_module,
            item: from_item,
            q: self.q.borrow(),
        };

        for (ir, name) in compiled {
            let value = interpreter.eval_value(&ir, Used::Used)?;
            interpreter.scopes.decl(name, value).with_span(span)?;
        }

        interpreter.module = query_const_fn.item_meta.module;
        interpreter.item = query_const_fn.item_meta.item;
        let value = interpreter.eval_value(&query_const_fn.ir_fn.ir, Used::Used)?;
        value.into_const(span)
    }
}

#[derive(Debug)]
#[must_use = "must be consumed to make sure the value is realized"]
struct Asm<'hir> {
    span: Span,
    kind: AsmKind<'hir>,
}

impl<'hir> Asm<'hir> {
    /// Construct an assembly result that leaves the value on the top of the
    /// stack.
    fn top(span: &dyn Spanned) -> Self {
        Self {
            span: span.span(),
            kind: AsmKind::Top,
        }
    }

    fn var(span: &dyn Spanned, var: Var<'hir>) -> Self {
        Self {
            span: span.span(),
            kind: AsmKind::Var(var),
        }
    }
}

#[derive(Debug)]
pub(crate) enum AsmKind<'hir> {
    // Result is pushed onto the top of the stack.
    Top,
    // Result belongs to the the given stack offset.
    Var(Var<'hir>),
}

impl<'hir> Asm<'hir> {
    /// Assemble into an instruction.
    fn apply(self, cx: &mut Ctxt) -> compile::Result<()> {
        if let AsmKind::Var(var) = self.kind {
            var.copy(cx, &self.span, &format_args!("var `{}`", var))?;
        }

        Ok(())
    }

    /// Assemble into an instruction declaring an anonymous variable if appropriate.
    fn apply_targeted(self, cx: &mut Ctxt) -> compile::Result<InstAddress> {
        let address = match self.kind {
            AsmKind::Top => {
                cx.scopes.alloc(&self.span)?;
                InstAddress::Top
            }
            AsmKind::Var(var) => InstAddress::Offset(var.offset),
        };

        Ok(address)
    }
}

/// Assemble a function from an [hir::ItemFn<'_>].
#[instrument(span = hir)]
pub(crate) fn fn_from_item_fn<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::ItemFn<'hir>,
    instance_fn: bool,
) -> compile::Result<()> {
    let mut patterns = Vec::new();
    let mut first = true;

    for arg in hir.args {
        match arg {
            hir::FnArg::SelfValue(span) => {
                if !instance_fn || !first {
                    return Err(compile::Error::new(*span, ErrorKind::UnsupportedSelf));
                }

                cx.scopes.define(hir::Name::SelfValue, span)?;
            }
            hir::FnArg::Pat(pat) => {
                let offset = cx.scopes.alloc(pat)?;
                patterns.try_push((pat, offset))?;
            }
        }

        first = false;
    }

    for (pat, offset) in patterns {
        pat_with_offset(cx, pat, offset)?;
    }

    if hir.body.statements.is_empty() {
        let total_var_count = cx.scopes.total(hir)?;
        cx.locals_pop(total_var_count, hir)?;
        cx.asm.push(Inst::ReturnUnit, hir)?;
        return Ok(());
    }

    if !hir.body.produces_nothing() {
        return_(cx, hir, &hir.body, block)?;
    } else {
        block(cx, &hir.body, Needs::None)?.apply(cx)?;

        let total_var_count = cx.scopes.total(hir)?;
        cx.locals_pop(total_var_count, hir)?;
        cx.asm.push(Inst::ReturnUnit, hir)?;
    }

    cx.scopes.pop_last(hir)?;
    Ok(())
}

/// Assemble an async block.
#[instrument(span = hir.block.span)]
pub(crate) fn async_block_secondary<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::AsyncBlock<'hir>,
) -> compile::Result<()> {
    for name in hir.captures.iter().copied() {
        cx.scopes.define(name, &hir.block)?;
    }

    return_(cx, &hir.block, &hir.block, block)?;
    cx.scopes.pop_last(&hir.block)?;
    Ok(())
}

/// Assemble the body of a closure function.
#[instrument(span = span)]
pub(crate) fn expr_closure_secondary<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::ExprClosure<'hir>,
    span: &'hir dyn Spanned,
) -> compile::Result<()> {
    let mut patterns = Vec::new();

    for arg in hir.args {
        match arg {
            hir::FnArg::SelfValue(..) => {
                return Err(compile::Error::new(arg, ErrorKind::UnsupportedSelf))
            }
            hir::FnArg::Pat(pat) => {
                let offset = cx.scopes.alloc(pat)?;
                patterns.try_push((pat, offset))?;
            }
        }
    }

    if !hir.captures.is_empty() {
        cx.asm.push(Inst::PushTuple, span)?;

        for capture in hir.captures.iter().copied() {
            cx.scopes.define(capture, span)?;
        }
    }

    for (pat, offset) in patterns {
        pat_with_offset(cx, pat, offset)?;
    }

    return_(cx, span, &hir.body, expr)?;
    cx.scopes.pop_last(span)?;
    Ok(())
}

/// Assemble a return statement from the given Assemble.
fn return_<'hir, T>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    span: &dyn Spanned,
    hir: T,
    asm: impl FnOnce(&mut Ctxt<'_, 'hir, '_>, T, Needs) -> compile::Result<Asm<'hir>>,
) -> compile::Result<()> {
    let clean = cx.scopes.total(span)?;

    let address = asm(cx, hir, Needs::Value)?.apply_targeted(cx)?;
    cx.asm.push(Inst::Return { address, clean }, span)?;

    // Top address produces an anonymous variable, which is consumed by the
    // return statement.
    if let InstAddress::Top = address {
        cx.scopes.free(span, 1)?;
    }

    Ok(())
}

/// Compile a pattern based on the given offset.
#[instrument(span = hir)]
fn pat_with_offset<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::Pat<'hir>,
    offset: usize,
) -> compile::Result<()> {
    let load = |cx: &mut Ctxt<'_, 'hir, '_>, needs: Needs| {
        if needs.value() {
            cx.asm.push(Inst::Copy { offset }, hir)?;
        }

        Ok(())
    };

    let false_label = cx.asm.new_label("let_panic");

    if pat(cx, hir, &false_label, &load)? {
        cx.q.diagnostics
            .let_pattern_might_panic(cx.source_id, hir, cx.context())?;

        let ok_label = cx.asm.new_label("let_ok");
        cx.asm.jump(&ok_label, hir)?;
        cx.asm.label(&false_label)?;
        cx.asm.push(
            Inst::Panic {
                reason: PanicReason::UnmatchedPattern,
            },
            hir,
        )?;

        cx.asm.label(&ok_label)?;
    }

    Ok(())
}

/// Encode a pattern.
///
/// Patterns will clean up their own locals and execute a jump to `false_label`
/// in case the pattern does not match.
///
/// Returns a boolean indicating if the label was used.
#[instrument(span = hir)]
fn pat<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::Pat<'hir>,
    false_label: &Label,
    load: &dyn Fn(&mut Ctxt<'_, 'hir, '_>, Needs) -> compile::Result<()>,
) -> compile::Result<bool> {
    let span = hir;

    match hir.kind {
        hir::PatKind::Ignore => {
            // ignore binding, but might still have side effects, so must
            // call the load generator.
            load(cx, Needs::None)?;
            Ok(false)
        }
        hir::PatKind::Path(kind) => match *kind {
            hir::PatPathKind::Kind(kind) => {
                load(cx, Needs::Value)?;
                cx.asm.push(pat_sequence_kind_to_inst(*kind), hir)?;
                cx.asm
                    .pop_and_jump_if_not(cx.scopes.local(hir)?, false_label, hir)?;
                Ok(true)
            }
            hir::PatPathKind::Ident(name) => {
                load(cx, Needs::Value)?;
                cx.scopes.define(hir::Name::Str(name), hir)?;
                Ok(false)
            }
        },
        hir::PatKind::Lit(hir) => Ok(pat_lit(cx, hir, false_label, load)?),
        hir::PatKind::Sequence(hir) => {
            pat_sequence(cx, hir, span, false_label, &load)?;
            Ok(true)
        }
        hir::PatKind::Object(hir) => {
            pat_object(cx, hir, span, false_label, &load)?;
            Ok(true)
        }
    }
}

/// Assemble a pattern literal.
#[instrument(span = hir)]
fn pat_lit<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::Expr<'_>,
    false_label: &Label,
    load: &dyn Fn(&mut Ctxt<'_, 'hir, '_>, Needs) -> compile::Result<()>,
) -> compile::Result<bool> {
    let Some(inst) = pat_lit_inst(cx, hir)? else {
        return Err(compile::Error::new(hir, ErrorKind::UnsupportedPatternExpr));
    };

    load(cx, Needs::Value)?;
    cx.asm.push(inst, hir)?;
    cx.asm
        .pop_and_jump_if_not(cx.scopes.local(hir)?, false_label, hir)?;
    Ok(true)
}

#[instrument(span = hir)]
fn pat_lit_inst<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::Expr<'_>,
) -> compile::Result<Option<Inst>> {
    let hir::ExprKind::Lit(lit) = hir.kind else {
        return Ok(None);
    };

    let inst = match lit {
        hir::Lit::Byte(byte) => Inst::EqByte { byte },
        hir::Lit::Char(char) => Inst::EqChar { char },
        hir::Lit::Str(string) => Inst::EqString {
            slot: cx.q.unit.new_static_string(hir, string)?,
        },
        hir::Lit::ByteStr(bytes) => Inst::EqBytes {
            slot: cx.q.unit.new_static_bytes(hir, bytes)?,
        },
        hir::Lit::Integer(integer) => Inst::EqInteger { integer },
        hir::Lit::Bool(boolean) => Inst::EqBool { boolean },
        _ => return Ok(None),
    };

    Ok(Some(inst))
}

/// Assemble an [hir::Condition<'_>].
#[instrument(span = condition)]
fn condition<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    condition: &hir::Condition<'hir>,
    then_label: &Label,
) -> compile::Result<Layer<'hir>> {
    match *condition {
        hir::Condition::Expr(e) => {
            let guard = cx.scopes.child(e)?;
            expr(cx, e, Needs::Value)?.apply(cx)?;
            cx.asm.jump_if(then_label, e)?;
            Ok(cx.scopes.pop(guard, e)?)
        }
        hir::Condition::ExprLet(expr_let) => {
            let span = expr_let;

            let false_label = cx.asm.new_label("if_condition_false");

            let expected = cx.scopes.child(span)?;

            let load = |cx: &mut Ctxt<'_, 'hir, '_>, needs: Needs| {
                expr(cx, &expr_let.expr, needs)?.apply(cx)?;
                Ok(())
            };

            if pat(cx, &expr_let.pat, &false_label, &load)? {
                cx.asm.jump(then_label, span)?;
                cx.asm.label(&false_label)?;
            } else {
                cx.asm.jump(then_label, span)?;
            };

            Ok(cx.scopes.pop(expected, span)?)
        }
    }
}

/// Encode a vector pattern match.
#[instrument(span = span)]
fn pat_sequence<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::PatSequence<'hir>,
    span: &dyn Spanned,
    false_label: &Label,
    load: &dyn Fn(&mut Ctxt<'_, 'hir, '_>, Needs) -> compile::Result<()>,
) -> compile::Result<()> {
    load(cx, Needs::Value)?;

    if matches!(
        hir.kind,
        hir::PatSequenceKind::Anonymous {
            type_check: TypeCheck::Tuple,
            count: 0,
            is_open: false
        }
    ) {
        cx.asm.push(Inst::IsUnit, span)?;
        cx.asm
            .pop_and_jump_if_not(cx.scopes.local(span)?, false_label, span)?;
        return Ok(());
    }

    // Assign the yet-to-be-verified tuple to an anonymous slot, so we can
    // interact with it multiple times.
    let offset = cx.scopes.alloc(span)?;

    let inst = pat_sequence_kind_to_inst(hir.kind);

    cx.asm.push(Inst::Copy { offset }, span)?;
    cx.asm.push(inst, span)?;

    cx.asm
        .pop_and_jump_if_not(cx.scopes.local(span)?, false_label, span)?;

    for (index, p) in hir.items.iter().enumerate() {
        let load = move |cx: &mut Ctxt<'_, 'hir, '_>, needs: Needs| {
            if needs.value() {
                cx.asm.push(Inst::TupleIndexGetAt { offset, index }, p)?;
            }

            Ok(())
        };

        pat(cx, p, false_label, &load)?;
    }

    Ok(())
}

fn pat_sequence_kind_to_inst(kind: hir::PatSequenceKind) -> Inst {
    match kind {
        hir::PatSequenceKind::Type { hash } => Inst::MatchType { hash },
        hir::PatSequenceKind::BuiltInVariant { type_check } => Inst::MatchBuiltIn { type_check },
        hir::PatSequenceKind::Variant {
            variant_hash,
            enum_hash,
            index,
        } => Inst::MatchVariant {
            variant_hash,
            enum_hash,
            index,
        },
        hir::PatSequenceKind::Anonymous {
            type_check,
            count,
            is_open,
        } => Inst::MatchSequence {
            type_check,
            len: count,
            exact: !is_open,
        },
    }
}

/// Assemble an object pattern.
#[instrument(span = span)]
fn pat_object<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::PatObject<'hir>,
    span: &dyn Spanned,
    false_label: &Label,
    load: &dyn Fn(&mut Ctxt<'_, 'hir, '_>, Needs) -> compile::Result<()>,
) -> compile::Result<()> {
    // NB: bind the loaded variable (once) to an anonymous var.
    // We reduce the number of copy operations by having specialized
    // operations perform the load from the given offset.
    load(cx, Needs::Value)?;
    let offset = cx.scopes.alloc(span)?;

    let mut string_slots = Vec::new();

    for binding in hir.bindings {
        string_slots.try_push(cx.q.unit.new_static_string(span, binding.key())?)?;
    }

    let inst = match hir.kind {
        hir::PatSequenceKind::Type { hash } => Inst::MatchType { hash },
        hir::PatSequenceKind::BuiltInVariant { type_check } => Inst::MatchBuiltIn { type_check },
        hir::PatSequenceKind::Variant {
            variant_hash,
            enum_hash,
            index,
        } => Inst::MatchVariant {
            variant_hash,
            enum_hash,
            index,
        },
        hir::PatSequenceKind::Anonymous { is_open, .. } => {
            let keys =
                cx.q.unit
                    .new_static_object_keys_iter(span, hir.bindings.iter().map(|b| b.key()))?;

            Inst::MatchObject {
                slot: keys,
                exact: !is_open,
            }
        }
    };

    // Copy the temporary and check that its length matches the pattern and
    // that it is indeed a vector.
    cx.asm.push(Inst::Copy { offset }, span)?;
    cx.asm.push(inst, span)?;

    cx.asm
        .pop_and_jump_if_not(cx.scopes.local(span)?, false_label, span)?;

    for (binding, slot) in hir.bindings.iter().zip(string_slots) {
        match *binding {
            hir::Binding::Binding(span, _, p) => {
                let load = move |cx: &mut Ctxt<'_, 'hir, '_>, needs: Needs| {
                    if needs.value() {
                        cx.asm
                            .push(Inst::ObjectIndexGetAt { offset, slot }, &span)?;
                    }

                    Ok(())
                };

                pat(cx, p, false_label, &load)?;
            }
            hir::Binding::Ident(span, name) => {
                cx.asm
                    .push(Inst::ObjectIndexGetAt { offset, slot }, &span)?;
                cx.scopes.define(hir::Name::Str(name), binding)?;
            }
        }
    }

    Ok(())
}

/// Call a block.
#[instrument(span = hir)]
fn block<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::Block<'hir>,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    cx.contexts.try_push(hir.span())?;
    let scopes_count = cx.scopes.child(hir)?;

    let mut last = None::<(&hir::Expr<'_>, bool)>;

    for stmt in hir.statements {
        let (e, semi) = match stmt {
            hir::Stmt::Local(l) => {
                if let Some((e, _)) = take(&mut last) {
                    // NB: terminated expressions do not need to produce a value.
                    expr(cx, e, Needs::None)?.apply(cx)?;
                }

                local(cx, l, Needs::None)?.apply(cx)?;
                continue;
            }
            hir::Stmt::Expr(expr) => (expr, false),
            hir::Stmt::Semi(expr) => (expr, true),
            hir::Stmt::Item(..) => continue,
        };

        if let Some((e, _)) = replace(&mut last, Some((e, semi))) {
            // NB: terminated expressions do not need to produce a value.
            expr(cx, e, Needs::None)?.apply(cx)?;
        }
    }

    let produced = if let Some((e, semi)) = last {
        if semi {
            expr(cx, e, Needs::None)?.apply(cx)?;
            false
        } else {
            expr(cx, e, needs)?.apply(cx)?;
            true
        }
    } else {
        false
    };

    let scope = cx.scopes.pop(scopes_count, hir)?;

    if needs.value() {
        if produced {
            cx.locals_clean(scope.local, hir)?;
        } else {
            cx.locals_pop(scope.local, hir)?;
            cx.asm.push(Inst::unit(), hir)?;
        }
    } else {
        cx.locals_pop(scope.local, hir)?;
    }

    cx.contexts
        .pop()
        .ok_or("Missing parent context")
        .with_span(hir)?;

    Ok(Asm::top(hir))
}

/// Assemble #[builtin] format_args!(...) macro.
#[instrument(span = format)]
fn builtin_format<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    format: &'hir hir::BuiltInFormat<'hir>,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    use crate::runtime::format;

    let fill = format.fill.unwrap_or(' ');
    let align = format.align.unwrap_or_default();
    let flags = format.flags.unwrap_or_default();
    let width = format.width;
    let precision = format.precision;
    let format_type = format.format_type.unwrap_or_default();

    let spec = format::FormatSpec::new(flags, fill, align, width, precision, format_type);

    expr(cx, &format.value, Needs::Value)?.apply(cx)?;
    cx.asm.push(Inst::Format { spec }, format)?;

    if !needs.value() {
        cx.asm.push(Inst::Pop, format)?;
    }

    Ok(Asm::top(format))
}

/// Assemble #[builtin] template!(...) macro.
#[instrument(span = template)]
fn builtin_template<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    template: &hir::BuiltInTemplate<'hir>,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    let span = template;

    let expected = cx.scopes.child(span)?;
    let mut size_hint = 0;
    let mut expansions = 0;

    for hir in template.exprs {
        if let hir::ExprKind::Lit(hir::Lit::Str(s)) = hir.kind {
            size_hint += s.len();
            let slot = cx.q.unit.new_static_string(span, s)?;
            cx.asm.push(Inst::String { slot }, span)?;
            cx.scopes.alloc(span)?;
            continue;
        }

        expansions += 1;

        expr(cx, hir, Needs::Value)?.apply(cx)?;
        cx.scopes.alloc(span)?;
    }

    if template.from_literal && expansions == 0 {
        cx.q.diagnostics
            .template_without_expansions(cx.source_id, span, cx.context())?;
    }

    cx.asm.push(
        Inst::StringConcat {
            len: template.exprs.len(),
            size_hint,
        },
        span,
    )?;

    if !needs.value() {
        cx.asm.push(Inst::Pop, span)?;
    }

    let _ = cx.scopes.pop(expected, span)?;
    Ok(Asm::top(span))
}

/// Assemble a constant value.
#[instrument(span = span)]
fn const_<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    value: &ConstValue,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<()> {
    if !needs.value() {
        cx.q.diagnostics
            .not_used(cx.source_id, span, cx.context())?;
        return Ok(());
    }

    match value {
        ConstValue::EmptyTuple => {
            cx.asm.push(Inst::unit(), span)?;
        }
        ConstValue::Byte(b) => {
            cx.asm.push(Inst::byte(*b), span)?;
        }
        ConstValue::Char(ch) => {
            cx.asm.push(Inst::char(*ch), span)?;
        }
        ConstValue::Integer(n) => {
            cx.asm.push(Inst::integer(*n), span)?;
        }
        ConstValue::Float(n) => {
            cx.asm.push(Inst::float(*n), span)?;
        }
        ConstValue::Bool(b) => {
            cx.asm.push(Inst::bool(*b), span)?;
        }
        ConstValue::String(s) => {
            let slot = cx.q.unit.new_static_string(span, s)?;
            cx.asm.push(Inst::String { slot }, span)?;
        }
        ConstValue::Bytes(b) => {
            let slot = cx.q.unit.new_static_bytes(span, b)?;
            cx.asm.push(Inst::Bytes { slot }, span)?;
        }
        ConstValue::Option(option) => match option {
            Some(value) => {
                const_(cx, value, span, Needs::Value)?;
                cx.asm.push(
                    Inst::Variant {
                        variant: InstVariant::Some,
                    },
                    span,
                )?;
            }
            None => {
                cx.asm.push(
                    Inst::Variant {
                        variant: InstVariant::None,
                    },
                    span,
                )?;
            }
        },
        ConstValue::Vec(vec) => {
            for value in vec.iter() {
                const_(cx, value, span, Needs::Value)?;
            }

            cx.asm.push(Inst::Vec { count: vec.len() }, span)?;
        }
        ConstValue::Tuple(tuple) => {
            for value in tuple.iter() {
                const_(cx, value, span, Needs::Value)?;
            }

            cx.asm.push(Inst::Tuple { count: tuple.len() }, span)?;
        }
        ConstValue::Object(object) => {
            let mut entries = object.iter().try_collect::<Vec<_>>()?;
            entries.sort_by_key(|k| k.0);

            for (_, value) in entries.iter().copied() {
                const_(cx, value, span, Needs::Value)?;
            }

            let slot =
                cx.q.unit
                    .new_static_object_keys_iter(span, entries.iter().map(|e| e.0))?;

            cx.asm.push(Inst::Object { slot }, span)?;
        }
    }

    Ok(())
}

/// Assemble an expression.
#[instrument(span = hir)]
fn expr<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::Expr<'hir>,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    let span = hir;

    let asm = match hir.kind {
        hir::ExprKind::Variable(name) => {
            let var = cx.scopes.get(&mut cx.q, name, span)?;
            Asm::var(span, var)
        }
        hir::ExprKind::Type(ty) => {
            cx.asm.push(
                Inst::Push {
                    value: InstValue::Type(ty),
                },
                span,
            )?;
            Asm::top(span)
        }
        hir::ExprKind::Fn(hash) => {
            cx.asm.push(Inst::LoadFn { hash }, span)?;
            Asm::top(span)
        }
        hir::ExprKind::For(hir) => expr_for(cx, hir, span, needs)?,
        hir::ExprKind::Loop(hir) => expr_loop(cx, hir, span, needs)?,
        hir::ExprKind::Let(hir) => expr_let(cx, hir, needs)?,
        hir::ExprKind::Group(hir) => expr(cx, hir, needs)?,
        hir::ExprKind::Unary(hir) => expr_unary(cx, hir, span, needs)?,
        hir::ExprKind::Assign(hir) => expr_assign(cx, hir, span, needs)?,
        hir::ExprKind::Binary(hir) => expr_binary(cx, hir, span, needs)?,
        hir::ExprKind::If(hir) => expr_if(cx, hir, span, needs)?,
        hir::ExprKind::Index(hir) => expr_index(cx, hir, span, needs)?,
        hir::ExprKind::Break(hir) => expr_break(cx, hir, span, needs)?,
        hir::ExprKind::Continue(hir) => expr_continue(cx, hir, span, needs)?,
        hir::ExprKind::Yield(hir) => expr_yield(cx, hir, span, needs)?,
        hir::ExprKind::Block(hir) => block(cx, hir, needs)?,
        hir::ExprKind::Return(hir) => expr_return(cx, hir, span, needs)?,
        hir::ExprKind::Match(hir) => expr_match(cx, hir, span, needs)?,
        hir::ExprKind::Await(hir) => expr_await(cx, hir, span, needs)?,
        hir::ExprKind::Try(hir) => expr_try(cx, hir, span, needs)?,
        hir::ExprKind::Select(hir) => expr_select(cx, hir, span, needs)?,
        hir::ExprKind::Call(hir) => expr_call(cx, hir, span, needs)?,
        hir::ExprKind::FieldAccess(hir) => expr_field_access(cx, hir, span, needs)?,
        hir::ExprKind::CallClosure(hir) => expr_call_closure(cx, hir, span, needs)?,
        hir::ExprKind::Lit(hir) => lit(cx, hir, span, needs)?,
        hir::ExprKind::Tuple(hir) => expr_tuple(cx, hir, span, needs)?,
        hir::ExprKind::Vec(hir) => expr_vec(cx, hir, span, needs)?,
        hir::ExprKind::Object(hir) => expr_object(cx, hir, span, needs)?,
        hir::ExprKind::Range(hir) => expr_range(cx, hir, span, needs)?,
        hir::ExprKind::Template(template) => builtin_template(cx, template, needs)?,
        hir::ExprKind::Format(format) => builtin_format(cx, format, needs)?,
        hir::ExprKind::AsyncBlock(hir) => expr_async_block(cx, hir, span, needs)?,
        hir::ExprKind::Const(id) => const_item(cx, id, span, needs)?,
        hir::ExprKind::Path => {
            return Err(compile::Error::msg(
                span,
                "Path expression is not supported here",
            ))
        }
    };

    Ok(asm)
}

/// Assemble an assign expression.
#[instrument(span = span)]
fn expr_assign<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::ExprAssign<'hir>,
    span: &'hir dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    let supported = match hir.lhs.kind {
        // <var> = <value>
        hir::ExprKind::Variable(name) => {
            expr(cx, &hir.rhs, Needs::Value)?.apply(cx)?;
            let var = cx.scopes.get(&mut cx.q, name, span)?;
            cx.asm.push_with_comment(
                Inst::Replace { offset: var.offset },
                span,
                &format_args!("var `{var}`"),
            )?;
            true
        }
        // <expr>.<field> = <value>
        hir::ExprKind::FieldAccess(field_access) => {
            // field assignment
            match field_access.expr_field {
                hir::ExprField::Ident(ident) => {
                    let slot = cx.q.unit.new_static_string(span, ident)?;

                    expr(cx, &hir.rhs, Needs::Value)?.apply(cx)?;
                    cx.scopes.alloc(&hir.rhs)?;

                    expr(cx, &field_access.expr, Needs::Value)?.apply(cx)?;
                    cx.scopes.alloc(span)?;

                    cx.asm.push(Inst::ObjectIndexSet { slot }, span)?;
                    cx.scopes.free(span, 2)?;
                    true
                }
                hir::ExprField::Index(index) => {
                    expr(cx, &hir.rhs, Needs::Value)?.apply(cx)?;
                    cx.scopes.alloc(&hir.rhs)?;

                    expr(cx, &field_access.expr, Needs::Value)?.apply(cx)?;
                    cx.asm.push(Inst::TupleIndexSet { index }, span)?;
                    cx.scopes.free(span, 1)?;
                    true
                }
                _ => {
                    return Err(compile::Error::new(span, ErrorKind::BadFieldAccess));
                }
            }
        }
        hir::ExprKind::Index(expr_index_get) => {
            expr(cx, &hir.rhs, Needs::Value)?.apply(cx)?;
            cx.scopes.alloc(span)?;

            expr(cx, &expr_index_get.target, Needs::Value)?.apply(cx)?;
            cx.scopes.alloc(span)?;

            expr(cx, &expr_index_get.index, Needs::Value)?.apply(cx)?;
            cx.scopes.alloc(span)?;

            cx.asm.push(Inst::IndexSet, span)?;
            cx.scopes.free(span, 3)?;
            true
        }
        _ => false,
    };

    if !supported {
        return Err(compile::Error::new(span, ErrorKind::UnsupportedAssignExpr));
    }

    if needs.value() {
        cx.asm.push(Inst::unit(), span)?;
    }

    Ok(Asm::top(span))
}

/// Assemble an `.await` expression.
#[instrument(span = hir)]
fn expr_await<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::Expr<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    expr(cx, hir, Needs::Value)?.apply(cx)?;
    cx.asm.push(Inst::Await, span)?;

    if !needs.value() {
        cx.asm.push(Inst::Pop, span)?;
    }

    Ok(Asm::top(span))
}

/// Assemble a binary expression.
#[instrument(span = span)]
fn expr_binary<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::ExprBinary<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    // Special expressions which operates on the stack in special ways.
    if hir.op.is_assign() {
        compile_assign_binop(cx, &hir.lhs, &hir.rhs, &hir.op, span, needs)?;
        return Ok(Asm::top(span));
    }

    if hir.op.is_conditional() {
        compile_conditional_binop(cx, &hir.lhs, &hir.rhs, &hir.op, span, needs)?;
        return Ok(Asm::top(span));
    }

    let guard = cx.scopes.child(span)?;

    // NB: need to declare these as anonymous local variables so that they
    // get cleaned up in case there is an early break (return, try, ...).
    let a = expr(cx, &hir.lhs, Needs::Value)?.apply_targeted(cx)?;
    let b = expr(cx, &hir.rhs, Needs::Value)?.apply_targeted(cx)?;

    let op = match hir.op {
        ast::BinOp::Eq(..) => InstOp::Eq,
        ast::BinOp::Neq(..) => InstOp::Neq,
        ast::BinOp::Lt(..) => InstOp::Lt,
        ast::BinOp::Gt(..) => InstOp::Gt,
        ast::BinOp::Lte(..) => InstOp::Lte,
        ast::BinOp::Gte(..) => InstOp::Gte,
        ast::BinOp::As(..) => InstOp::As,
        ast::BinOp::Is(..) => InstOp::Is,
        ast::BinOp::IsNot(..) => InstOp::IsNot,
        ast::BinOp::And(..) => InstOp::And,
        ast::BinOp::Or(..) => InstOp::Or,
        ast::BinOp::Add(..) => InstOp::Add,
        ast::BinOp::Sub(..) => InstOp::Sub,
        ast::BinOp::Div(..) => InstOp::Div,
        ast::BinOp::Mul(..) => InstOp::Mul,
        ast::BinOp::Rem(..) => InstOp::Rem,
        ast::BinOp::BitAnd(..) => InstOp::BitAnd,
        ast::BinOp::BitXor(..) => InstOp::BitXor,
        ast::BinOp::BitOr(..) => InstOp::BitOr,
        ast::BinOp::Shl(..) => InstOp::Shl,
        ast::BinOp::Shr(..) => InstOp::Shr,

        op => {
            return Err(compile::Error::new(
                span,
                ErrorKind::UnsupportedBinaryOp { op },
            ));
        }
    };

    cx.asm.push(Inst::Op { op, a, b }, span)?;

    // NB: we put it here to preserve the call in case it has side effects.
    // But if we don't need the value, then pop it from the stack.
    if !needs.value() {
        cx.asm.push(Inst::Pop, span)?;
    }

    cx.scopes.pop(guard, span)?;
    return Ok(Asm::top(span));

    fn compile_conditional_binop<'hir>(
        cx: &mut Ctxt<'_, 'hir, '_>,
        lhs: &'hir hir::Expr<'hir>,
        rhs: &'hir hir::Expr<'hir>,
        bin_op: &ast::BinOp,
        span: &dyn Spanned,
        needs: Needs,
    ) -> compile::Result<()> {
        let end_label = cx.asm.new_label("conditional_end");

        expr(cx, lhs, Needs::Value)?.apply(cx)?;

        match bin_op {
            ast::BinOp::And(..) => {
                cx.asm.jump_if_not_or_pop(&end_label, lhs)?;
            }
            ast::BinOp::Or(..) => {
                cx.asm.jump_if_or_pop(&end_label, lhs)?;
            }
            op => {
                return Err(compile::Error::new(
                    span,
                    ErrorKind::UnsupportedBinaryOp { op: *op },
                ));
            }
        }

        expr(cx, rhs, Needs::Value)?.apply(cx)?;

        cx.asm.label(&end_label)?;

        if !needs.value() {
            cx.asm.push(Inst::Pop, span)?;
        }

        Ok(())
    }

    fn compile_assign_binop<'hir>(
        cx: &mut Ctxt<'_, 'hir, '_>,
        lhs: &'hir hir::Expr<'hir>,
        rhs: &'hir hir::Expr<'hir>,
        bin_op: &ast::BinOp,
        span: &dyn Spanned,
        needs: Needs,
    ) -> compile::Result<()> {
        let supported = match lhs.kind {
            // <var> <op> <expr>
            hir::ExprKind::Variable(name) => {
                expr(cx, rhs, Needs::Value)?.apply(cx)?;
                let var = cx.scopes.get(&mut cx.q, name, lhs)?;
                Some(InstTarget::Offset(var.offset))
            }
            // <expr>.<field> <op> <value>
            hir::ExprKind::FieldAccess(field_access) => {
                expr(cx, &field_access.expr, Needs::Value)?.apply(cx)?;
                expr(cx, rhs, Needs::Value)?.apply(cx)?;

                // field assignment
                match field_access.expr_field {
                    hir::ExprField::Index(index) => Some(InstTarget::TupleField(index)),
                    hir::ExprField::Ident(ident) => {
                        let n = cx.q.unit.new_static_string(&field_access.expr, ident)?;
                        Some(InstTarget::Field(n))
                    }
                    _ => {
                        return Err(compile::Error::new(span, ErrorKind::BadFieldAccess));
                    }
                }
            }
            _ => None,
        };

        let Some(target) = supported else {
            return Err(compile::Error::new(span, ErrorKind::UnsupportedBinaryExpr));
        };

        let op = match bin_op {
            ast::BinOp::AddAssign(..) => InstAssignOp::Add,
            ast::BinOp::SubAssign(..) => InstAssignOp::Sub,
            ast::BinOp::MulAssign(..) => InstAssignOp::Mul,
            ast::BinOp::DivAssign(..) => InstAssignOp::Div,
            ast::BinOp::RemAssign(..) => InstAssignOp::Rem,
            ast::BinOp::BitAndAssign(..) => InstAssignOp::BitAnd,
            ast::BinOp::BitXorAssign(..) => InstAssignOp::BitXor,
            ast::BinOp::BitOrAssign(..) => InstAssignOp::BitOr,
            ast::BinOp::ShlAssign(..) => InstAssignOp::Shl,
            ast::BinOp::ShrAssign(..) => InstAssignOp::Shr,
            _ => {
                return Err(compile::Error::new(span, ErrorKind::UnsupportedBinaryExpr));
            }
        };

        cx.asm.push(Inst::Assign { target, op }, span)?;

        if needs.value() {
            cx.asm.push(Inst::unit(), span)?;
        }

        Ok(())
    }
}

/// Assemble a block expression.
#[instrument(span = span)]
fn expr_async_block<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::ExprAsyncBlock<'hir>,
    span: &'hir dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    for capture in hir.captures.iter().copied() {
        if hir.do_move {
            let var = cx.scopes.take(&mut cx.q, capture, span)?;
            var.do_move(cx.asm, span, &"capture")?;
        } else {
            let var = cx.scopes.get(&mut cx.q, capture, span)?;
            var.copy(cx, span, &"capture")?;
        }
    }

    cx.asm.push_with_comment(
        Inst::Call {
            hash: hir.hash,
            args: hir.captures.len(),
        },
        span,
        &"async block",
    )?;

    if !needs.value() {
        cx.asm
            .push_with_comment(Inst::Pop, span, &"value is not needed")?;
    }

    Ok(Asm::top(span))
}

/// Assemble a constant item.
#[instrument(span = span)]
fn const_item<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hash: Hash,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    let Some(const_value) = cx.q.get_const_value(hash) else {
        return Err(compile::Error::msg(
            span,
            try_format!("Missing constant value for hash {hash}"),
        ));
    };

    let const_value = const_value.try_clone().with_span(span)?;
    const_(cx, &const_value, span, needs)?;
    Ok(Asm::top(span))
}

/// Assemble a break expression.
///
/// NB: loops are expected to produce a value at the end of their expression.
#[instrument(span = span)]
fn expr_break<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::ExprBreak<'hir>,
    span: &dyn Spanned,
    _: Needs,
) -> compile::Result<Asm<'hir>> {
    let Some(current_loop) = cx.loops.last().try_cloned()? else {
        return Err(compile::Error::new(span, ErrorKind::BreakOutsideOfLoop));
    };

    let (last_loop, to_drop, has_value) = match (hir.label, hir.expr) {
        (None, Some(e)) => {
            expr(cx, e, current_loop.needs)?.apply(cx)?;
            let to_drop = current_loop.drop.into_iter().try_collect()?;
            (current_loop, to_drop, true)
        }
        (Some(label), None) => {
            let (last_loop, to_drop) = cx.loops.walk_until_label(label, span)?;
            (last_loop.try_clone()?, to_drop, false)
        }
        (Some(label), Some(e)) => {
            expr(cx, e, current_loop.needs)?.apply(cx)?;
            let (last_loop, to_drop) = cx.loops.walk_until_label(label, span)?;
            (last_loop.try_clone()?, to_drop, true)
        }
        (None, None) => {
            let to_drop = current_loop.drop.into_iter().try_collect()?;
            (current_loop, to_drop, false)
        }
    };

    // Drop loop temporaries. Typically an iterator.
    for offset in to_drop {
        cx.asm.push(Inst::Drop { offset }, span)?;
    }

    let vars = cx
        .scopes
        .total(span)?
        .checked_sub(last_loop.break_var_count)
        .ok_or("Var count should be larger")
        .with_span(span)?;

    if last_loop.needs.value() {
        if has_value {
            cx.locals_clean(vars, span)?;
        } else {
            cx.locals_pop(vars, span)?;
            cx.asm.push(Inst::unit(), span)?;
        }
    } else {
        cx.locals_pop(vars, span)?;
    }

    cx.asm.jump(&last_loop.break_label, span)?;
    Ok(Asm::top(span))
}

/// Assemble a call expression.
#[instrument(span = span)]
fn expr_call<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::ExprCall<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    let args = hir.args.len();

    match hir.call {
        hir::Call::Var { name, .. } => {
            let var = cx.scopes.get(&mut cx.q, name, span)?;

            for e in hir.args {
                expr(cx, e, Needs::Value)?.apply(cx)?;
                cx.scopes.alloc(span)?;
            }

            var.copy(cx, span, &"call")?;
            cx.scopes.alloc(span)?;

            cx.asm.push(Inst::CallFn { args }, span)?;

            cx.scopes.free(span, hir.args.len() + 1)?;
        }
        hir::Call::Associated { target, hash } => {
            expr(cx, target, Needs::Value)?.apply(cx)?;
            cx.scopes.alloc(target)?;

            for e in hir.args {
                expr(cx, e, Needs::Value)?.apply(cx)?;
                cx.scopes.alloc(span)?;
            }

            cx.asm.push(Inst::CallAssociated { hash, args }, span)?;
            cx.scopes.free(span, hir.args.len() + 1)?;
        }
        hir::Call::Meta { hash } => {
            for e in hir.args {
                expr(cx, e, Needs::Value)?.apply(cx)?;
                cx.scopes.alloc(span)?;
            }

            cx.asm.push(Inst::Call { hash, args }, span)?;
            cx.scopes.free(span, args)?;
        }
        hir::Call::Expr { expr: e } => {
            for e in hir.args {
                expr(cx, e, Needs::Value)?.apply(cx)?;
                cx.scopes.alloc(span)?;
            }

            expr(cx, e, Needs::Value)?.apply(cx)?;
            cx.scopes.alloc(span)?;

            cx.asm.push(Inst::CallFn { args }, span)?;

            cx.scopes.free(span, args + 1)?;
        }
        hir::Call::ConstFn {
            from_module,
            from_item,
            id,
        } => {
            let const_fn = cx.q.const_fn_for(id).with_span(span)?;
            let value = cx.call_const_fn(span, from_module, from_item, &const_fn, hir.args)?;
            const_(cx, &value, span, Needs::Value)?;
        }
    }

    if !needs.value() {
        cx.asm.push(Inst::Pop, span)?;
    }

    Ok(Asm::top(span))
}

/// Assemble a closure expression.
#[instrument(span = span)]
fn expr_call_closure<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::ExprCallClosure<'hir>,
    span: &'hir dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    if !needs.value() {
        cx.q.diagnostics
            .not_used(cx.source_id, span, cx.context())?;
        return Ok(Asm::top(span));
    }

    tracing::trace!(?hir.captures, "assemble call closure");

    // Construct a closure environment.
    for capture in hir.captures.iter().copied() {
        if hir.do_move {
            let var = cx.scopes.take(&mut cx.q, capture, span)?;
            var.do_move(cx.asm, span, &"capture")?;
        } else {
            let var = cx.scopes.get(&mut cx.q, capture, span)?;
            var.copy(cx, span, &"capture")?;
        }
    }

    cx.asm.push(
        Inst::Closure {
            hash: hir.hash,
            count: hir.captures.len(),
        },
        span,
    )?;

    Ok(Asm::top(span))
}

/// Assemble a continue expression.
#[instrument(span = span)]
fn expr_continue<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::ExprContinue<'hir>,
    span: &dyn Spanned,
    _: Needs,
) -> compile::Result<Asm<'hir>> {
    let Some(current_loop) = cx.loops.last().try_cloned()? else {
        return Err(compile::Error::new(span, ErrorKind::ContinueOutsideOfLoop));
    };

    let last_loop = if let Some(label) = hir.label {
        let (last_loop, _) = cx.loops.walk_until_label(label, span)?;
        last_loop.try_clone()?
    } else {
        current_loop
    };

    let vars = cx
        .scopes
        .total(span)?
        .checked_sub(last_loop.continue_var_count)
        .ok_or("Var count should be larger")
        .with_span(span)?;

    cx.locals_pop(vars, span)?;

    cx.asm.jump(&last_loop.continue_label, span)?;
    Ok(Asm::top(span))
}

/// Assemble an expr field access, like `<value>.<field>`.
#[instrument(span = span)]
fn expr_field_access<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::ExprFieldAccess<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    // Optimizations!
    //
    // TODO: perform deferred compilation for expressions instead, so we can
    // e.g. inspect if it compiles down to a local access instead of
    // climbing the hir like we do here.
    if let (hir::ExprKind::Variable(name), hir::ExprField::Index(index)) =
        (hir.expr.kind, hir.expr_field)
    {
        let var = cx.scopes.get(&mut cx.q, name, span)?;

        cx.asm.push_with_comment(
            Inst::TupleIndexGetAt {
                offset: var.offset,
                index,
            },
            span,
            &var,
        )?;

        if !needs.value() {
            cx.q.diagnostics
                .not_used(cx.source_id, span, cx.context())?;
            cx.asm.push(Inst::Pop, span)?;
        }

        return Ok(Asm::top(span));
    }

    expr(cx, &hir.expr, Needs::Value)?.apply(cx)?;

    match hir.expr_field {
        hir::ExprField::Index(index) => {
            cx.asm.push(Inst::TupleIndexGet { index }, span)?;

            if !needs.value() {
                cx.q.diagnostics
                    .not_used(cx.source_id, span, cx.context())?;
                cx.asm.push(Inst::Pop, span)?;
            }

            Ok(Asm::top(span))
        }
        hir::ExprField::Ident(field) => {
            let slot = cx.q.unit.new_static_string(span, field)?;

            cx.asm.push(Inst::ObjectIndexGet { slot }, span)?;

            if !needs.value() {
                cx.q.diagnostics
                    .not_used(cx.source_id, span, cx.context())?;
                cx.asm.push(Inst::Pop, span)?;
            }

            Ok(Asm::top(span))
        }
        _ => Err(compile::Error::new(span, ErrorKind::BadFieldAccess)),
    }
}

/// Assemble an expression for loop.
#[instrument(span = span)]
fn expr_for<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::ExprFor<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    let continue_label = cx.asm.new_label("for_continue");
    let end_label = cx.asm.new_label("for_end");
    let break_label = cx.asm.new_label("for_break");

    let break_var_count = cx.scopes.total(span)?;

    let (iter_offset, loop_scope_expected) = {
        let loop_scope_expected = cx.scopes.child(span)?;
        expr(cx, &hir.iter, Needs::Value)?.apply(cx)?;

        let iter_offset = cx.scopes.alloc(span)?;

        cx.asm.push_with_comment(
            Inst::CallAssociated {
                hash: *Protocol::INTO_ITER,
                args: 0,
            },
            &hir.iter,
            &format_args!("into_iter (offset: {})", iter_offset),
        )?;

        (iter_offset, loop_scope_expected)
    };

    // Declare named loop variable.
    let binding_offset = {
        cx.asm.push(Inst::unit(), &hir.iter)?;
        cx.scopes.alloc(&hir.binding)?
    };

    // Declare storage for memoized `next` instance fn.
    let next_offset = if cx.options.memoize_instance_fn {
        let offset = cx.scopes.alloc(&hir.iter)?;

        // Declare the named loop variable and put it in the scope.
        cx.asm.push_with_comment(
            Inst::Copy {
                offset: iter_offset,
            },
            &hir.iter,
            &"copy iterator (memoize)",
        )?;

        cx.asm.push_with_comment(
            Inst::LoadInstanceFn {
                hash: *Protocol::NEXT,
            },
            &hir.iter,
            &"load instance fn (memoize)",
        )?;

        Some(offset)
    } else {
        None
    };

    let continue_var_count = cx.scopes.total(span)?;
    cx.asm.label(&continue_label)?;

    cx.loops.push(Loop {
        label: hir.label,
        continue_label: continue_label.try_clone()?,
        continue_var_count,
        break_label: break_label.try_clone()?,
        break_var_count,
        needs,
        drop: Some(iter_offset),
    })?;

    // Use the memoized loop variable.
    if let Some(next_offset) = next_offset {
        cx.asm.push_with_comment(
            Inst::Copy {
                offset: iter_offset,
            },
            &hir.iter,
            &"copy iterator",
        )?;

        cx.asm.push_with_comment(
            Inst::Copy {
                offset: next_offset,
            },
            &hir.iter,
            &"copy next",
        )?;

        cx.asm.push(Inst::CallFn { args: 1 }, span)?;

        cx.asm.push(
            Inst::Replace {
                offset: binding_offset,
            },
            &hir.binding,
        )?;
    } else {
        // call the `next` function to get the next level of iteration, bind the
        // result to the loop variable in the loop.
        cx.asm.push(
            Inst::Copy {
                offset: iter_offset,
            },
            &hir.iter,
        )?;

        cx.asm.push_with_comment(
            Inst::CallAssociated {
                hash: *Protocol::NEXT,
                args: 0,
            },
            span,
            &"next",
        )?;

        cx.asm.push(
            Inst::Replace {
                offset: binding_offset,
            },
            &hir.binding,
        )?;
    }

    // Test loop condition and unwrap the option, or jump to `end_label` if the current value is `None`.
    cx.asm.iter_next(binding_offset, &end_label, &hir.binding)?;

    let guard = cx.scopes.child(&hir.body)?;

    pat_with_offset(cx, &hir.binding, binding_offset)?;

    block(cx, &hir.body, Needs::None)?.apply(cx)?;
    cx.clean_last_scope(span, guard, Needs::None)?;

    cx.asm.jump(&continue_label, span)?;
    cx.asm.label(&end_label)?;

    // Drop the iterator.
    cx.asm.push(
        Inst::Drop {
            offset: iter_offset,
        },
        span,
    )?;

    cx.clean_last_scope(span, loop_scope_expected, Needs::None)?;

    // NB: If a value is needed from a for loop, encode it as a unit.
    if needs.value() {
        cx.asm.push(Inst::unit(), span)?;
    }

    // NB: breaks produce their own value.
    cx.asm.label(&break_label)?;
    cx.loops.pop();
    Ok(Asm::top(span))
}

/// Assemble an if expression.
#[instrument(span = span)]
fn expr_if<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::Conditional<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    let end_label = cx.asm.new_label("if_end");

    let mut branches = Vec::new();
    let mut fallback = None;

    for branch in hir.branches {
        if fallback.is_some() {
            continue;
        }

        let Some(cond) = branch.condition else {
            fallback = Some(&branch.block);
            continue;
        };

        let label = cx.asm.new_label("if_branch");
        let scope = condition(cx, cond, &label)?;
        branches.try_push((branch, label, scope))?;
    }

    // use fallback as fall through.
    if let Some(b) = fallback {
        block(cx, b, needs)?.apply(cx)?;
    } else {
        // NB: if we must produce a value and there is no fallback branch,
        // encode the result of the statement as a unit.
        if needs.value() {
            cx.asm.push(Inst::unit(), span)?;
        }
    }

    cx.asm.jump(&end_label, span)?;

    let mut it = branches.into_iter().peekable();

    while let Some((branch, label, scope)) = it.next() {
        cx.asm.label(&label)?;

        let scopes = cx.scopes.push(scope)?;
        block(cx, &branch.block, needs)?.apply(cx)?;
        cx.clean_last_scope(branch, scopes, needs)?;

        if it.peek().is_some() {
            cx.asm.jump(&end_label, branch)?;
        }
    }

    cx.asm.label(&end_label)?;
    Ok(Asm::top(span))
}

/// Assemble an expression.
#[instrument(span = span)]
fn expr_index<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::ExprIndex<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    let guard = cx.scopes.child(span)?;

    let target = expr(cx, &hir.target, Needs::Value)?.apply_targeted(cx)?;
    let index = expr(cx, &hir.index, Needs::Value)?.apply_targeted(cx)?;

    cx.asm.push(Inst::IndexGet { index, target }, span)?;

    // NB: we still need to perform the operation since it might have side
    // effects, but pop the result in case a value is not needed.
    if !needs.value() {
        cx.asm.push(Inst::Pop, span)?;
    }

    cx.scopes.pop(guard, span)?;
    Ok(Asm::top(span))
}

/// Assemble a let expression.
#[instrument(span = hir)]
fn expr_let<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::ExprLet<'hir>,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    let load = |cx: &mut Ctxt<'_, 'hir, '_>, needs: Needs| {
        // NB: assignments "move" the value being assigned.
        expr(cx, &hir.expr, needs)?.apply(cx)?;
        Ok(())
    };

    let false_label = cx.asm.new_label("let_panic");

    if pat(cx, &hir.pat, &false_label, &load)? {
        cx.q.diagnostics
            .let_pattern_might_panic(cx.source_id, hir, cx.context())?;

        let ok_label = cx.asm.new_label("let_ok");
        cx.asm.jump(&ok_label, hir)?;
        cx.asm.label(&false_label)?;
        cx.asm.push(
            Inst::Panic {
                reason: PanicReason::UnmatchedPattern,
            },
            hir,
        )?;

        cx.asm.label(&ok_label)?;
    }

    // If a value is needed for a let expression, it is evaluated as a unit.
    if needs.value() {
        cx.asm.push(Inst::unit(), hir)?;
    }

    Ok(Asm::top(hir))
}

#[instrument(span = span)]
fn expr_match<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::ExprMatch<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    let expected_scopes = cx.scopes.child(span)?;

    expr(cx, &hir.expr, Needs::Value)?.apply(cx)?;
    // Offset of the expression.
    let offset = cx.scopes.alloc(span)?;

    let end_label = cx.asm.new_label("match_end");
    let mut branches = Vec::new();

    for branch in hir.branches {
        let span = branch;

        let branch_label = cx.asm.new_label("match_branch");
        let match_false = cx.asm.new_label("match_false");

        let parent_guard = cx.scopes.child(span)?;

        let load = move |this: &mut Ctxt, needs: Needs| {
            if needs.value() {
                this.asm.push(Inst::Copy { offset }, span)?;
            }

            Ok(())
        };

        pat(cx, &branch.pat, &match_false, &load)?;

        let scope = if let Some(condition) = branch.condition {
            let span = condition;

            let guard = cx.scopes.child(span)?;

            expr(cx, condition, Needs::Value)?.apply(cx)?;
            cx.clean_last_scope(span, guard, Needs::Value)?;
            let scope = cx.scopes.pop(parent_guard, span)?;

            cx.asm
                .pop_and_jump_if_not(scope.local, &match_false, span)?;

            cx.asm.jump(&branch_label, span)?;
            scope
        } else {
            cx.scopes.pop(parent_guard, span)?
        };

        cx.asm.jump(&branch_label, span)?;
        cx.asm.label(&match_false)?;

        branches.try_push((branch_label, scope))?;
    }

    // what to do in case nothing matches and the pattern doesn't have any
    // default match branch.
    if needs.value() {
        cx.asm.push(Inst::unit(), span)?;
    }

    cx.asm.jump(&end_label, span)?;

    let mut it = hir.branches.iter().zip(&branches).peekable();

    while let Some((branch, (label, scope))) = it.next() {
        let span = branch;

        cx.asm.label(label)?;

        let expected = cx.scopes.push(scope.try_clone()?)?;
        expr(cx, &branch.body, needs)?.apply(cx)?;
        cx.clean_last_scope(span, expected, needs)?;

        if it.peek().is_some() {
            cx.asm.jump(&end_label, span)?;
        }
    }

    cx.asm.label(&end_label)?;

    // pop the implicit scope where we store the anonymous match variable.
    cx.clean_last_scope(span, expected_scopes, needs)?;
    Ok(Asm::top(span))
}

/// Compile a literal object.
#[instrument(span = span)]
fn expr_object<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::ExprObject<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    let guard = cx.scopes.child(span)?;

    let base = cx.scopes.total(span)?;

    for assign in hir.assignments.iter() {
        expr(cx, &assign.assign, Needs::Value)?.apply(cx)?;
        cx.scopes.alloc(&span)?;
    }

    let slot =
        cx.q.unit
            .new_static_object_keys_iter(span, hir.assignments.iter().map(|a| a.key.1))?;

    match hir.kind {
        hir::ExprObjectKind::EmptyStruct { hash } => {
            cx.asm.push(Inst::EmptyStruct { hash }, span)?;
        }
        hir::ExprObjectKind::Struct { hash } => {
            cx.asm.push(Inst::Struct { hash, slot }, span)?;
        }
        hir::ExprObjectKind::StructVariant { hash } => {
            cx.asm.push(Inst::StructVariant { hash, slot }, span)?;
        }
        hir::ExprObjectKind::ExternalType { hash, args } => {
            reorder_field_assignments(cx, hir, base, span)?;
            cx.asm.push(Inst::Call { hash, args }, span)?;
        }
        hir::ExprObjectKind::Anonymous => {
            cx.asm.push(Inst::Object { slot }, span)?;
        }
    }

    // No need to encode an object since the value is not needed.
    if !needs.value() {
        cx.q.diagnostics
            .not_used(cx.source_id, span, cx.context())?;
        cx.asm.push(Inst::Pop, span)?;
    }

    cx.scopes.pop(guard, span)?;
    Ok(Asm::top(span))
}

/// Reorder the position of the field assignments on the stack so that they
/// match the expected argument order when invoking the constructor function.
fn reorder_field_assignments<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::ExprObject<'hir>,
    base: usize,
    span: &dyn Spanned,
) -> compile::Result<()> {
    let mut order = Vec::try_with_capacity(hir.assignments.len())?;

    for assign in hir.assignments {
        let Some(position) = assign.position else {
            return Err(compile::Error::msg(
                span,
                try_format!("Missing position for field assignment {}", assign.key.1),
            ));
        };

        order.try_push(position)?;
    }

    for a in 0..hir.assignments.len() {
        loop {
            let Some(&b) = order.get(a) else {
                return Err(compile::Error::msg(span, "Order out-of-bounds"));
            };

            if a == b {
                break;
            }

            order.swap(a, b);

            let (Some(a), Some(b)) = (base.checked_add(a), base.checked_add(b)) else {
                return Err(compile::Error::msg(
                    span,
                    "Field repositioning out-of-bounds",
                ));
            };

            cx.asm.push(Inst::Swap { a, b }, span)?;
        }
    }

    Ok(())
}

/// Assemble a range expression.
#[instrument(span = span)]
fn expr_range<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::ExprRange<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    let guard = cx.scopes.child(span)?;

    let (range, count) = match hir {
        hir::ExprRange::RangeFrom { start } => {
            expr(cx, start, needs)?.apply(cx)?;
            cx.scopes.alloc(start)?;
            (InstRange::RangeFrom, 1)
        }
        hir::ExprRange::RangeFull => (InstRange::RangeFull, 0),
        hir::ExprRange::RangeInclusive { start, end } => {
            expr(cx, start, needs)?.apply(cx)?;
            cx.scopes.alloc(start)?;
            expr(cx, end, needs)?.apply(cx)?;
            cx.scopes.alloc(end)?;
            (InstRange::RangeInclusive, 2)
        }
        hir::ExprRange::RangeToInclusive { end } => {
            expr(cx, end, needs)?.apply(cx)?;
            cx.scopes.alloc(end)?;
            (InstRange::RangeToInclusive, 1)
        }
        hir::ExprRange::RangeTo { end } => {
            expr(cx, end, needs)?.apply(cx)?;
            cx.scopes.alloc(end)?;
            (InstRange::RangeTo, 1)
        }
        hir::ExprRange::Range { start, end } => {
            expr(cx, start, needs)?.apply(cx)?;
            cx.scopes.alloc(start)?;
            expr(cx, end, needs)?.apply(cx)?;
            cx.scopes.alloc(end)?;
            (InstRange::Range, 2)
        }
    };

    if needs.value() {
        cx.asm.push(Inst::Range { range }, span)?;
    }

    cx.scopes.free(span, count)?;
    cx.scopes.pop(guard, span)?;
    Ok(Asm::top(span))
}

/// Assemble a return expression.
#[instrument(span = span)]
fn expr_return<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: Option<&'hir hir::Expr<'hir>>,
    span: &dyn Spanned,
    _: Needs,
) -> compile::Result<Asm<'hir>> {
    // NB: drop any loop temporaries.
    for l in cx.loops.iter() {
        if let Some(offset) = l.drop {
            cx.asm.push(Inst::Drop { offset }, span)?;
        }
    }

    if let Some(e) = hir {
        return_(cx, span, e, expr)?;
    } else {
        // NB: we actually want total_var_count here since we need to clean up
        // _every_ variable declared until we reached the current return.
        let clean = cx.scopes.total(span)?;
        cx.locals_pop(clean, span)?;
        cx.asm.push(Inst::ReturnUnit, span)?;
    }

    Ok(Asm::top(span))
}

/// Assemble a select expression.
#[instrument(span = span)]
fn expr_select<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::ExprSelect<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    cx.contexts.try_push(span.span())?;

    let len = hir.branches.len();
    let mut default_branch = None;
    let mut branches = Vec::new();

    let end_label = cx.asm.new_label("select_end");

    for branch in hir.branches {
        match *branch {
            hir::ExprSelectBranch::Pat(pat) => {
                let label = cx.asm.new_label("select_branch");
                branches.try_push((label, pat))?;
            }
            hir::ExprSelectBranch::Default(def) => {
                if default_branch.is_some() {
                    return Err(compile::Error::new(span, ErrorKind::SelectMultipleDefaults));
                }

                let label = cx.asm.new_label("select_default");
                default_branch = Some((def, label));
            }
        }
    }

    for (_, branch) in &branches {
        expr(cx, &branch.expr, Needs::Value)?.apply(cx)?;
    }

    cx.asm.push(Inst::Select { len }, span)?;

    for (branch, (label, _)) in branches.iter().enumerate() {
        cx.asm.jump_if_branch(branch as i64, label, span)?;
    }

    if let Some((_, label)) = &default_branch {
        cx.asm.push(Inst::Pop, span)?;
        cx.asm.jump(label, span)?;
    }

    if !needs.value() {
        cx.asm.push(Inst::Pop, span)?;
    }

    cx.asm.jump(&end_label, span)?;

    for (label, branch) in branches {
        cx.asm.label(&label)?;

        let expected = cx.scopes.child(&branch.body)?;

        match branch.pat.kind {
            hir::PatKind::Path(&hir::PatPathKind::Ident(name)) => {
                cx.scopes.define(hir::Name::Str(name), &branch.pat)?;
            }
            hir::PatKind::Ignore => {
                cx.asm.push(Inst::Pop, &branch.body)?;
            }
            _ => {
                return Err(compile::Error::new(
                    branch.pat.span,
                    ErrorKind::UnsupportedSelectPattern,
                ));
            }
        }

        // Set up a new scope with the binding.
        expr(cx, &branch.body, needs)?.apply(cx)?;
        cx.clean_last_scope(&branch.body, expected, needs)?;
        cx.asm.jump(&end_label, span)?;
    }

    if let Some((branch, label)) = default_branch {
        cx.asm.label(&label)?;
        expr(cx, branch, needs)?.apply(cx)?;
    }

    cx.asm.label(&end_label)?;

    cx.contexts
        .pop()
        .ok_or("Missing parent context")
        .with_span(span)?;

    Ok(Asm::top(span))
}

/// Assemble a try expression.
#[instrument(span = span)]
fn expr_try<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::Expr<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    let clean = cx.scopes.total(span)?;
    let address = expr(cx, hir, Needs::Value)?.apply_targeted(cx)?;

    cx.asm.push(
        Inst::Try {
            address,
            clean,
            preserve: needs.value(),
        },
        span,
    )?;

    if let InstAddress::Top = address {
        cx.scopes.free(span, 1)?;
    }

    // Why no needs.value() check here to declare another anonymous
    // variable? Because when these assembling functions were initially
    // implemented it was decided that the caller that indicates
    // Needs::Value is responsible for declaring any anonymous variables.
    //
    // TODO: This should probably change!

    Ok(Asm::top(span))
}

/// Assemble a literal tuple.
#[instrument(span = span)]
fn expr_tuple<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::ExprSeq<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    macro_rules! tuple {
        ($variant:ident, $($var:ident),*) => {{
            let guard = cx.scopes.child(span)?;

            let mut it = hir.items.iter();

            $(
            let $var = it.next().ok_or_else(|| compile::Error::msg(span, "items ended unexpectedly"))?;
            let $var = expr(cx, $var, Needs::Value)?.apply_targeted(cx)?;
            )*

            cx.asm.push(
                Inst::$variant {
                    args: [$($var,)*],
                },
                span,
            )?;

            cx.scopes.pop(guard, span)?;
        }};
    }

    if hir.items.is_empty() {
        cx.asm.push(Inst::unit(), span)?;
    } else {
        match hir.items.len() {
            1 => tuple!(Tuple1, e1),
            2 => tuple!(Tuple2, e1, e2),
            3 => tuple!(Tuple3, e1, e2, e3),
            4 => tuple!(Tuple4, e1, e2, e3, e4),
            _ => {
                for e in hir.items {
                    expr(cx, e, Needs::Value)?.apply(cx)?;
                    cx.scopes.alloc(e)?;
                }

                cx.asm.push(
                    Inst::Tuple {
                        count: hir.items.len(),
                    },
                    span,
                )?;

                cx.scopes.free(span, hir.items.len())?;
            }
        }
    }

    if !needs.value() {
        cx.q.diagnostics
            .not_used(cx.source_id, span, cx.context())?;
        cx.asm.push(Inst::Pop, span)?;
    }

    Ok(Asm::top(span))
}

/// Assemble a unary expression.
#[instrument(span = span)]
fn expr_unary<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::ExprUnary<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    expr(cx, &hir.expr, Needs::Value)?.apply(cx)?;

    match hir.op {
        ast::UnOp::Not(..) => {
            cx.asm.push(Inst::Not, span)?;
        }
        ast::UnOp::Neg(..) => {
            cx.asm.push(Inst::Neg, span)?;
        }
        op => {
            return Err(compile::Error::new(
                span,
                ErrorKind::UnsupportedUnaryOp { op },
            ));
        }
    }

    // NB: we put it here to preserve the call in case it has side effects.
    // But if we don't need the value, then pop it from the stack.
    if !needs.value() {
        cx.asm.push(Inst::Pop, span)?;
    }

    Ok(Asm::top(span))
}

/// Assemble a literal vector.
#[instrument(span = span)]
fn expr_vec<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::ExprSeq<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    let count = hir.items.len();

    for e in hir.items {
        expr(cx, e, Needs::Value)?.apply(cx)?;
        cx.scopes.alloc(e)?;
    }

    cx.asm.push(Inst::Vec { count }, span)?;
    cx.scopes.free(span, hir.items.len())?;

    // Evaluate the expressions one by one, then pop them to cause any
    // side effects (without creating an object).
    if !needs.value() {
        cx.q.diagnostics
            .not_used(cx.source_id, span, cx.context())?;
        cx.asm.push(Inst::Pop, span)?;
    }

    Ok(Asm::top(span))
}

/// Assemble a while loop.
#[instrument(span = span)]
fn expr_loop<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::ExprLoop<'hir>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    let continue_label = cx.asm.new_label("while_continue");
    let then_label = cx.asm.new_label("while_then");
    let end_label = cx.asm.new_label("while_end");
    let break_label = cx.asm.new_label("while_break");

    let var_count = cx.scopes.total(span)?;

    cx.loops.push(Loop {
        label: hir.label,
        continue_label: continue_label.try_clone()?,
        continue_var_count: var_count,
        break_label: break_label.try_clone()?,
        break_var_count: var_count,
        needs,
        drop: None,
    })?;

    cx.asm.label(&continue_label)?;

    let expected = if let Some(hir) = hir.condition {
        let then_scope = condition(cx, hir, &then_label)?;
        let expected = cx.scopes.push(then_scope)?;

        cx.asm.jump(&end_label, span)?;
        cx.asm.label(&then_label)?;
        Some(expected)
    } else {
        None
    };

    block(cx, &hir.body, Needs::None)?.apply(cx)?;

    if let Some(expected) = expected {
        cx.clean_last_scope(span, expected, Needs::None)?;
    }

    cx.asm.jump(&continue_label, span)?;
    cx.asm.label(&end_label)?;

    if needs.value() {
        cx.asm.push(Inst::unit(), span)?;
    }

    // NB: breaks produce their own value / perform their own cleanup.
    cx.asm.label(&break_label)?;
    cx.loops.pop();
    Ok(Asm::top(span))
}

/// Assemble a `yield` expression.
#[instrument(span = span)]
fn expr_yield<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: Option<&'hir hir::Expr<'hir>>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    if let Some(e) = hir {
        expr(cx, e, Needs::Value)?.apply(cx)?;
        cx.asm.push(Inst::Yield, span)?;
    } else {
        cx.asm.push(Inst::YieldUnit, span)?;
    }

    if !needs.value() {
        cx.asm.push(Inst::Pop, span)?;
    }

    Ok(Asm::top(span))
}

/// Assemble a literal value.
#[instrument(span = span)]
fn lit<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: hir::Lit<'_>,
    span: &dyn Spanned,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    // Elide the entire literal if it's not needed.
    if !needs.value() {
        cx.q.diagnostics
            .not_used(cx.source_id, span, cx.context())?;
        return Ok(Asm::top(span));
    }

    match hir {
        hir::Lit::Bool(boolean) => {
            cx.asm.push(Inst::bool(boolean), span)?;
        }
        hir::Lit::Byte(byte) => {
            cx.asm.push(Inst::byte(byte), span)?;
        }
        hir::Lit::Char(char) => {
            cx.asm.push(Inst::char(char), span)?;
        }
        hir::Lit::Integer(integer) => {
            cx.asm.push(Inst::integer(integer), span)?;
        }
        hir::Lit::Float(float) => {
            cx.asm.push(Inst::float(float), span)?;
        }
        hir::Lit::Str(string) => {
            let slot = cx.q.unit.new_static_string(span, string)?;
            cx.asm.push(Inst::String { slot }, span)?;
        }
        hir::Lit::ByteStr(bytes) => {
            let slot = cx.q.unit.new_static_bytes(span, bytes)?;
            cx.asm.push(Inst::Bytes { slot }, span)?;
        }
    };

    Ok(Asm::top(span))
}

/// Assemble a local expression.
#[instrument(span = hir)]
fn local<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::Local<'hir>,
    needs: Needs,
) -> compile::Result<Asm<'hir>> {
    let load = |cx: &mut Ctxt<'_, 'hir, '_>, needs: Needs| {
        // NB: assignments "move" the value being assigned.
        expr(cx, &hir.expr, needs)?.apply(cx)?;
        Ok(())
    };

    let false_label = cx.asm.new_label("let_panic");

    if pat(cx, &hir.pat, &false_label, &load)? {
        cx.q.diagnostics
            .let_pattern_might_panic(cx.source_id, hir, cx.context())?;

        let ok_label = cx.asm.new_label("let_ok");
        cx.asm.jump(&ok_label, hir)?;
        cx.asm.label(&false_label)?;
        cx.asm.push(
            Inst::Panic {
                reason: PanicReason::UnmatchedPattern,
            },
            hir,
        )?;

        cx.asm.label(&ok_label)?;
    }

    // If a value is needed for a let expression, it is evaluated as a unit.
    if needs.value() {
        cx.asm.push(Inst::unit(), hir)?;
    }

    Ok(Asm::top(hir))
}
