use core::fmt;

use crate::alloc::prelude::*;
use crate::alloc::{try_format, Vec};
use crate::ast::{self, Span, Spanned};
use crate::compile::ir;
use crate::compile::v1::{Loop, Loops, Scope, Scopes};
use crate::compile::{self, Assembly, ErrorKind, ItemId, ModId, Options, WithSpan};
use crate::hir;
use crate::query::{ConstFn, Query, Used};
use crate::runtime::{
    ConstValue, Inst, InstAddress, InstAssignOp, InstOp, InstRange, InstTarget, InstValue,
    InstVariant, Label, Output, PanicReason, Protocol, TypeCheck,
};
use crate::{Hash, SourceId};

use super::Needs;

use rune_macros::instrument;

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
        Ok(crate::from_value(value).with_span(span)?)
    }
}

struct Asm<'hir> {
    span: &'hir dyn Spanned,
}

impl<'hir> Asm<'hir> {
    /// Construct an assembly result that leaves the value on the top of the
    /// stack.
    fn new(span: &'hir dyn Spanned) -> Self {
        Self { span }
    }
}

impl fmt::Debug for Asm<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Asm")
            .field("span", &self.span.span())
            .finish()
    }
}

/// Assemble a function from an [hir::ItemFn<'_>].
#[instrument(span = hir)]
pub(crate) fn fn_from_item_fn<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::ItemFn<'hir>,
    instance_fn: bool,
) -> compile::Result<()> {
    let mut first = true;

    let arguments = cx.scopes.linear(hir, hir.args.len())?;

    for (arg, needs) in hir.args.iter().zip(&arguments) {
        match arg {
            hir::FnArg::SelfValue(span) => {
                if !instance_fn || !first {
                    return Err(compile::Error::new(*span, ErrorKind::UnsupportedSelf));
                }

                cx.scopes
                    .define(span, hir::Name::SelfValue, needs.addr()?)?;
            }
            hir::FnArg::Pat(pat) => {
                pat_binding_with_addr(cx, pat, needs.addr()?)?;
            }
        }

        first = false;
    }

    if hir.body.value.is_some() {
        return_(cx, hir, &hir.body, block)?;
    } else {
        if !hir.body.statements.is_empty() {
            block(cx, &hir.body, &mut Needs::none(&hir.body))?;
        }

        cx.asm.push(Inst::ReturnUnit, hir)?;
    }

    cx.scopes.free_linear(arguments)?;
    cx.scopes.pop_last(hir)?;
    Ok(())
}

/// Assemble an async block.
#[instrument(span = hir.block.span)]
pub(crate) fn async_block_secondary<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::AsyncBlock<'hir>,
) -> compile::Result<()> {
    let linear = cx.scopes.linear(&hir.block, hir.captures.len())?;

    for (name, needs) in hir.captures.iter().copied().zip(&linear) {
        cx.scopes.define(&hir.block, name, needs.addr()?)?;
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
    let environment = cx.scopes.linear(span, hir.captures.len())?;
    let arguments = cx.scopes.linear(span, hir.args.len())?;

    if !hir.captures.is_empty() {
        cx.asm.push(
            Inst::PushEnvironment {
                addr: environment.addr(),
                count: hir.captures.len(),
                out: environment.addr().output(),
            },
            span,
        )?;

        for (capture, needs) in hir.captures.iter().copied().zip(&environment) {
            cx.scopes.define(span, capture, needs.addr()?)?;
        }
    }

    for (arg, needs) in hir.args.iter().zip(&arguments) {
        match arg {
            hir::FnArg::SelfValue(..) => {
                return Err(compile::Error::new(arg, ErrorKind::UnsupportedSelf))
            }
            hir::FnArg::Pat(pat) => {
                pat_binding_with_addr(cx, pat, needs.addr()?)?;
            }
        }
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
    asm: impl FnOnce(&mut Ctxt<'_, 'hir, '_>, T, &mut Needs<'_>) -> compile::Result<Asm<'hir>>,
) -> compile::Result<()> {
    let mut needs = Needs::alloc(cx, span)?;
    asm(cx, hir, &mut needs)?;

    cx.asm.push(
        Inst::Return {
            addr: needs.addr()?,
        },
        span,
    )?;

    cx.scopes.free(needs)?;
    Ok(())
}

/// Compile a pattern with bindings based on the given offset.
#[instrument(span = hir)]
fn pat_binding_with_addr<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::PatBinding<'hir>,
    addr: InstAddress,
) -> compile::Result<()> {
    pat_with_addr(cx, &hir.pat, addr)
}

/// Compile a pattern based on the given offset.
#[instrument(span = hir)]
fn pat_with_addr<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::Pat<'hir>,
    addr: InstAddress,
) -> compile::Result<()> {
    let load = |cx: &mut Ctxt<'_, 'hir, '_>, needs: &mut Needs<'_>| {
        needs.assign_addr(cx, addr)?;
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

/// Encode a pattern from a known set of bindings.
///
/// Returns a boolean indicating if the label was used.
#[instrument(span = hir)]
fn pat_binding<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::PatBinding<'hir>,
    false_label: &Label,
    load: &dyn Fn(&mut Ctxt<'_, 'hir, '_>, &mut Needs<'_>) -> compile::Result<()>,
) -> compile::Result<bool> {
    pat(cx, &hir.pat, false_label, load)
}

/// Encode a pattern.
///
/// Returns a boolean indicating if the label was used.
#[instrument(span = hir)]
fn pat<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::Pat<'hir>,
    false_label: &Label,
    load: &dyn Fn(&mut Ctxt<'_, 'hir, '_>, &mut Needs<'_>) -> compile::Result<()>,
) -> compile::Result<bool> {
    let span = hir;

    match hir.kind {
        hir::PatKind::Ignore => {
            // ignore binding, but might still have side effects, so must
            // call the load generator.
            load(cx, &mut Needs::none(hir))?;
            Ok(false)
        }
        hir::PatKind::Path(kind) => match *kind {
            hir::PatPathKind::Kind(kind) => {
                let mut needs = Needs::alloc(cx, hir)?;

                load(cx, &mut needs)?;

                cx.asm.push(
                    pat_sequence_kind_to_inst(*kind, needs.addr()?, needs.output()?),
                    hir,
                )?;

                cx.asm.jump_if_not(needs.addr()?, false_label, hir)?;
                Ok(true)
            }
            hir::PatPathKind::Ident(name) => {
                let mut needs = Needs::alloc(cx, hir)?;
                load(cx, &mut needs)?;
                let addr = needs.addr()?;
                cx.scopes.define(hir, hir::Name::Str(name), addr)?;
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
    load: &dyn Fn(&mut Ctxt<'_, 'hir, '_>, &mut Needs<'_>) -> compile::Result<()>,
) -> compile::Result<bool> {
    let mut needs = Needs::alloc(cx, hir)?;

    load(cx, &mut needs)?;

    let addr = needs.addr()?;

    let Some(inst) = pat_lit_inst(cx, hir, addr)? else {
        return Err(compile::Error::new(hir, ErrorKind::UnsupportedPatternExpr));
    };

    cx.asm.push(inst, hir)?;
    cx.asm.jump_if_not(addr, false_label, hir)?;
    Ok(true)
}

#[instrument(span = hir)]
fn pat_lit_inst<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::Expr<'_>,
    addr: InstAddress,
) -> compile::Result<Option<Inst>> {
    let hir::ExprKind::Lit(lit) = hir.kind else {
        return Ok(None);
    };

    let out = addr.output();

    let inst = match lit {
        hir::Lit::Byte(value) => Inst::EqByte { addr, value, out },
        hir::Lit::Char(value) => Inst::EqChar { addr, value, out },
        hir::Lit::Str(string) => Inst::EqString {
            addr,
            slot: cx.q.unit.new_static_string(hir, string)?,
            out,
        },
        hir::Lit::ByteStr(bytes) => Inst::EqBytes {
            addr,
            slot: cx.q.unit.new_static_bytes(hir, bytes)?,
            out,
        },
        hir::Lit::Integer(value) => Inst::EqInteger { addr, value, out },
        hir::Lit::Bool(value) => Inst::EqBool { addr, value, out },
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
) -> compile::Result<Scope<'hir>> {
    match *condition {
        hir::Condition::Expr(e) => {
            let guard = cx.scopes.child(e)?;
            let mut addr = cx.scopes.alloc(e)?;
            expr(cx, e, &mut addr)?;
            cx.asm.jump_if(addr.addr()?, then_label, e)?;
            Ok(cx.scopes.pop(e, guard)?)
        }
        hir::Condition::ExprLet(expr_let) => {
            let span = expr_let;

            let false_label = cx.asm.new_label("if_condition_false");

            let expected = cx.scopes.child(span)?;

            let load = |cx: &mut Ctxt<'_, 'hir, '_>, needs: &mut Needs<'_>| {
                expr(cx, &expr_let.expr, needs)?;
                Ok(())
            };

            if pat_binding(cx, &expr_let.pat, &false_label, &load)? {
                cx.asm.jump(then_label, span)?;
                cx.asm.label(&false_label)?;
            } else {
                cx.asm.jump(then_label, span)?;
            };

            Ok(cx.scopes.pop(span, expected)?)
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
    load: &dyn Fn(&mut Ctxt<'_, 'hir, '_>, &mut Needs<'_>) -> compile::Result<()>,
) -> compile::Result<()> {
    let mut needs = Needs::alloc(cx, span)?;

    load(cx, &mut needs)?;

    if matches!(
        hir.kind,
        hir::PatSequenceKind::Anonymous {
            type_check: TypeCheck::Tuple,
            count: 0,
            is_open: false
        }
    ) {
        let addr = needs.addr()?;

        cx.asm.push(
            Inst::IsUnit {
                addr,
                out: needs.output()?,
            },
            span,
        )?;
        cx.asm.jump_if_not(addr, false_label, span)?;
        return Ok(());
    }

    let cond = cx.scopes.alloc(span)?;
    let inst = pat_sequence_kind_to_inst(hir.kind, needs.addr()?, cond.output()?);
    cx.asm.push(inst, span)?;
    cx.asm.jump_if_not(cond.addr()?, false_label, span)?;
    cx.scopes.free(cond)?;

    for (index, p) in hir.items.iter().enumerate() {
        let addr = needs.addr()?;

        let load = move |cx: &mut Ctxt<'_, 'hir, '_>, n: &mut Needs<'_>| {
            cx.asm.push(
                Inst::TupleIndexGetAt {
                    addr,
                    index,
                    out: n.output()?,
                },
                p,
            )?;
            Ok(())
        };

        pat(cx, p, false_label, &load)?;
    }

    Ok(())
}

fn pat_sequence_kind_to_inst(kind: hir::PatSequenceKind, addr: InstAddress, out: Output) -> Inst {
    match kind {
        hir::PatSequenceKind::Type { hash } => Inst::MatchType { hash, addr, out },
        hir::PatSequenceKind::BuiltInVariant { type_check } => Inst::MatchBuiltIn {
            type_check,
            addr,
            out,
        },
        hir::PatSequenceKind::Variant {
            variant_hash,
            enum_hash,
            index,
        } => Inst::MatchVariant {
            variant_hash,
            enum_hash,
            index,
            addr,
            out,
        },
        hir::PatSequenceKind::Anonymous {
            type_check,
            count,
            is_open,
        } => Inst::MatchSequence {
            type_check,
            len: count,
            exact: !is_open,
            addr,
            out,
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
    load: &dyn Fn(&mut Ctxt<'_, 'hir, '_>, &mut Needs<'_>) -> compile::Result<()>,
) -> compile::Result<()> {
    let mut needs = Needs::alloc(cx, span)?;

    load(cx, &mut needs)?;

    let mut string_slots = Vec::new();

    for binding in hir.bindings {
        string_slots.try_push(cx.q.unit.new_static_string(span, binding.key())?)?;
    }

    let cond = cx.scopes.alloc(span)?;

    let inst = match hir.kind {
        hir::PatSequenceKind::Type { hash } => Inst::MatchType {
            hash,
            addr: needs.addr()?,
            out: cond.output()?,
        },
        hir::PatSequenceKind::BuiltInVariant { type_check } => Inst::MatchBuiltIn {
            type_check,
            addr: needs.addr()?,
            out: cond.output()?,
        },
        hir::PatSequenceKind::Variant {
            variant_hash,
            enum_hash,
            index,
        } => Inst::MatchVariant {
            variant_hash,
            enum_hash,
            index,
            addr: needs.addr()?,
            out: cond.output()?,
        },
        hir::PatSequenceKind::Anonymous { is_open, .. } => {
            let keys =
                cx.q.unit
                    .new_static_object_keys_iter(span, hir.bindings.iter().map(|b| b.key()))?;

            Inst::MatchObject {
                slot: keys,
                exact: !is_open,
                addr: needs.addr()?,
                out: cond.output()?,
            }
        }
    };

    // Copy the temporary and check that its length matches the pattern and
    // that it is indeed a vector.
    cx.asm.push(inst, span)?;
    cx.asm.jump_if_not(cond.addr()?, false_label, span)?;

    for (binding, slot) in hir.bindings.iter().zip(string_slots) {
        match *binding {
            hir::Binding::Binding(span, _, p) => {
                let load = |cx: &mut Ctxt<'_, 'hir, '_>, needs: &mut Needs<'_>| {
                    cx.asm.push(
                        Inst::ObjectIndexGetAt {
                            addr: needs.addr()?,
                            slot,
                            out: needs.output()?,
                        },
                        &span,
                    )?;
                    Ok(())
                };

                pat(cx, p, false_label, &load)?;
            }
            hir::Binding::Ident(span, name) => {
                let addr = cx.scopes.alloc(&span)?;

                cx.scopes
                    .define(binding, hir::Name::Str(name), addr.addr()?)?;

                cx.asm.push(
                    Inst::ObjectIndexGetAt {
                        addr: needs.addr()?,
                        slot,
                        out: addr.output()?,
                    },
                    &span,
                )?;
            }
        }
    }

    Ok(())
}

/// Call a block.
#[instrument(span = hir)]
fn block<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::Block<'hir>,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    cx.contexts.try_push(hir.span())?;
    let scopes_count = cx.scopes.child(hir)?;

    for stmt in hir.statements {
        match stmt {
            hir::Stmt::Local(hir) => {
                local(cx, hir, &mut Needs::none(hir))?;
            }
            hir::Stmt::Expr(hir) => {
                expr(cx, hir, &mut Needs::none(hir))?;
            }
        }
    }

    if let Some(e) = hir.value {
        expr(cx, e, needs)?;
    } else if needs.value() {
        let out = needs.alloc_output(&mut cx.scopes)?;
        cx.asm.push(Inst::unit(out), hir)?;
    }

    cx.scopes.pop(hir, scopes_count)?;

    cx.contexts
        .pop()
        .ok_or("Missing parent context")
        .with_span(hir)?;

    Ok(Asm::new(hir))
}

/// Assemble #[builtin] format_args!(...) macro.
#[instrument(span = format)]
fn builtin_format<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    format: &'hir hir::BuiltInFormat<'hir>,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    use crate::runtime::format;

    let fill = format.fill.unwrap_or(' ');
    let align = format.align.unwrap_or_default();
    let flags = format.flags.unwrap_or_default();
    let width = format.width;
    let precision = format.precision;
    let format_type = format.format_type.unwrap_or_default();

    let spec = format::FormatSpec::new(flags, fill, align, width, precision, format_type);

    expr(cx, &format.value, needs)?;

    if let Some(addr) = needs.try_alloc_addr(&mut cx.scopes)? {
        cx.asm.push(
            Inst::Format {
                addr,
                spec,
                out: addr.output(),
            },
            format,
        )?;
    }

    Ok(Asm::new(format))
}

/// Assemble #[builtin] template!(...) macro.
#[instrument(span = template)]
fn builtin_template<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    template: &'hir hir::BuiltInTemplate<'hir>,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    let span = template;

    let expected = cx.scopes.child(span)?;
    let mut size_hint = 0;
    let mut expansions = 0;

    let mut linear = cx.scopes.linear(template, template.exprs.len())?;

    for (hir, addr) in template.exprs.iter().zip(&mut linear) {
        if let hir::ExprKind::Lit(hir::Lit::Str(s)) = hir.kind {
            if needs.value() {
                size_hint += s.len();
                let slot = cx.q.unit.new_static_string(span, s)?;
                cx.asm.push(
                    Inst::String {
                        slot,
                        out: addr.output()?,
                    },
                    span,
                )?;
            }

            continue;
        }

        expansions += 1;
        expr(cx, hir, addr)?;
    }

    if template.from_literal && expansions == 0 {
        cx.q.diagnostics
            .template_without_expansions(cx.source_id, span, cx.context())?;
    }

    if needs.value() {
        cx.asm.push(
            Inst::StringConcat {
                addr: linear.addr(),
                len: template.exprs.len(),
                size_hint,
                out: needs.alloc_output(&mut cx.scopes)?,
            },
            span,
        )?;
    }

    let _ = cx.scopes.pop(span, expected)?;
    Ok(Asm::new(span))
}

/// Assemble a constant value.
#[instrument(span = span)]
fn const_<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    value: &ConstValue,
    span: &dyn Spanned,
    needs: &mut Needs<'_>,
) -> compile::Result<()> {
    let Some(addr) = needs.try_alloc_addr(&mut cx.scopes)? else {
        cx.q.diagnostics
            .not_used(cx.source_id, span, cx.context())?;
        return Ok(());
    };

    let out = addr.output();

    match *value {
        ConstValue::EmptyTuple => {
            cx.asm.push(Inst::unit(out), span)?;
        }
        ConstValue::Byte(v) => {
            cx.asm.push(Inst::byte(v, out), span)?;
        }
        ConstValue::Char(v) => {
            cx.asm.push(Inst::char(v, out), span)?;
        }
        ConstValue::Integer(v) => {
            cx.asm.push(Inst::integer(v, out), span)?;
        }
        ConstValue::Float(v) => {
            cx.asm.push(Inst::float(v, out), span)?;
        }
        ConstValue::Bool(v) => {
            cx.asm.push(Inst::bool(v, out), span)?;
        }
        ConstValue::String(ref s) => {
            let slot = cx.q.unit.new_static_string(span, s)?;
            cx.asm.push(Inst::String { slot, out: out }, span)?;
        }
        ConstValue::Bytes(ref b) => {
            let slot = cx.q.unit.new_static_bytes(span, b)?;
            cx.asm.push(Inst::Bytes { slot, out: out }, span)?;
        }
        ConstValue::Option(ref option) => match option {
            Some(value) => {
                const_(cx, value, span, &mut Needs::with_addr(span, addr))?;
                cx.asm.push(
                    Inst::Variant {
                        variant: InstVariant::Some,
                        addr,
                        out,
                    },
                    span,
                )?;
            }
            None => {
                cx.asm.push(
                    Inst::Variant {
                        variant: InstVariant::None,
                        addr,
                        out,
                    },
                    span,
                )?;
            }
        },
        ConstValue::Vec(ref vec) => {
            let mut linear = cx.scopes.linear(span, vec.len())?;

            for (value, needs) in vec.iter().zip(&mut linear) {
                const_(cx, value, span, needs)?;
            }

            cx.asm.push(
                Inst::Vec {
                    addr: linear.addr(),
                    count: vec.len(),
                    out,
                },
                span,
            )?;

            cx.scopes.free_linear(linear)?;
        }
        ConstValue::Tuple(ref tuple) => {
            let mut linear = cx.scopes.linear(span, tuple.len())?;

            for (value, needs) in tuple.iter().zip(&mut linear) {
                const_(cx, value, span, needs)?;
            }

            cx.asm.push(
                Inst::Tuple {
                    addr: linear.addr(),
                    count: tuple.len(),
                    out,
                },
                span,
            )?;

            cx.scopes.free_linear(linear)?;
        }
        ConstValue::Object(ref object) => {
            let mut linear = cx.scopes.linear(span, object.len())?;

            let mut entries = object.iter().try_collect::<Vec<_>>()?;
            entries.sort_by_key(|k| k.0);

            for ((_, value), needs) in entries.iter().copied().zip(&mut linear) {
                const_(cx, value, span, needs)?;
            }

            let slot =
                cx.q.unit
                    .new_static_object_keys_iter(span, entries.iter().map(|e| e.0))?;

            cx.asm.push(
                Inst::Object {
                    addr: linear.addr(),
                    slot,
                    out,
                },
                span,
            )?;

            cx.scopes.free_linear(linear)?;
        }
    }

    Ok(())
}

/// Assemble an expression.
#[instrument(span = hir)]
fn expr<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::Expr<'hir>,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    let span = hir;

    let asm = match hir.kind {
        hir::ExprKind::Variable(name) => {
            let var = cx.scopes.get(&mut cx.q, span, name)?;
            needs.assign_addr(cx, var.addr)?;
            Asm::new(span)
        }
        hir::ExprKind::Type(ty) => {
            cx.asm.push(
                Inst::Store {
                    value: InstValue::Type(ty),
                    out: needs.alloc_output(&mut cx.scopes)?,
                },
                span,
            )?;

            Asm::new(span)
        }
        hir::ExprKind::Fn(hash) => {
            cx.asm.push(
                Inst::LoadFn {
                    hash,
                    out: needs.alloc_output(&mut cx.scopes)?,
                },
                span,
            )?;

            Asm::new(span)
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
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    let supported = match hir.lhs.kind {
        // <var> = <value>
        hir::ExprKind::Variable(name) => {
            let var = cx.scopes.get(&mut cx.q, span, name)?;
            expr(cx, &hir.rhs, &mut Needs::with_addr(span, var.addr))?;
            true
        }
        // <expr>.<field> = <value>
        hir::ExprKind::FieldAccess(field_access) => {
            // field assignment
            match field_access.expr_field {
                hir::ExprField::Ident(ident) => {
                    let slot = cx.q.unit.new_static_string(span, ident)?;

                    let mut target = Needs::alloc(cx, span)?;
                    let mut value = Needs::alloc(cx, &hir.rhs)?;

                    expr(cx, &field_access.expr, &mut target)?;
                    expr(cx, &hir.rhs, &mut value)?;

                    cx.asm.push(
                        Inst::ObjectIndexSet {
                            target: target.addr()?,
                            slot,
                            value: value.addr()?,
                        },
                        span,
                    )?;

                    cx.scopes.free(target)?;
                    cx.scopes.free(value)?;
                    true
                }
                hir::ExprField::Index(index) => {
                    let mut target = cx.scopes.alloc(span)?;
                    let mut value = cx.scopes.alloc(&hir.rhs)?;

                    expr(cx, &field_access.expr, &mut target)?;
                    expr(cx, &hir.rhs, &mut value)?;

                    cx.asm.push(
                        Inst::TupleIndexSet {
                            target: target.addr()?,
                            index,
                            value: value.addr()?,
                        },
                        span,
                    )?;

                    cx.scopes.free(target)?;
                    cx.scopes.free(value)?;
                    true
                }
                _ => {
                    return Err(compile::Error::new(span, ErrorKind::BadFieldAccess));
                }
            }
        }
        hir::ExprKind::Index(expr_index_get) => {
            let mut target = cx.scopes.alloc(span)?;
            let mut index = cx.scopes.alloc(span)?;
            let mut value = cx.scopes.alloc(span)?;

            expr(cx, &expr_index_get.target, &mut target)?;
            expr(cx, &expr_index_get.index, &mut index)?;
            expr(cx, &hir.rhs, &mut value)?;

            cx.asm.push(
                Inst::IndexSet {
                    target: target.addr()?,
                    index: index.addr()?,
                    value: value.addr()?,
                },
                span,
            )?;

            cx.scopes.free(value)?;
            cx.scopes.free(index)?;
            cx.scopes.free(target)?;
            true
        }
        _ => false,
    };

    if !supported {
        return Err(compile::Error::new(span, ErrorKind::UnsupportedAssignExpr));
    }

    if needs.value() {
        cx.asm
            .push(Inst::unit(needs.alloc_output(&mut cx.scopes)?), span)?;
    }

    Ok(Asm::new(span))
}

/// Assemble an `.await` expression.
#[instrument(span = hir)]
fn expr_await<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::Expr<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    let mut addr = cx.scopes.alloc(span)?;
    expr(cx, hir, &mut addr)?;

    cx.asm.push(
        Inst::Await {
            addr: addr.addr()?,
            out: needs.alloc_output(&mut cx.scopes)?,
        },
        span,
    )?;

    cx.scopes.free(addr)?;
    Ok(Asm::new(span))
}

/// Assemble a binary expression.
#[instrument(span = span)]
fn expr_binary<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::ExprBinary<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    // Special expressions which operates on the stack in special ways.
    if hir.op.is_assign() {
        compile_assign_binop(cx, &hir.lhs, &hir.rhs, &hir.op, span, needs)?;
        return Ok(Asm::new(span));
    }

    if hir.op.is_conditional() {
        compile_conditional_binop(cx, &hir.lhs, &hir.rhs, &hir.op, span, needs)?;
        return Ok(Asm::new(span));
    }

    let guard = cx.scopes.child(span)?;

    let mut a = Needs::alloc_in(guard, span)?;
    let mut b = Needs::alloc_in(guard, span)?;

    // NB: need to declare these as anonymous local variables so that they
    // get cleaned up in case there is an early break (return, try, ...).
    expr(cx, &hir.lhs, &mut a)?;
    expr(cx, &hir.rhs, &mut b)?;

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

    cx.asm.push(
        Inst::Op {
            op,
            a: a.addr()?,
            b: b.addr()?,
            out: needs.alloc_output(&mut cx.scopes)?,
        },
        span,
    )?;

    cx.scopes.free(a)?;
    cx.scopes.free(b)?;
    cx.scopes.pop(span, guard)?;
    return Ok(Asm::new(span));

    fn compile_conditional_binop<'hir>(
        cx: &mut Ctxt<'_, 'hir, '_>,
        lhs: &'hir hir::Expr<'hir>,
        rhs: &'hir hir::Expr<'hir>,
        bin_op: &ast::BinOp,
        span: &dyn Spanned,
        needs: &mut Needs<'_>,
    ) -> compile::Result<()> {
        let end_label = cx.asm.new_label("conditional_end");

        expr(cx, lhs, needs)?;

        match bin_op {
            ast::BinOp::And(..) => {
                cx.asm.jump_if_not(needs.addr()?, &end_label, lhs)?;
            }
            ast::BinOp::Or(..) => {
                cx.asm.jump_if(needs.addr()?, &end_label, lhs)?;
            }
            op => {
                return Err(compile::Error::new(
                    span,
                    ErrorKind::UnsupportedBinaryOp { op: *op },
                ));
            }
        }

        expr(cx, rhs, needs)?;
        cx.asm.label(&end_label)?;
        Ok(())
    }

    fn compile_assign_binop<'hir>(
        cx: &mut Ctxt<'_, 'hir, '_>,
        lhs: &'hir hir::Expr<'hir>,
        rhs: &'hir hir::Expr<'hir>,
        bin_op: &ast::BinOp,
        span: &dyn Spanned,
        needs: &mut Needs<'_>,
    ) -> compile::Result<()> {
        let supported = match lhs.kind {
            // <var> <op> <expr>
            hir::ExprKind::Variable(name) => {
                let var = cx.scopes.get(&mut cx.q, lhs, name)?;
                Some(InstTarget::Address(var.addr))
            }
            // <expr>.<field> <op> <value>
            hir::ExprKind::FieldAccess(field_access) => {
                let mut field = cx.scopes.alloc(&field_access.expr)?;
                expr(cx, &field_access.expr, &mut field)?;

                // field assignment
                match field_access.expr_field {
                    hir::ExprField::Index(index) => {
                        Some(InstTarget::TupleField(field.addr()?, index))
                    }
                    hir::ExprField::Ident(ident) => {
                        let n = cx.q.unit.new_static_string(&field_access.expr, ident)?;
                        Some(InstTarget::Field(field.addr()?, n))
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

        let mut value = cx.scopes.alloc(rhs)?;
        expr(cx, rhs, &mut value)?;

        cx.asm.push(
            Inst::Assign {
                target,
                op,
                value: value.addr()?,
            },
            span,
        )?;

        if needs.value() {
            cx.asm
                .push(Inst::unit(needs.alloc_output(&mut cx.scopes)?), span)?;
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
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    let linear = cx.scopes.linear(span, hir.captures.len())?;

    for (capture, needs) in hir.captures.iter().copied().zip(&linear) {
        let out = needs.output()?;

        if hir.do_move {
            let var = cx.scopes.take(&mut cx.q, span, capture)?;
            var.move_(cx.asm, span, Some(&"capture"), out)?;
        } else {
            let var = cx.scopes.get(&mut cx.q, span, capture)?;
            var.copy(cx, span, Some(&"capture"), out)?;
        }
    }

    cx.asm.push_with_comment(
        Inst::Call {
            hash: hir.hash,
            addr: linear.addr(),
            args: hir.captures.len(),
            out: needs.alloc_output(&mut cx.scopes)?,
        },
        span,
        &"async block",
    )?;

    cx.scopes.free_linear(linear)?;
    Ok(Asm::new(span))
}

/// Assemble a constant item.
#[instrument(span = span)]
fn const_item<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hash: Hash,
    span: &'hir dyn Spanned,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    let Some(const_value) = cx.q.get_const_value(hash) else {
        return Err(compile::Error::msg(
            span,
            try_format!("Missing constant value for hash {hash}"),
        ));
    };

    let const_value = const_value.try_clone().with_span(span)?;
    const_(cx, &const_value, span, needs)?;
    Ok(Asm::new(span))
}

/// Assemble a break expression.
///
/// NB: loops are expected to produce a value at the end of their expression.
#[instrument(span = span)]
fn expr_break<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::ExprBreak<'hir>,
    span: &'hir dyn Spanned,
    _: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    let Some(current_loop) = cx.loops.last().try_cloned()? else {
        return Err(compile::Error::new(span, ErrorKind::BreakOutsideOfLoop));
    };

    let (last_loop, to_drop, has_value) = match (hir.label, hir.expr) {
        (None, Some(e)) => {
            let mut needs = match current_loop.output.as_addr() {
                Some(addr) => Needs::with_addr(e, addr),
                None => Needs::none(e),
            };

            expr(cx, e, &mut needs)?;
            let to_drop = current_loop.drop.into_iter().try_collect()?;
            (current_loop, to_drop, true)
        }
        (Some(label), None) => {
            let (last_loop, to_drop) = cx.loops.walk_until_label(label, span)?;
            (last_loop.try_clone()?, to_drop, false)
        }
        (Some(label), Some(e)) => {
            let mut needs = match current_loop.output.as_addr() {
                Some(addr) => Needs::with_addr(span, addr),
                None => Needs::none(span),
            };

            expr(cx, e, &mut needs)?;
            let (last_loop, to_drop) = cx.loops.walk_until_label(label, span)?;
            (last_loop.try_clone()?, to_drop, true)
        }
        (None, None) => {
            let to_drop = current_loop.drop.into_iter().try_collect()?;
            (current_loop, to_drop, false)
        }
    };

    // Drop loop temporaries. Typically an iterator.
    for addr in to_drop {
        cx.asm.push(Inst::Drop { addr }, span)?;
    }

    if let Some(addr) = last_loop.output.as_addr() {
        if !has_value {
            cx.asm.push(Inst::unit(addr.output()), span)?;
        }
    }

    cx.asm.jump(&last_loop.break_label, span)?;
    Ok(Asm::new(span))
}

/// Assemble a call expression.
#[instrument(span = span)]
fn expr_call<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::ExprCall<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    let args = hir.args.len();

    match hir.call {
        hir::Call::Var { name, .. } => {
            let var = cx.scopes.get(&mut cx.q, span, name)?;
            let mut linear = cx.scopes.linear(span, args)?;

            for (e, needs) in hir.args.iter().zip(&mut linear) {
                expr(cx, e, needs)?;
            }

            cx.asm.push(
                Inst::CallFn {
                    function: var.addr,
                    addr: linear.addr(),
                    args,
                    out: needs.alloc_output(&mut cx.scopes)?,
                },
                span,
            )?;

            cx.scopes.free_linear(linear)?;
        }
        hir::Call::Associated { target, hash } => {
            let mut linear = cx.scopes.linear(span, args + 1)?;

            for (e, needs) in [target].into_iter().chain(hir.args.iter()).zip(&mut linear) {
                expr(cx, e, needs)?;
            }

            cx.asm.push(
                Inst::CallAssociated {
                    hash,
                    addr: linear.addr(),
                    args,
                    out: needs.alloc_output(&mut cx.scopes)?,
                },
                span,
            )?;

            cx.scopes.free_linear(linear)?;
        }
        hir::Call::Meta { hash } => {
            let mut linear = cx.scopes.linear(span, args)?;

            for (e, needs) in hir.args.iter().zip(&mut linear) {
                expr(cx, e, needs)?;
            }

            cx.asm.push(
                Inst::Call {
                    hash,
                    addr: linear.addr(),
                    args,
                    out: needs.alloc_output(&mut cx.scopes)?,
                },
                span,
            )?;

            cx.scopes.free_linear(linear)?;
        }
        hir::Call::Expr { expr: e } => {
            let mut function = cx.scopes.alloc(span)?;
            expr(cx, e, &mut function)?;

            let mut linear = cx.scopes.linear(span, args)?;

            for (e, needs) in hir.args.iter().zip(&mut linear) {
                expr(cx, e, needs)?;
            }

            cx.asm.push(
                Inst::CallFn {
                    function: function.addr()?,
                    addr: linear.addr(),
                    args,
                    out: needs.alloc_output(&mut cx.scopes)?,
                },
                span,
            )?;

            cx.scopes.free_linear(linear)?;
            cx.scopes.free(function)?;
        }
        hir::Call::ConstFn {
            from_module,
            from_item,
            id,
        } => {
            let const_fn = cx.q.const_fn_for(id).with_span(span)?;
            let value = cx.call_const_fn(span, from_module, from_item, &const_fn, hir.args)?;
            const_(cx, &value, span, needs)?;
        }
    }

    Ok(Asm::new(span))
}

/// Assemble a closure expression.
#[instrument(span = span)]
fn expr_call_closure<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::ExprCallClosure<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    if !needs.value() {
        cx.q.diagnostics
            .not_used(cx.source_id, span, cx.context())?;
        return Ok(Asm::new(span));
    }

    tracing::trace!(?hir.captures, "assemble call closure");

    let linear = cx.scopes.linear(span, hir.captures.len())?;

    // Construct a closure environment.
    for (capture, needs) in hir.captures.iter().copied().zip(&linear) {
        let out = needs.output()?;

        if hir.do_move {
            let var = cx.scopes.take(&mut cx.q, span, capture)?;
            var.move_(cx.asm, span, Some(&"capture"), out)?;
        } else {
            let var = cx.scopes.get(&mut cx.q, span, capture)?;
            var.copy(cx, span, Some(&"capture"), out)?;
        }
    }

    cx.asm.push(
        Inst::Closure {
            hash: hir.hash,
            addr: linear.addr(),
            count: hir.captures.len(),
            out: needs.alloc_output(&mut cx.scopes)?,
        },
        span,
    )?;

    Ok(Asm::new(span))
}

/// Assemble a continue expression.
#[instrument(span = span)]
fn expr_continue<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::ExprContinue<'hir>,
    span: &'hir dyn Spanned,
    _: &mut Needs<'_>,
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

    cx.asm.jump(&last_loop.continue_label, span)?;
    Ok(Asm::new(span))
}

/// Assemble an expr field access, like `<value>.<field>`.
#[instrument(span = span)]
fn expr_field_access<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::ExprFieldAccess<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    // Optimizations!
    //
    // TODO: perform deferred compilation for expressions instead, so we can
    // e.g. inspect if it compiles down to a local access instead of
    // climbing the hir like we do here.
    if let (hir::ExprKind::Variable(name), hir::ExprField::Index(index)) =
        (hir.expr.kind, hir.expr_field)
    {
        let var = cx.scopes.get(&mut cx.q, span, name)?;

        cx.asm.push_with_comment(
            Inst::TupleIndexGetAt {
                addr: var.addr,
                index,
                out: needs.alloc_output(&mut cx.scopes)?,
            },
            span,
            &var,
        )?;

        return Ok(Asm::new(span));
    }

    let mut addr = Needs::alloc(cx, span)?;
    expr(cx, &hir.expr, &mut addr)?;

    match hir.expr_field {
        hir::ExprField::Index(index) => {
            cx.asm.push(
                Inst::TupleIndexGetAt {
                    addr: addr.addr()?,
                    index,
                    out: needs.alloc_output(&mut cx.scopes)?,
                },
                span,
            )?;
        }
        hir::ExprField::Ident(field) => {
            let slot = cx.q.unit.new_static_string(span, field)?;
            cx.asm.push(
                Inst::ObjectIndexGetAt {
                    addr: addr.addr()?,
                    slot,
                    out: needs.alloc_output(&mut cx.scopes)?,
                },
                span,
            )?;
        }
        _ => return Err(compile::Error::new(span, ErrorKind::BadFieldAccess)),
    }

    cx.scopes.free(addr)?;
    Ok(Asm::new(span))
}

/// Assemble an expression for loop.
#[instrument(span = span)]
fn expr_for<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::ExprFor<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    let continue_label = cx.asm.new_label("for_continue");
    let end_label = cx.asm.new_label("for_end");
    let break_label = cx.asm.new_label("for_break");

    let (iter_offset, loop_scope_expected) = {
        let loop_scope_expected = cx.scopes.child(span)?;
        let mut iter_offset = cx.scopes.alloc(span)?;

        expr(cx, &hir.iter, &mut iter_offset)?;

        cx.asm.push_with_comment(
            Inst::CallAssociated {
                addr: iter_offset.addr()?,
                hash: *Protocol::INTO_ITER,
                args: 0,
                out: iter_offset.output()?,
            },
            &hir.iter,
            &"Protocol::INTO_ITER",
        )?;

        (iter_offset, loop_scope_expected)
    };

    // Declare named loop variable.
    let binding_offset = cx.scopes.alloc(&hir.binding)?;

    // Declare storage for memoized `next` instance fn.
    let next_offset = if cx.options.memoize_instance_fn {
        let offset = cx.scopes.alloc(&hir.iter)?;

        cx.asm.push_with_comment(
            Inst::LoadInstanceFn {
                addr: iter_offset.addr()?,
                hash: *Protocol::NEXT,
                out: offset.output()?,
            },
            &hir.iter,
            &"Protocol::NEXT",
        )?;

        Some(offset)
    } else {
        None
    };

    cx.asm.label(&continue_label)?;

    cx.loops.push(Loop {
        label: hir.label,
        continue_label: continue_label.try_clone()?,
        break_label: break_label.try_clone()?,
        output: needs.alloc_output(&mut cx.scopes)?,
        drop: Some(iter_offset.addr()?),
    })?;

    // Use the memoized loop variable.
    if let Some(next_offset) = next_offset {
        cx.asm.push(
            Inst::CallFn {
                function: iter_offset.addr()?,
                addr: next_offset.addr()?,
                args: 1,
                out: binding_offset.output()?,
            },
            span,
        )?;
    } else {
        cx.asm.push_with_comment(
            Inst::CallAssociated {
                addr: iter_offset.addr()?,
                hash: *Protocol::NEXT,
                args: 0,
                out: binding_offset.output()?,
            },
            span,
            &"Protocol::NEXT",
        )?;
    }

    // Test loop condition and unwrap the option, or jump to `end_label` if the current value is `None`.
    cx.asm.iter_next(
        binding_offset.addr()?,
        &end_label,
        &hir.binding,
        binding_offset.output()?,
    )?;

    let guard = cx.scopes.child(&hir.body)?;

    pat_binding_with_addr(cx, &hir.binding, binding_offset.addr()?)?;

    block(cx, &hir.body, &mut Needs::none(span))?;
    cx.scopes.pop(span, guard)?;

    cx.asm.jump(&continue_label, span)?;
    cx.asm.label(&end_label)?;

    // Drop the iterator.
    cx.asm.push(
        Inst::Drop {
            addr: iter_offset.addr()?,
        },
        span,
    )?;

    cx.scopes.pop(span, loop_scope_expected)?;

    if let Some(out) = needs.try_alloc_addr(&mut cx.scopes)? {
        cx.asm.push(Inst::unit(out.output()), span)?;
    }

    // NB: breaks produce their own value.
    cx.asm.label(&break_label)?;
    cx.loops.pop();
    Ok(Asm::new(span))
}

/// Assemble an if expression.
#[instrument(span = span)]
fn expr_if<'a, 'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::Conditional<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    let output_addr = if hir.fallback.is_none() {
        needs.try_alloc_addr(&mut cx.scopes)?
    } else {
        None
    };

    let end_label = cx.asm.new_label("if_end");

    let mut branches = Vec::new();

    for branch in hir.branches {
        let label = cx.asm.new_label("if_branch");
        let scope = condition(cx, branch.condition, &label)?;
        branches.try_push((branch, label, scope))?;
    }

    // use fallback as fall through.
    if let Some(b) = hir.fallback {
        block(cx, b, needs)?;
    } else if let Some(out) = output_addr {
        cx.asm.push(Inst::unit(out.output()), span)?;
    }

    cx.asm.jump(&end_label, span)?;

    let mut it = branches.into_iter().peekable();

    while let Some((branch, label, scope)) = it.next() {
        cx.asm.label(&label)?;

        let scopes = cx.scopes.push(branch, scope)?;

        if hir.fallback.is_none() {
            block(cx, &branch.block, &mut Needs::none(branch))?;

            if let Some(out) = output_addr {
                cx.asm.push(Inst::unit(out.output()), span)?;
            }
        } else {
            block(cx, &branch.block, needs)?;
        }

        cx.scopes.pop(branch, scopes)?;

        if it.peek().is_some() {
            cx.asm.jump(&end_label, branch)?;
        }
    }

    cx.asm.label(&end_label)?;
    Ok(Asm::new(span))
}

/// Assemble an expression.
#[instrument(span = span)]
fn expr_index<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::ExprIndex<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    let guard = cx.scopes.child(span)?;

    let mut target = cx.scopes.alloc(span)?;
    let mut index = cx.scopes.alloc(span)?;

    expr(cx, &hir.target, &mut target)?;
    expr(cx, &hir.index, &mut index)?;

    cx.asm.push(
        Inst::IndexGet {
            index: index.addr()?,
            target: target.addr()?,
            out: needs.alloc_output(&mut cx.scopes)?,
        },
        span,
    )?;

    cx.scopes.pop(span, guard)?;
    Ok(Asm::new(span))
}

/// Assemble a let expression.
#[instrument(span = hir)]
fn expr_let<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::ExprLet<'hir>,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    let load = |cx: &mut Ctxt<'_, 'hir, '_>, needs: &mut Needs<'_>| {
        // NB: assignments "move" the value being assigned.
        expr(cx, &hir.expr, needs)?;
        Ok(())
    };

    let false_label = cx.asm.new_label("let_panic");

    if pat_binding(cx, &hir.pat, &false_label, &load)? {
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
    if let Some(out) = needs.try_alloc_addr(&mut cx.scopes)? {
        cx.asm.push(Inst::unit(out.output()), hir)?;
    }

    Ok(Asm::new(hir))
}

#[instrument(span = span)]
fn expr_match<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::ExprMatch<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    let expected_scopes = cx.scopes.child(span)?;

    let mut offset = cx.scopes.alloc(span)?;
    expr(cx, &hir.expr, &mut offset)?;

    let end_label = cx.asm.new_label("match_end");
    let mut branches = Vec::new();

    for branch in hir.branches {
        let span = branch;

        let branch_label = cx.asm.new_label("match_branch");
        let match_false = cx.asm.new_label("match_false");

        let parent_guard = cx.scopes.child(span)?;

        let load = |cx: &mut Ctxt, needs: &mut Needs<'_>| {
            needs.assign_addr(cx, offset.addr()?)?;
            Ok(())
        };

        pat_binding(cx, &branch.pat, &match_false, &load)?;

        let scope = if let Some(condition) = branch.condition {
            let span = condition;
            let mut cond = cx.scopes.alloc(condition)?;

            let guard = cx.scopes.child(span)?;
            expr(cx, condition, &mut cond)?;
            cx.scopes.pop(span, guard)?;

            let scope = cx.scopes.pop(span, parent_guard)?;
            cx.asm.jump_if_not(cond.addr()?, &match_false, span)?;
            cx.asm.jump(&branch_label, span)?;
            scope
        } else {
            cx.scopes.pop(span, parent_guard)?
        };

        cx.asm.jump(&branch_label, span)?;
        cx.asm.label(&match_false)?;

        branches.try_push((branch_label, scope))?;
    }

    // what to do in case nothing matches and the pattern doesn't have any
    // default match branch.
    if let Some(addr) = needs.try_alloc_addr(&mut cx.scopes)? {
        cx.asm.push(Inst::unit(addr.output()), span)?;
    }

    cx.asm.jump(&end_label, span)?;

    let mut it = hir.branches.iter().zip(branches).peekable();

    while let Some((branch, (label, scope))) = it.next() {
        let span = branch;

        cx.asm.label(&label)?;

        let expected = cx.scopes.push(span, scope)?;
        expr(cx, &branch.body, needs)?;
        cx.scopes.pop(span, expected)?;

        if it.peek().is_some() {
            cx.asm.jump(&end_label, span)?;
        }
    }

    cx.asm.label(&end_label)?;

    // pop the implicit scope where we store the anonymous match variable.
    cx.scopes.pop(span, expected_scopes)?;
    Ok(Asm::new(span))
}

/// Compile a literal object.
#[instrument(span = span)]
fn expr_object<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::ExprObject<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    let guard = cx.scopes.child(span)?;

    let mut linear = cx.scopes.linear(span, hir.assignments.len())?;

    for (assign, needs) in hir.assignments.iter().zip(&mut linear) {
        expr(cx, &assign.assign, needs)?;
    }

    let slot =
        cx.q.unit
            .new_static_object_keys_iter(span, hir.assignments.iter().map(|a| a.key.1))?;

    match hir.kind {
        hir::ExprObjectKind::EmptyStruct { hash } => {
            if needs.value() {
                cx.asm.push(
                    Inst::EmptyStruct {
                        hash,
                        out: needs.alloc_output(&mut cx.scopes)?,
                    },
                    span,
                )?;
            }
        }
        hir::ExprObjectKind::Struct { hash } => {
            cx.asm.push(
                Inst::Struct {
                    addr: linear.addr(),
                    hash,
                    slot,
                    out: needs.alloc_output(&mut cx.scopes)?,
                },
                span,
            )?;
        }
        hir::ExprObjectKind::StructVariant { hash } => {
            cx.asm.push(
                Inst::StructVariant {
                    addr: linear.addr(),
                    hash,
                    slot,
                    out: needs.alloc_output(&mut cx.scopes)?,
                },
                span,
            )?;
        }
        hir::ExprObjectKind::ExternalType { hash, args } => {
            reorder_field_assignments(cx, hir, linear.addr(), span)?;

            cx.asm.push(
                Inst::Call {
                    hash,
                    addr: linear.addr(),
                    args,
                    out: needs.alloc_output(&mut cx.scopes)?,
                },
                span,
            )?;
        }
        hir::ExprObjectKind::Anonymous => {
            cx.asm.push(
                Inst::Object {
                    addr: linear.addr(),
                    slot,
                    out: needs.alloc_output(&mut cx.scopes)?,
                },
                span,
            )?;
        }
    }

    // No need to encode an object since the value is not needed.
    if !needs.value() {
        cx.q.diagnostics
            .not_used(cx.source_id, span, cx.context())?;
    }

    cx.scopes.pop(span, guard)?;
    Ok(Asm::new(span))
}

/// Reorder the position of the field assignments on the stack so that they
/// match the expected argument order when invoking the constructor function.
fn reorder_field_assignments<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::ExprObject<'hir>,
    base: InstAddress,
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

    let base = base.offset();

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

            let a = InstAddress::new(a);
            let b = InstAddress::new(b);
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
    span: &'hir dyn Spanned,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    let guard = cx.scopes.child(span)?;

    let one: [&hir::Expr<'hir>; 1];
    let two: [&hir::Expr<'hir>; 2];

    let (range, values) = match hir {
        hir::ExprRange::RangeFrom { start } => {
            one = [start];
            (InstRange::RangeFrom, &one[..])
        }
        hir::ExprRange::RangeFull => (InstRange::RangeFull, &[][..]),
        hir::ExprRange::RangeInclusive { start, end } => {
            two = [start, end];
            (InstRange::RangeInclusive, &two[..])
        }
        hir::ExprRange::RangeToInclusive { end } => {
            one = [end];
            (InstRange::RangeToInclusive, &one[..])
        }
        hir::ExprRange::RangeTo { end } => {
            one = [end];
            (InstRange::RangeTo, &one[..])
        }
        hir::ExprRange::Range { start, end } => {
            two = [start, end];
            (InstRange::Range, &two[..])
        }
    };

    let mut linear = cx.scopes.linear(span, values.len())?;

    for (e, needs) in values.iter().zip(&mut linear) {
        expr(cx, e, needs)?;
    }

    if needs.value() {
        cx.asm.push(
            Inst::Range {
                addr: linear.addr(),
                range,
                out: needs.alloc_output(&mut cx.scopes)?,
            },
            span,
        )?;
    }

    cx.scopes.pop(span, guard)?;
    cx.scopes.free_linear(linear)?;
    Ok(Asm::new(span))
}

/// Assemble a return expression.
#[instrument(span = span)]
fn expr_return<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: Option<&'hir hir::Expr<'hir>>,
    span: &'hir dyn Spanned,
    _: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    // NB: drop any loop temporaries.
    for l in cx.loops.iter() {
        if let Some(addr) = l.drop {
            cx.asm.push(Inst::Drop { addr }, span)?;
        }
    }

    if let Some(e) = hir {
        return_(cx, span, e, expr)?;
    } else {
        cx.asm.push(Inst::ReturnUnit, span)?;
    }

    Ok(Asm::new(span))
}

/// Assemble a select expression.
#[instrument(span = span)]
fn expr_select<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::ExprSelect<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut Needs<'_>,
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
        expr(cx, &branch.expr, needs)?;
    }

    let branch_addr = cx.scopes.alloc(span)?;

    cx.asm.push(
        Inst::Select {
            len,
            out: branch_addr.output()?,
        },
        span,
    )?;

    for (branch, (label, _)) in branches.iter().enumerate() {
        cx.asm
            .jump_if_branch(branch_addr.addr()?, branch as i64, label, span)?;
    }

    if let Some((_, label)) = &default_branch {
        cx.asm.jump(label, span)?;
    }

    cx.asm.jump(&end_label, span)?;

    for (label, branch) in branches {
        cx.asm.label(&label)?;

        let expected = cx.scopes.child(&branch.body)?;

        let mut needs = match branch.pat.pat.kind {
            hir::PatKind::Path(&hir::PatPathKind::Ident(name)) => {
                let needs = cx.scopes.alloc(&branch.pat)?;
                cx.scopes
                    .define(&branch.pat, hir::Name::Str(name), needs.addr()?)?;
                needs
            }
            hir::PatKind::Ignore => Needs::none(&branch.pat),
            _ => {
                return Err(compile::Error::new(
                    &branch.pat,
                    ErrorKind::UnsupportedSelectPattern,
                ));
            }
        };

        // Set up a new scope with the binding.
        expr(cx, &branch.body, &mut needs)?;
        cx.scopes.pop(&branch.body, expected)?;
        cx.asm.jump(&end_label, span)?;
    }

    if let Some((branch, label)) = default_branch {
        cx.asm.label(&label)?;
        expr(cx, branch, needs)?;
    }

    cx.asm.label(&end_label)?;

    cx.contexts
        .pop()
        .ok_or("Missing parent context")
        .with_span(span)?;

    Ok(Asm::new(span))
}

/// Assemble a try expression.
#[instrument(span = span)]
fn expr_try<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::Expr<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    let mut addr = cx.scopes.alloc(span)?;
    expr(cx, hir, &mut addr)?;

    cx.asm.push(
        Inst::Try {
            addr: addr.addr()?,
            out: needs.alloc_output(&mut cx.scopes)?,
        },
        span,
    )?;

    cx.scopes.free(addr)?;
    Ok(Asm::new(span))
}

/// Assemble a literal tuple.
#[instrument(span = span)]
fn expr_tuple<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::ExprSeq<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    macro_rules! tuple {
        ($variant:ident, $($var:ident),*) => {{
            let guard = cx.scopes.child(span)?;

            $(
            let $var = {
                let mut addr = cx.scopes.alloc($var)?;
                expr(cx, $var, &mut addr)?;
                addr
            };
            )*

            if let Some(addr) = needs.try_alloc_addr(&mut cx.scopes)? {
                cx.asm.push(
                    Inst::$variant {
                        args: [$($var.addr()?,)*],
                        out: addr.output(),
                    },
                    span,
                )?;
            }

            cx.scopes.pop(span, guard)?;
        }};
    }

    match hir.items {
        [] => {
            cx.asm
                .push(Inst::unit(needs.alloc_output(&mut cx.scopes)?), span)?;
        }
        [e1] => tuple!(Tuple1, e1),
        [e1, e2] => tuple!(Tuple2, e1, e2),
        [e1, e2, e3] => tuple!(Tuple3, e1, e2, e3),
        [e1, e2, e3, e4] => tuple!(Tuple4, e1, e2, e3, e4),
        _ => {
            let mut linear = cx.scopes.linear(span, hir.items.len())?;

            for (e, needs) in hir.items.iter().zip(&mut linear) {
                expr(cx, e, needs)?;
            }

            if needs.value() {
                cx.asm.push(
                    Inst::Tuple {
                        addr: linear.addr(),
                        count: hir.items.len(),
                        out: needs.alloc_output(&mut cx.scopes)?,
                    },
                    span,
                )?;
            }

            cx.scopes.free_linear(linear)?;
        }
    }

    Ok(Asm::new(span))
}

/// Assemble a unary expression.
#[instrument(span = span)]
fn expr_unary<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::ExprUnary<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    expr(cx, &hir.expr, needs)?;

    match hir.op {
        ast::UnOp::Not(..) => {
            cx.asm.push(
                Inst::Not {
                    addr: needs.addr()?,
                    out: needs.output()?,
                },
                span,
            )?;
        }
        ast::UnOp::Neg(..) => {
            cx.asm.push(
                Inst::Neg {
                    addr: needs.addr()?,
                    out: needs.output()?,
                },
                span,
            )?;
        }
        op => {
            return Err(compile::Error::new(
                span,
                ErrorKind::UnsupportedUnaryOp { op },
            ));
        }
    }

    Ok(Asm::new(span))
}

/// Assemble a literal vector.
#[instrument(span = span)]
fn expr_vec<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::ExprSeq<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    let mut linear = cx.scopes.linear(span, hir.items.len())?;
    let count = hir.items.len();

    for (e, needs) in hir.items.iter().zip(&mut linear) {
        expr(cx, e, needs)?;
    }

    if let Some(out) = needs.try_alloc_addr(&mut cx.scopes)? {
        cx.asm.push(
            Inst::Vec {
                addr: linear.addr(),
                count,
                out: out.output(),
            },
            span,
        )?;
    }

    cx.scopes.free_linear(linear)?;
    Ok(Asm::new(span))
}

/// Assemble a while loop.
#[instrument(span = span)]
fn expr_loop<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::ExprLoop<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    let continue_label = cx.asm.new_label("while_continue");
    let then_label = cx.asm.new_label("while_then");
    let end_label = cx.asm.new_label("while_end");
    let break_label = cx.asm.new_label("while_break");

    cx.loops.push(Loop {
        label: hir.label,
        continue_label: continue_label.try_clone()?,
        break_label: break_label.try_clone()?,
        output: needs.alloc_output(&mut cx.scopes)?,
        drop: None,
    })?;

    cx.asm.label(&continue_label)?;

    let expected = if let Some(hir) = hir.condition {
        let then_scope = condition(cx, hir, &then_label)?;
        let expected = cx.scopes.push(hir, then_scope)?;

        cx.asm.jump(&end_label, span)?;
        cx.asm.label(&then_label)?;
        Some(expected)
    } else {
        None
    };

    block(cx, &hir.body, &mut Needs::none(span))?;

    if let Some(expected) = expected {
        cx.scopes.pop(span, expected)?;
    }

    cx.asm.jump(&continue_label, span)?;
    cx.asm.label(&end_label)?;

    if let Some(out) = needs.try_alloc_addr(&mut cx.scopes)? {
        cx.asm.push(Inst::unit(out.output()), span)?;
    }

    // NB: breaks produce their own value / perform their own cleanup.
    cx.asm.label(&break_label)?;
    cx.loops.pop();
    Ok(Asm::new(span))
}

/// Assemble a `yield` expression.
#[instrument(span = span)]
fn expr_yield<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: Option<&'hir hir::Expr<'hir>>,
    span: &'hir dyn Spanned,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    let out = needs.alloc_output(&mut cx.scopes)?;

    if let Some(e) = hir {
        let mut addr = cx.scopes.alloc(span)?;
        expr(cx, e, &mut addr)?;

        cx.asm.push(
            Inst::Yield {
                addr: addr.addr()?,
                out,
            },
            span,
        )?;
    } else {
        cx.asm.push(Inst::YieldUnit { out }, span)?;
    }

    Ok(Asm::new(span))
}

/// Assemble a literal value.
#[instrument(span = span)]
fn lit<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: hir::Lit<'_>,
    span: &'hir dyn Spanned,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    // Elide the entire literal if it's not needed.
    let Some(addr) = needs.try_alloc_addr(&mut cx.scopes)? else {
        cx.q.diagnostics
            .not_used(cx.source_id, span, cx.context())?;
        return Ok(Asm::new(span));
    };

    let out = addr.output();

    match hir {
        hir::Lit::Bool(v) => {
            cx.asm.push(Inst::bool(v, out), span)?;
        }
        hir::Lit::Byte(v) => {
            cx.asm.push(Inst::byte(v, out), span)?;
        }
        hir::Lit::Char(v) => {
            cx.asm.push(Inst::char(v, out), span)?;
        }
        hir::Lit::Integer(v) => {
            cx.asm.push(Inst::integer(v, out), span)?;
        }
        hir::Lit::Float(v) => {
            cx.asm.push(Inst::float(v, out), span)?;
        }
        hir::Lit::Str(string) => {
            let slot = cx.q.unit.new_static_string(span, string)?;
            cx.asm.push(Inst::String { slot, out }, span)?;
        }
        hir::Lit::ByteStr(bytes) => {
            let slot = cx.q.unit.new_static_bytes(span, bytes)?;
            cx.asm.push(Inst::Bytes { slot, out }, span)?;
        }
    };

    Ok(Asm::new(span))
}

/// Assemble a local expression.
#[instrument(span = hir)]
fn local<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::Local<'hir>,
    needs: &mut Needs<'_>,
) -> compile::Result<Asm<'hir>> {
    let load = |cx: &mut Ctxt<'_, 'hir, '_>, needs: &mut Needs<'_>| {
        expr(cx, &hir.expr, needs)?;
        Ok(())
    };

    let false_label = cx.asm.new_label("let_panic");

    if pat_binding(cx, &hir.pat, &false_label, &load)? {
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
        cx.asm.push(Inst::unit(needs.output()?), hir)?;
    }

    Ok(Asm::new(hir))
}
