use core::fmt;
use core::mem::take;
use core::slice;

use crate::alloc::prelude::*;
use crate::alloc::BTreeMap;
use crate::ast::{self, Span, Spanned};
use crate::compile::ir;
use crate::compile::{self, Assembly, AssemblyInst, ErrorKind, ItemId, ModId, Options, WithSpan};
use crate::hir;
use crate::query::{ConstFn, Query, Used};
use crate::runtime::{
    ConstValue, Inst, InstAddress, InstAssignOp, InstOp, InstRange, InstTarget, InstValue,
    InstVariant, Label, PanicReason, Protocol, TypeCheck,
};
use crate::shared::FixedVec;
use crate::{Hash, SourceId};

use super::{Address, Any, Break, Breaks, Linear, Needs, ScopeHandle, Scopes};

use rune_macros::instrument;

macro_rules! converge {
    ($expr:expr $(, $method:ident($($diverge:expr),* $(,)?))?) => {
        match $expr {
            Asm {
                outcome: Outcome::Converge(data),
                ..
            } => data,
            Asm {
                span,
                outcome: Outcome::Diverge,
            } => {
                $($($diverge.$method()?;)*)*

                return Ok(Asm {
                    span,
                    outcome: Outcome::Diverge,
                })
            }
        }
    };
}

enum Pattern {
    Irrefutable,
    Refutable,
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
    pub(crate) scopes: &'a Scopes<'hir>,
    /// Context for which to emit warnings.
    pub(crate) contexts: Vec<Span>,
    /// The nesting of loop we are currently in.
    pub(crate) breaks: Breaks<'hir>,
    /// Enabled optimizations.
    pub(crate) options: &'a Options,
    /// Work buffer for select branches.
    pub(crate) select_branches: Vec<(Label, &'hir hir::ExprSelectBranch<'hir>)>,
    /// Values to drop.
    pub(crate) drop: Vec<InstAddress>,
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

enum Outcome<T> {
    Converge(T),
    Diverge,
}

#[must_use = "Assembly should be checked for convergence to reduce code generation"]
struct Asm<'hir, T = ()> {
    span: &'hir dyn Spanned,
    outcome: Outcome<T>,
}

impl<'hir, T> Asm<'hir, T> {
    #[inline]
    fn new(span: &'hir dyn Spanned, data: T) -> Self {
        Self {
            span,
            outcome: Outcome::Converge(data),
        }
    }

    #[inline]
    fn diverge(span: &'hir dyn Spanned) -> Self {
        Self {
            span,
            outcome: Outcome::Diverge,
        }
    }

    /// Used as to ignore divergence.
    #[inline]
    fn ignore(self) {}
}

impl<'hir, T> Asm<'hir, T> {
    /// Test if the assembly converges and return the data associated with it.
    #[inline]
    fn into_converging(self) -> Option<T> {
        match self.outcome {
            Outcome::Converge(data) => Some(data),
            Outcome::Diverge => None,
        }
    }

    /// Test if the assembly diverges.
    #[inline]
    fn diverging(self) -> bool {
        matches!(self.outcome, Outcome::Diverge)
    }

    /// Test if the assembly converges.
    #[inline]
    fn converging(self) -> bool {
        matches!(self.outcome, Outcome::Converge(..))
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
pub(crate) fn fn_from_item_fn<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ItemFn<'hir>,
    instance_fn: bool,
) -> compile::Result<()> {
    let mut first = true;

    let mut arguments = cx.scopes.linear(hir, hir.args.len())?;

    for (arg, needs) in hir.args.iter().zip(&mut arguments) {
        match arg {
            hir::FnArg::SelfValue(span) => {
                if !instance_fn || !first {
                    return Err(compile::Error::new(span, ErrorKind::UnsupportedSelf));
                }

                cx.scopes.define(span, hir::Name::SelfValue, needs)?;
            }
            hir::FnArg::Pat(pat) => {
                let asm = pattern_panic(cx, pat, move |cx, false_label| {
                    fn_arg_pat(cx, pat, needs, false_label)
                })?;

                asm.ignore();
            }
        }

        first = false;
    }

    if hir.body.value.is_some() {
        return_(cx, hir, &hir.body, block_without_scope)?.ignore();
    } else {
        let mut needs = Any::ignore(&hir.body);

        if block_without_scope(cx, &hir.body, &mut needs)?.converging() {
            cx.asm.push(Inst::ReturnUnit, hir)?;
        }
    }

    arguments.free()?;
    cx.scopes.pop_last(hir)?;
    Ok(())
}

/// Assemble an async block.
#[instrument(span = hir.block.span)]
pub(crate) fn async_block_secondary<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::AsyncBlock<'hir>,
) -> compile::Result<()> {
    let linear = cx.scopes.linear(&hir.block, hir.captures.len())?;

    for (name, needs) in hir.captures.iter().copied().zip(&linear) {
        cx.scopes.define(&hir.block, name, needs)?;
    }

    return_(cx, &hir.block, &hir.block, block_without_scope)?.ignore();

    linear.free()?;
    cx.scopes.pop_last(&hir.block)?;
    Ok(())
}

/// Assemble the body of a closure function.
#[instrument(span = span)]
pub(crate) fn expr_closure_secondary<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ExprClosure<'hir>,
    span: &'hir dyn Spanned,
) -> compile::Result<()> {
    let mut arguments = cx.scopes.linear(span, hir.args.len())?;
    let environment = cx.scopes.linear(span, hir.captures.len())?;

    if !hir.captures.is_empty() {
        cx.asm.push(
            Inst::Environment {
                addr: environment.addr(),
                count: hir.captures.len(),
                out: environment.addr().output(),
            },
            span,
        )?;

        for (capture, needs) in hir.captures.iter().copied().zip(&environment) {
            cx.scopes.define(span, capture, needs)?;
        }
    }

    for (arg, needs) in hir.args.iter().zip(&mut arguments) {
        match arg {
            hir::FnArg::SelfValue(span) => {
                return Err(compile::Error::new(span, ErrorKind::UnsupportedSelf))
            }
            hir::FnArg::Pat(pat) => {
                let asm = pattern_panic(cx, pat, move |cx, false_label| {
                    fn_arg_pat(cx, pat, needs, false_label)
                })?;

                asm.ignore();
            }
        }
    }

    return_(cx, span, &hir.body, expr)?.ignore();

    environment.free()?;
    arguments.free()?;
    cx.scopes.pop_last(span)?;
    Ok(())
}

#[instrument(span = pat)]
fn fn_arg_pat<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    pat: &'hir hir::PatBinding<'hir>,
    needs: &mut dyn Needs<'a, 'hir>,
    false_label: &Label,
) -> compile::Result<Asm<'hir, Pattern>> {
    let Some(addr) = needs.try_as_addr()? else {
        return Err(compile::Error::msg(
            needs.span(),
            "Expected need to be populated outside of pattern",
        ));
    };

    let addr = addr.addr();

    let mut load = |cx: &mut Ctxt<'a, 'hir, '_>, needs: &mut dyn Needs<'a, 'hir>| {
        needs.assign_addr(cx, addr)?;
        Ok(Asm::new(pat, ()))
    };

    let out = match pat.names {
        [name] => pat_binding_with_single(cx, pat, &pat.pat, *name, false_label, &mut load, needs)?,
        _ => pat_binding(cx, pat, false_label, &mut load)?,
    };

    Ok(out)
}

/// Assemble a return statement from the given Assemble.
fn return_<'a, 'hir, T>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    span: &'hir dyn Spanned,
    hir: T,
    asm: impl FnOnce(&mut Ctxt<'a, 'hir, '_>, T, &mut dyn Needs<'a, 'hir>) -> compile::Result<Asm<'hir>>,
) -> compile::Result<Asm<'hir>> {
    let mut needs = cx.scopes.defer(span).with_name("return value");
    converge!(asm(cx, hir, &mut needs)?, free(needs));

    cx.asm.push(
        Inst::Return {
            addr: needs.addr()?.addr(),
        },
        span,
    )?;

    needs.free()?;
    Ok(Asm::new(span, ()))
}

fn pattern_panic<'a, 'hir, 'arena, F>(
    cx: &mut Ctxt<'a, 'hir, 'arena>,
    span: &'hir dyn Spanned,
    f: F,
) -> compile::Result<Asm<'hir>>
where
    F: FnOnce(&mut Ctxt<'a, 'hir, 'arena>, &Label) -> compile::Result<Asm<'hir, Pattern>>,
{
    let false_label = cx.asm.new_label("pattern_panic");

    if matches!(converge!(f(cx, &false_label)?), Pattern::Refutable) {
        cx.q.diagnostics
            .let_pattern_might_panic(cx.source_id, span, cx.context())?;

        let match_label = cx.asm.new_label("patter_match");

        cx.asm.jump(&match_label, span)?;
        cx.asm.label(&false_label)?;
        cx.asm.push(
            Inst::Panic {
                reason: PanicReason::UnmatchedPattern,
            },
            span,
        )?;

        cx.asm.label(&match_label)?;
    }

    Ok(Asm::new(span, ()))
}

/// Encode a pattern from a known set of bindings.
///
/// Returns a boolean indicating if the label was used.
#[instrument(span = hir)]
fn pat_binding<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::PatBinding<'hir>,
    false_label: &Label,
    load: &mut dyn FnMut(
        &mut Ctxt<'a, 'hir, '_>,
        &mut dyn Needs<'a, 'hir>,
    ) -> compile::Result<Asm<'hir>>,
) -> compile::Result<Asm<'hir, Pattern>> {
    let mut linear = cx.scopes.linear(hir, hir.names.len())?;
    let pat = pat_binding_with(cx, hir, &hir.pat, hir.names, false_label, load, &mut linear)?;
    linear.forget()?;
    Ok(pat)
}

#[instrument(span = span)]
fn pat_binding_with<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    span: &'hir dyn Spanned,
    pat: &'hir hir::Pat<'hir>,
    names: &[hir::Name<'hir>],
    false_label: &Label,
    load: &mut dyn FnMut(
        &mut Ctxt<'a, 'hir, '_>,
        &mut dyn Needs<'a, 'hir>,
    ) -> compile::Result<Asm<'hir>>,
    linear: &mut [Address<'a, 'hir>],
) -> compile::Result<Asm<'hir, Pattern>> {
    let mut bindings = BTreeMap::<_, &mut dyn Needs<'a, 'hir>>::new();

    for (name, needs) in names.iter().copied().zip(linear.iter_mut()) {
        bindings.try_insert(name, needs).with_span(span)?;
    }

    let asm = self::pat(cx, pat, false_label, load, &mut bindings)?;

    if let Some(key) = bindings.into_keys().next() {
        return Err(compile::Error::msg(
            span,
            format!("Unbound name in pattern: {key:?}"),
        ));
    }

    for (name, needs) in names.iter().copied().zip(linear.iter()) {
        cx.scopes.define(needs.span(), name, needs)?;
    }

    Ok(asm)
}

#[instrument(span = span)]
fn pat_binding_with_single<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    span: &'hir dyn Spanned,
    pat: &'hir hir::Pat<'hir>,
    name: hir::Name<'hir>,
    false_label: &Label,
    load: &mut dyn FnMut(
        &mut Ctxt<'a, 'hir, '_>,
        &mut dyn Needs<'a, 'hir>,
    ) -> compile::Result<Asm<'hir>>,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir, Pattern>> {
    let mut bindings = Some::<(_, &mut dyn Needs<'a, 'hir>)>((name, needs));

    let asm = self::pat(cx, pat, false_label, load, &mut bindings)?;

    if let Some((name, _)) = bindings {
        return Err(compile::Error::msg(
            span,
            format!("Unbound name in pattern: {name:?}"),
        ));
    }

    let Some(addr) = needs.try_as_addr()? else {
        return Err(compile::Error::msg(
            needs.span(),
            "Expected need to be populated by pattern",
        ));
    };

    cx.scopes.define(needs.span(), name, addr)?;
    Ok(asm)
}

trait Bindings<K, T> {
    fn remove(&mut self, name: &K) -> Option<T>;
}

impl<K, T> Bindings<K, T> for BTreeMap<K, T>
where
    K: Ord,
{
    #[inline]
    fn remove(&mut self, name: &K) -> Option<T> {
        BTreeMap::remove(self, name)
    }
}

impl<K, T> Bindings<K, T> for Option<(K, T)>
where
    K: PartialEq,
{
    #[inline]
    fn remove(&mut self, name: &K) -> Option<T> {
        let (current, value) = self.take()?;

        if current != *name {
            *self = Some((current, value));
            return None;
        }

        Some(value)
    }
}

/// Encode a pattern.
///
/// Returns a boolean indicating if the label was used.
#[instrument(span = hir)]
fn pat<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::Pat<'hir>,
    false_label: &Label,
    load: &mut dyn FnMut(
        &mut Ctxt<'a, 'hir, '_>,
        &mut dyn Needs<'a, 'hir>,
    ) -> compile::Result<Asm<'hir>>,
    bindings: &mut dyn Bindings<hir::Name<'hir>, &mut dyn Needs<'a, 'hir>>,
) -> compile::Result<Asm<'hir, Pattern>> {
    let span = hir;

    match hir.kind {
        hir::PatKind::Ignore => {
            // ignore binding, but might still have effects, so must call load.
            converge!(load(cx, &mut Any::ignore(hir))?);
            Ok(Asm::new(span, Pattern::Irrefutable))
        }
        hir::PatKind::Path(kind) => match *kind {
            hir::PatPathKind::Kind(kind) => {
                let mut needs = cx.scopes.defer(hir);
                converge!(load(cx, &mut needs)?, free(needs));

                let inst = pat_sequence_kind_to_inst(*kind, needs.addr()?.addr(), false_label)?;
                cx.asm.push_asm_inst(inst, hir)?;

                needs.free()?;
                Ok(Asm::new(span, Pattern::Refutable))
            }
            hir::PatPathKind::Ident(name) => {
                let name = hir::Name::Str(name);

                let Some(binding) = bindings.remove(&name) else {
                    return Err(compile::Error::msg(hir, format!("No binding for {name:?}")));
                };

                converge!(load(cx, binding)?);
                Ok(Asm::new(span, Pattern::Irrefutable))
            }
        },
        hir::PatKind::Lit(hir) => Ok(pat_lit(cx, hir, false_label, load)?),
        hir::PatKind::Sequence(hir) => pat_sequence(cx, hir, span, false_label, load, bindings),
        hir::PatKind::Object(hir) => pat_object(cx, hir, span, false_label, load, bindings),
    }
}

/// Assemble a pattern literal.
#[instrument(span = hir)]
fn pat_lit<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::Expr<'_>,
    false_label: &Label,
    load: &mut dyn FnMut(
        &mut Ctxt<'a, 'hir, '_>,
        &mut dyn Needs<'a, 'hir>,
    ) -> compile::Result<Asm<'hir>>,
) -> compile::Result<Asm<'hir, Pattern>> {
    let mut needs = cx.scopes.defer(hir);
    converge!(load(cx, &mut needs)?, free(needs));

    let Some(inst) = pat_lit_inst(cx, hir, needs.addr()?.addr(), false_label)? else {
        return Err(compile::Error::new(hir, ErrorKind::UnsupportedPatternExpr));
    };

    cx.asm.push_asm_inst(inst, hir)?;

    needs.free()?;
    Ok(Asm::new(hir, Pattern::Refutable))
}

#[instrument(span = hir)]
fn pat_lit_inst<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::Expr<'_>,
    addr: InstAddress,
    false_label: &Label,
) -> compile::Result<Option<AssemblyInst>> {
    let hir::ExprKind::Lit(lit) = hir.kind else {
        return Ok(None);
    };

    let inst = match lit {
        hir::Lit::Byte(value) => AssemblyInst::EqByte {
            addr,
            value,
            else_: false_label.try_clone()?,
        },
        hir::Lit::Char(value) => AssemblyInst::EqChar {
            addr,
            value,
            else_: false_label.try_clone()?,
        },
        hir::Lit::Str(string) => AssemblyInst::EqString {
            addr,
            slot: cx.q.unit.new_static_string(hir, string)?,
            else_: false_label.try_clone()?,
        },
        hir::Lit::ByteStr(bytes) => AssemblyInst::EqBytes {
            addr,
            slot: cx.q.unit.new_static_bytes(hir, bytes)?,
            else_: false_label.try_clone()?,
        },
        hir::Lit::Integer(value) => AssemblyInst::EqInteger {
            addr,
            value,
            else_: false_label.try_clone()?,
        },
        hir::Lit::Bool(value) => AssemblyInst::EqBool {
            addr,
            value,
            else_: false_label.try_clone()?,
        },
        _ => return Ok(None),
    };

    Ok(Some(inst))
}

/// Assemble an [hir::Condition<'_>].
#[instrument(span = hir)]
fn condition<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::Condition<'hir>,
    then_label: &Label,
    false_label: &Label,
    linear: &mut [Address<'a, 'hir>],
) -> compile::Result<Asm<'hir, (ScopeHandle, Pattern)>> {
    match *hir {
        hir::Condition::Expr(hir) => {
            let scope = cx.scopes.child(hir)?;
            let mut addr = cx.scopes.defer(hir);

            let asm = if expr(cx, hir, &mut addr)?.converging() {
                cx.asm.jump_if(addr.addr()?.addr(), then_label, hir)?;
                Asm::new(hir, (scope, Pattern::Irrefutable))
            } else {
                cx.scopes.pop(hir, scope)?;
                Asm::diverge(hir)
            };

            addr.free()?;
            Ok(asm)
        }
        hir::Condition::ExprLet(hir) => {
            let span = hir;

            let scope = cx.scopes.child(span)?;

            let mut load = |cx: &mut Ctxt<'a, 'hir, '_>, needs: &mut dyn Needs<'a, 'hir>| {
                expr(cx, &hir.expr, needs)
            };

            let asm = pat_binding_with(
                cx,
                &hir.pat,
                &hir.pat.pat,
                hir.pat.names,
                false_label,
                &mut load,
                linear,
            )?;

            if let Some(pat) = asm.into_converging() {
                cx.asm.jump(then_label, span)?;
                Ok(Asm::new(span, (scope, pat)))
            } else {
                cx.scopes.pop(span, scope)?;
                Ok(Asm::diverge(span))
            }
        }
    }
}

/// Encode a vector pattern match.
#[instrument(span = span)]
fn pat_sequence<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::PatSequence<'hir>,
    span: &'hir dyn Spanned,
    false_label: &Label,
    load: &mut dyn FnMut(
        &mut Ctxt<'a, 'hir, '_>,
        &mut dyn Needs<'a, 'hir>,
    ) -> compile::Result<Asm<'hir>>,
    bindings: &mut dyn Bindings<hir::Name<'hir>, &mut dyn Needs<'a, 'hir>>,
) -> compile::Result<Asm<'hir, Pattern>> {
    let mut addr = cx.scopes.defer(span).with_name("loaded pattern sequence");
    converge!(load(cx, &mut addr)?, free(addr));

    let addr = addr.into_addr()?;
    let cond = cx.scopes.alloc(span)?.with_name("loaded pattern condition");

    if matches!(
        hir.kind,
        hir::PatSequenceKind::Anonymous {
            type_check: TypeCheck::Tuple,
            count: 0,
            is_open: false
        }
    ) {
        cx.asm.push(
            Inst::IsUnit {
                addr: addr.addr(),
                out: cond.output(),
            },
            span,
        )?;

        cx.asm.jump_if_not(cond.addr(), false_label, span)?;
    } else {
        let inst = pat_sequence_kind_to_inst(hir.kind, addr.addr(), false_label)?;
        cx.asm.push_asm_inst(inst, span)?;

        for (index, p) in hir.items.iter().enumerate() {
            let mut load = |cx: &mut Ctxt<'a, 'hir, '_>, needs: &mut dyn Needs<'a, 'hir>| {
                cx.asm.push(
                    Inst::TupleIndexGetAt {
                        addr: addr.addr(),
                        index,
                        out: needs.alloc_output()?,
                    },
                    p,
                )?;
                Ok(Asm::new(p, ()))
            };

            converge!(
                self::pat(cx, p, false_label, &mut load, bindings)?,
                free(cond, addr)
            );
        }
    }

    cond.free()?;
    addr.free()?;
    Ok(Asm::new(span, Pattern::Refutable))
}

fn pat_sequence_kind_to_inst(
    kind: hir::PatSequenceKind,
    addr: InstAddress,
    false_label: &Label,
) -> compile::Result<AssemblyInst> {
    Ok(match kind {
        hir::PatSequenceKind::Type { hash } => AssemblyInst::MatchType {
            hash,
            addr,
            else_: false_label.try_clone()?,
        },
        hir::PatSequenceKind::BuiltInVariant { type_check } => AssemblyInst::MatchBuiltIn {
            type_check,
            addr,
            else_: false_label.try_clone()?,
        },
        hir::PatSequenceKind::Variant {
            variant_hash,
            enum_hash,
            index,
        } => AssemblyInst::MatchVariant {
            variant_hash,
            enum_hash,
            index,
            addr,
            else_: false_label.try_clone()?,
        },
        hir::PatSequenceKind::Anonymous {
            type_check,
            count,
            is_open,
        } => AssemblyInst::MatchSequence {
            type_check,
            len: count,
            exact: !is_open,
            addr,
            else_: false_label.try_clone()?,
        },
    })
}

/// Assemble an object pattern.
#[instrument(span = span)]
fn pat_object<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::PatObject<'hir>,
    span: &'hir dyn Spanned,
    false_label: &Label,
    load: &mut dyn FnMut(
        &mut Ctxt<'a, 'hir, '_>,
        &mut dyn Needs<'a, 'hir>,
    ) -> compile::Result<Asm<'hir>>,
    bindings: &mut dyn Bindings<hir::Name<'hir>, &mut dyn Needs<'a, 'hir>>,
) -> compile::Result<Asm<'hir, Pattern>> {
    let mut needs = cx.scopes.defer(span);
    converge!(load(cx, &mut needs)?, free(needs));
    let addr = needs.addr()?;

    let mut string_slots = Vec::new();

    for binding in hir.bindings {
        string_slots.try_push(cx.q.unit.new_static_string(span, binding.key())?)?;
    }

    let inst = match hir.kind {
        hir::PatSequenceKind::Type { hash } => AssemblyInst::MatchType {
            hash,
            addr: addr.addr(),
            else_: false_label.try_clone()?,
        },
        hir::PatSequenceKind::BuiltInVariant { type_check } => AssemblyInst::MatchBuiltIn {
            type_check,
            addr: addr.addr(),
            else_: false_label.try_clone()?,
        },
        hir::PatSequenceKind::Variant {
            variant_hash,
            enum_hash,
            index,
        } => AssemblyInst::MatchVariant {
            variant_hash,
            enum_hash,
            index,
            addr: addr.addr(),
            else_: false_label.try_clone()?,
        },
        hir::PatSequenceKind::Anonymous { is_open, .. } => {
            let keys =
                cx.q.unit
                    .new_static_object_keys_iter(span, hir.bindings.iter().map(|b| b.key()))?;

            AssemblyInst::MatchObject {
                slot: keys,
                exact: !is_open,
                addr: addr.addr(),
                else_: false_label.try_clone()?,
            }
        }
    };

    // Copy the temporary and check that its length matches the pattern and
    // that it is indeed a vector.
    cx.asm.push_asm_inst(inst, span)?;

    for (binding, slot) in hir.bindings.iter().zip(string_slots) {
        match binding {
            hir::Binding::Binding(span, _, p) => {
                let mut load =
                    move |cx: &mut Ctxt<'a, 'hir, '_>, needs: &mut dyn Needs<'a, 'hir>| {
                        cx.asm.push(
                            Inst::ObjectIndexGetAt {
                                addr: addr.addr(),
                                slot,
                                out: needs.alloc_output()?,
                            },
                            span,
                        )?;
                        Ok(Asm::new(span, ()))
                    };

                converge!(
                    self::pat(cx, p, false_label, &mut load, bindings)?,
                    free(needs)
                );
            }
            hir::Binding::Ident(span, name) => {
                let name = hir::Name::Str(name);

                let Some(binding) = bindings.remove(&name) else {
                    return Err(compile::Error::msg(
                        binding,
                        format!("No binding for {name:?}"),
                    ));
                };

                cx.asm.push(
                    Inst::ObjectIndexGetAt {
                        addr: addr.addr(),
                        slot,
                        out: binding.output()?,
                    },
                    &span,
                )?;
            }
        }
    }

    needs.free()?;
    Ok(Asm::new(span, Pattern::Refutable))
}

/// Call a block.
#[instrument(span = hir)]
fn block<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::Block<'hir>,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let break_label = if let Some(label) = hir.label {
        let break_label = cx.asm.new_label("block_break");

        cx.breaks.push(Break {
            label: Some(label),
            continue_label: None,
            break_label: break_label.try_clone()?,
            output: Some(needs.alloc_output()?),
            drop: None,
        })?;

        Some(break_label)
    } else {
        None
    };

    let scope = cx.scopes.child(hir)?;
    let asm = block_without_scope(cx, hir, needs)?;
    cx.scopes.pop(hir, scope)?;

    if let Some(break_label) = break_label {
        cx.asm.label(&break_label)?;
        cx.breaks.pop();
    }

    Ok(asm)
}

/// Call a block.
#[instrument(span = hir)]
fn block_without_scope<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::Block<'hir>,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let mut diverge = None;
    cx.contexts.try_push(hir.span())?;

    for stmt in hir.statements {
        let mut needs = Any::ignore(hir).with_name("statement ignore");

        if let Some(cause) = diverge {
            cx.q.diagnostics.unreachable(cx.source_id, stmt, cause)?;
            continue;
        }

        let asm = match stmt {
            hir::Stmt::Local(hir) => local(cx, hir, &mut needs)?,
            hir::Stmt::Expr(hir) => expr(cx, hir, &mut needs)?,
        };

        if asm.diverging() && diverge.is_none() {
            diverge = Some(stmt);
        }
    }

    if let Some(cause) = diverge {
        if let Some(e) = hir.value {
            cx.q.diagnostics.unreachable(cx.source_id, e, cause)?;
        }
    } else if let Some(e) = hir.value {
        if expr(cx, e, needs)?.diverging() {
            diverge = Some(e);
        }
    } else if let Some(out) = needs.try_alloc_output()? {
        cx.asm.push(Inst::unit(out), hir)?;
    }

    cx.contexts
        .pop()
        .ok_or("Missing parent context")
        .with_span(hir)?;

    if diverge.is_some() {
        return Ok(Asm::diverge(hir));
    }

    Ok(Asm::new(hir, ()))
}

/// Assemble #[builtin] format_args!(...) macro.
#[instrument(span = format)]
fn builtin_format<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    format: &'hir hir::BuiltInFormat<'hir>,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    use crate::runtime::format;

    let fill = format.fill.unwrap_or(' ');
    let align = format.align.unwrap_or_default();
    let flags = format.flags.unwrap_or_default();
    let width = format.width;
    let precision = format.precision;
    let format_type = format.format_type.unwrap_or_default();

    let spec = format::FormatSpec::new(flags, fill, align, width, precision, format_type);

    converge!(expr(cx, &format.value, needs)?);

    if let Some(addr) = needs.try_alloc_addr()? {
        cx.asm.push(
            Inst::Format {
                addr: addr.addr(),
                spec,
                out: addr.output(),
            },
            format,
        )?;
    }

    Ok(Asm::new(format, ()))
}

/// Assemble #[builtin] template!(...) macro.
#[instrument(span = hir)]
fn builtin_template<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::BuiltInTemplate<'hir>,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let span = hir;

    let mut size_hint = 0;
    let mut expansions = 0;

    let mut linear = cx.scopes.linear(hir, hir.exprs.len())?;

    let mut converge = true;

    for (hir, addr) in hir.exprs.iter().zip(&mut linear) {
        if let hir::ExprKind::Lit(hir::Lit::Str(s)) = hir.kind {
            size_hint += s.len();
            let slot = cx.q.unit.new_static_string(span, s)?;
            cx.asm.push(
                Inst::String {
                    slot,
                    out: addr.output(),
                },
                span,
            )?;

            continue;
        }

        expansions += 1;

        if expr(cx, hir, addr)?.diverging() {
            converge = false;
            break;
        }
    }

    if hir.from_literal && expansions == 0 {
        cx.q.diagnostics
            .template_without_expansions(cx.source_id, span, cx.context())?;
    }

    if converge {
        cx.asm.push(
            Inst::StringConcat {
                addr: linear.addr(),
                len: hir.exprs.len(),
                size_hint,
                out: needs.alloc_output()?,
            },
            span,
        )?;
    }

    linear.free()?;

    if converge {
        Ok(Asm::new(span, ()))
    } else {
        Ok(Asm::diverge(span))
    }
}

/// Assemble a constant value.
#[instrument(span = span)]
fn const_<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    value: &ConstValue,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<()> {
    let Some(addr) = needs.try_alloc_addr()? else {
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
            cx.asm.push(Inst::String { slot, out }, span)?;
        }
        ConstValue::Bytes(ref b) => {
            let slot = cx.q.unit.new_static_bytes(span, b)?;
            cx.asm.push(Inst::Bytes { slot, out }, span)?;
        }
        ConstValue::Option(ref option) => match option {
            Some(value) => {
                const_(cx, value, span, addr)?;

                cx.asm.push(
                    Inst::Variant {
                        variant: InstVariant::Some,
                        addr: addr.addr(),
                        out,
                    },
                    span,
                )?;
            }
            None => {
                cx.asm.push(
                    Inst::Variant {
                        variant: InstVariant::None,
                        addr: addr.addr(),
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

            linear.free()?;
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

            linear.free()?;
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

            linear.free()?;
        }
    }

    Ok(())
}

/// Assemble an expression.
#[instrument(span = hir)]
fn expr<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::Expr<'hir>,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let span = hir;

    let asm = match hir.kind {
        hir::ExprKind::Variable(name) => {
            let var = cx.scopes.get(&mut cx.q, span, name)?;
            needs.assign_addr(cx, var.addr)?;
            Asm::new(span, ())
        }
        hir::ExprKind::Type(ty) => {
            if let Some(out) = needs.try_alloc_output()? {
                cx.asm.push(
                    Inst::Store {
                        value: InstValue::Type(ty),
                        out,
                    },
                    span,
                )?;
            }

            Asm::new(span, ())
        }
        hir::ExprKind::Fn(hash) => {
            if let Some(out) = needs.try_alloc_output()? {
                cx.asm.push(Inst::LoadFn { hash, out }, span)?;
            }

            Asm::new(span, ())
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
        hir::ExprKind::Break(hir) => expr_break(cx, hir, span)?,
        hir::ExprKind::Continue(hir) => expr_continue(cx, hir, span, needs)?,
        hir::ExprKind::Yield(hir) => expr_yield(cx, hir, span, needs)?,
        hir::ExprKind::Block(hir) => block(cx, hir, needs)?,
        hir::ExprKind::Return(hir) => expr_return(cx, hir, span)?,
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
fn expr_assign<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ExprAssign<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let supported = match hir.lhs.kind {
        // <var> = <value>
        hir::ExprKind::Variable(name) => {
            let var = cx.scopes.get(&mut cx.q, span, name)?;
            let mut needs = Address::assigned(var.span, cx.scopes, var.addr);
            converge!(expr(cx, &hir.rhs, &mut needs)?, free(needs));
            needs.free()?;
            true
        }
        // <expr>.<field> = <value>
        hir::ExprKind::FieldAccess(field_access) => {
            let mut target = cx.scopes.defer(&field_access.expr);
            let mut value = cx.scopes.defer(&hir.rhs);

            let asm = expr_array(
                cx,
                span,
                [(&field_access.expr, &mut target), (&hir.rhs, &mut value)],
            )?;

            // field assignment
            match field_access.expr_field {
                hir::ExprField::Ident(ident) => {
                    if let Some([target, value]) = asm.into_converging() {
                        let slot = cx.q.unit.new_static_string(span, ident)?;

                        cx.asm.push(
                            Inst::ObjectIndexSet {
                                target: target.addr(),
                                slot,
                                value: value.addr(),
                            },
                            span,
                        )?;
                    }
                }
                hir::ExprField::Index(index) => {
                    if let Some([target, value]) = asm.into_converging() {
                        cx.asm.push(
                            Inst::TupleIndexSet {
                                target: target.addr(),
                                index,
                                value: value.addr(),
                            },
                            span,
                        )?;
                    }
                }
                _ => {
                    return Err(compile::Error::new(span, ErrorKind::BadFieldAccess));
                }
            };

            target.free()?;
            value.free()?;
            true
        }
        hir::ExprKind::Index(expr_index_get) => {
            let mut target = cx.scopes.defer(&expr_index_get.target);
            let mut index = cx.scopes.defer(&expr_index_get.index);
            let mut value = cx.scopes.defer(&hir.rhs);

            let asm = expr_array(
                cx,
                span,
                [
                    (&expr_index_get.target, &mut target),
                    (&expr_index_get.index, &mut index),
                    (&hir.rhs, &mut value),
                ],
            )?;

            if let Some([target, index, value]) = asm.into_converging() {
                cx.asm.push(
                    Inst::IndexSet {
                        target: target.addr(),
                        index: index.addr(),
                        value: value.addr(),
                    },
                    span,
                )?;
            }

            value.free()?;
            index.free()?;
            target.free()?;
            true
        }
        _ => false,
    };

    if !supported {
        return Err(compile::Error::new(span, ErrorKind::UnsupportedAssignExpr));
    }

    if let Some(out) = needs.try_alloc_output()? {
        cx.asm.push(Inst::unit(out), span)?;
    }

    Ok(Asm::new(span, ()))
}

/// Assemble an `.await` expression.
#[instrument(span = hir)]
fn expr_await<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::Expr<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let mut addr = cx.scopes.defer(span);
    converge!(expr(cx, hir, &mut addr)?, free(addr));

    cx.asm.push(
        Inst::Await {
            addr: addr.addr()?.addr(),
            out: needs.alloc_output()?,
        },
        span,
    )?;

    addr.free()?;
    Ok(Asm::new(span, ()))
}

/// Assemble a binary expression.
#[instrument(span = span)]
fn expr_binary<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ExprBinary<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    // Special expressions which operates on the stack in special ways.
    if hir.op.is_assign() {
        return compile_assign_binop(cx, &hir.lhs, &hir.rhs, &hir.op, span, needs);
    }

    if hir.op.is_conditional() {
        return compile_conditional_binop(cx, &hir.lhs, &hir.rhs, &hir.op, span, needs);
    }

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

    let mut a = cx.scopes.defer(span);
    let mut b = cx.scopes.defer(span);

    let asm = expr_array(cx, span, [(&hir.lhs, &mut a), (&hir.rhs, &mut b)])?;

    if let Some([a, b]) = asm.into_converging() {
        cx.asm.push(
            Inst::Op {
                op,
                a: a.addr(),
                b: b.addr(),
                out: needs.alloc_output()?,
            },
            span,
        )?;
    }

    a.free()?;
    b.free()?;
    Ok(Asm::new(span, ()))
}

fn compile_conditional_binop<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    lhs: &'hir hir::Expr<'hir>,
    rhs: &'hir hir::Expr<'hir>,
    bin_op: &ast::BinOp,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let end_label = cx.asm.new_label("conditional_end");
    converge!(expr(cx, lhs, needs)?);
    let addr = needs.addr()?;

    match bin_op {
        ast::BinOp::And(..) => {
            cx.asm.jump_if_not(addr.addr(), &end_label, lhs)?;
        }
        ast::BinOp::Or(..) => {
            cx.asm.jump_if(addr.addr(), &end_label, lhs)?;
        }
        op => {
            return Err(compile::Error::new(
                span,
                ErrorKind::UnsupportedBinaryOp { op: *op },
            ));
        }
    }

    // rhs needs to be ignored since it might be jumped over.
    expr(cx, rhs, needs)?.ignore();
    cx.asm.label(&end_label)?;
    Ok(Asm::new(span, ()))
}

fn compile_assign_binop<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    lhs: &'hir hir::Expr<'hir>,
    rhs: &'hir hir::Expr<'hir>,
    bin_op: &ast::BinOp,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let supported = match lhs.kind {
        // <var> <op> <expr>
        hir::ExprKind::Variable(name) => {
            let var = cx.scopes.get(&mut cx.q, lhs, name)?;
            Some(InstTarget::Address(var.addr))
        }
        // <expr>.<field> <op> <value>
        hir::ExprKind::FieldAccess(field_access) => {
            let mut field = cx.scopes.defer(&field_access.expr);
            converge!(expr(cx, &field_access.expr, &mut field)?, free(field));
            let field = field.into_addr()?;

            // field assignment
            let output = match field_access.expr_field {
                hir::ExprField::Index(index) => Some(InstTarget::TupleField(field.addr(), index)),
                hir::ExprField::Ident(ident) => {
                    let n = cx.q.unit.new_static_string(&field_access.expr, ident)?;
                    Some(InstTarget::Field(field.addr(), n))
                }
                _ => {
                    return Err(compile::Error::new(span, ErrorKind::BadFieldAccess));
                }
            };

            field.free()?;
            output
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

    let mut value = cx.scopes.defer(rhs);

    if expr(cx, rhs, &mut value)?.converging() {
        cx.asm.push(
            Inst::Assign {
                target,
                op,
                value: value.addr()?.addr(),
            },
            span,
        )?;

        if let Some(out) = needs.try_alloc_output()? {
            cx.asm.push(Inst::unit(out), span)?;
        }
    }

    value.free()?;
    Ok(Asm::new(span, ()))
}

/// Assemble a block expression.
#[instrument(span = span)]
fn expr_async_block<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::ExprAsyncBlock<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let linear = cx.scopes.linear(span, hir.captures.len())?;

    for (capture, needs) in hir.captures.iter().copied().zip(&linear) {
        let out = needs.output();

        if hir.do_move {
            let var = cx.scopes.take(&mut cx.q, span, capture)?;
            var.move_(cx.asm, span, Some(&"capture"), out)?;
        } else {
            let var = cx.scopes.get(&mut cx.q, span, capture)?;
            var.copy(cx.asm, span, Some(&"capture"), out)?;
        }
    }

    cx.asm.push_with_comment(
        Inst::Call {
            hash: hir.hash,
            addr: linear.addr(),
            args: hir.captures.len(),
            out: needs.alloc_output()?,
        },
        span,
        &"async block",
    )?;

    linear.free()?;
    Ok(Asm::new(span, ()))
}

/// Assemble a constant item.
#[instrument(span = span)]
fn const_item<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hash: Hash,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let Some(const_value) = cx.q.get_const_value(hash) else {
        return Err(compile::Error::msg(
            span,
            try_format!("Missing constant value for hash {hash}"),
        ));
    };

    let const_value = const_value.try_clone().with_span(span)?;
    const_(cx, &const_value, span, needs)?;
    Ok(Asm::new(span, ()))
}

/// Assemble a break expression.
///
/// NB: loops are expected to produce a value at the end of their expression.
#[instrument(span = span)]
fn expr_break<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::ExprBreak<'hir>,
    span: &'hir dyn Spanned,
) -> compile::Result<Asm<'hir>> {
    let (break_label, output) = match hir.label {
        Some(label) => {
            let l = cx.breaks.walk_until_label(span, label, &mut cx.drop)?;
            (l.break_label.try_clone()?, l.output)
        }
        None => {
            let Some(l) = cx.breaks.last() else {
                return Err(compile::Error::new(span, ErrorKind::BreakUnsupported));
            };

            cx.drop.clear();
            cx.drop.try_extend(l.drop).with_span(span)?;
            (l.break_label.try_clone()?, l.output)
        }
    };

    if let Some(hir) = hir.expr {
        let Some(output) = output else {
            return Err(compile::Error::new(span, ErrorKind::BreakUnsupportedValue));
        };

        let mut needs = match output.as_addr() {
            Some(addr) => Any::assigned(span, cx.scopes, addr),
            None => Any::ignore(span),
        };

        converge!(expr(cx, hir, &mut needs)?, free(needs));
        needs.free()?;
    }

    // Drop loop temporaries.
    for addr in cx.drop.drain(..) {
        cx.asm.push(Inst::Drop { addr }, span)?;
    }

    cx.asm.jump(&break_label, span)?;
    Ok(Asm::diverge(span))
}

/// Assemble a call expression.
#[instrument(span = span)]
fn expr_call<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::ExprCall<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let args = hir.args.len();

    match hir.call {
        hir::Call::Var { name, .. } => {
            let linear = converge!(exprs(cx, span, hir.args)?);

            let var = cx.scopes.get(&mut cx.q, span, name)?;

            cx.asm.push(
                Inst::CallFn {
                    function: var.addr,
                    addr: linear.addr(),
                    args: hir.args.len(),
                    out: needs.alloc_output()?,
                },
                span,
            )?;

            linear.free()?;
        }
        hir::Call::Associated { target, hash } => {
            let linear = converge!(exprs_2(cx, span, slice::from_ref(target), hir.args)?);

            cx.asm.push(
                Inst::CallAssociated {
                    hash,
                    addr: linear.addr(),
                    args: args + 1,
                    out: needs.alloc_output()?,
                },
                span,
            )?;

            linear.free()?;
        }
        hir::Call::Meta { hash } => {
            let linear = converge!(exprs(cx, span, hir.args)?);

            cx.asm.push(
                Inst::Call {
                    hash,
                    addr: linear.addr(),
                    args: hir.args.len(),
                    out: needs.alloc_output()?,
                },
                span,
            )?;

            linear.free()?;
        }
        hir::Call::Expr { expr: e } => {
            let mut function = cx.scopes.defer(span);
            converge!(expr(cx, e, &mut function)?, free(function));
            let linear = converge!(exprs(cx, span, hir.args)?, free(function));

            cx.asm.push(
                Inst::CallFn {
                    function: function.addr()?.addr(),
                    addr: linear.addr(),
                    args: hir.args.len(),
                    out: needs.alloc_output()?,
                },
                span,
            )?;

            linear.free()?;
            function.free()?;
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

    Ok(Asm::new(span, ()))
}

/// Assemble an array of expressions.
#[instrument(span = span)]
fn expr_array<'a, 'hir, 'needs, const N: usize>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    span: &'hir dyn Spanned,
    array: [(&'hir hir::Expr<'hir>, &'needs mut dyn Needs<'a, 'hir>); N],
) -> compile::Result<Asm<'hir, [&'needs Address<'a, 'hir>; N]>> {
    let mut out = FixedVec::new();

    for (expr, needs) in array {
        converge!(self::expr(cx, expr, needs)?);
        let addr = needs.addr()?;
        out.try_push(addr).with_span(span)?;
    }

    Ok(Asm::new(span, out.into_inner()))
}

#[instrument(span = span)]
fn exprs<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    span: &'hir dyn Spanned,
    args: &'hir [hir::Expr<'hir>],
) -> compile::Result<Asm<'hir, Linear<'a, 'hir>>> {
    exprs_2(cx, span, args, &[])
}

#[instrument(span = span)]
fn exprs_with<'a, 'hir, T>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    span: &'hir dyn Spanned,
    args: &'hir [T],
    map: fn(&'hir T) -> &'hir hir::Expr,
) -> compile::Result<Asm<'hir, Linear<'a, 'hir>>> {
    exprs_2_with(cx, span, args, &[], map)
}

/// Assemble a linear sequence of expressions.
#[instrument(span = span)]
fn exprs_2<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    span: &'hir dyn Spanned,
    a: &'hir [hir::Expr<'hir>],
    b: &'hir [hir::Expr<'hir>],
) -> compile::Result<Asm<'hir, Linear<'a, 'hir>>> {
    exprs_2_with(cx, span, a, b, |e| e)
}

fn exprs_2_with<'a, 'hir, T>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    span: &'hir dyn Spanned,
    a: &'hir [T],
    b: &'hir [T],
    map: fn(&'hir T) -> &'hir hir::Expr,
) -> compile::Result<Asm<'hir, Linear<'a, 'hir>>> {
    let mut linear;

    match (a, b) {
        ([], []) => {
            linear = Linear::empty();
        }
        ([e], []) | ([], [e]) => {
            let e = map(e);
            let mut needs = cx.scopes.defer(e);
            converge!(expr(cx, e, &mut needs)?, free(needs));
            linear = Linear::single(needs.into_addr()?);
        }
        _ => {
            let len = a.len() + b.len();

            linear = cx.scopes.linear(span, len)?;

            let mut diverge = false;

            for (e, needs) in a.iter().chain(b.iter()).zip(&mut linear) {
                if expr(cx, map(e), needs)?.diverging() {
                    diverge = true;
                    break;
                };
            }

            if diverge {
                linear.free()?;
                return Ok(Asm::diverge(span));
            }
        }
    }

    Ok(Asm::new(span, linear))
}

/// Assemble a closure expression.
#[instrument(span = span)]
fn expr_call_closure<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::ExprCallClosure<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let Some(out) = needs.try_alloc_output()? else {
        cx.q.diagnostics
            .not_used(cx.source_id, span, cx.context())?;
        return Ok(Asm::new(span, ()));
    };

    tracing::trace!(?hir.captures, "assemble call closure");

    let linear = cx.scopes.linear(span, hir.captures.len())?;

    // Construct a closure environment.
    for (capture, needs) in hir.captures.iter().copied().zip(&linear) {
        let out = needs.output();

        if hir.do_move {
            let var = cx.scopes.take(&mut cx.q, span, capture)?;
            var.move_(cx.asm, span, Some(&"capture"), out)?;
        } else {
            let var = cx.scopes.get(&mut cx.q, span, capture)?;
            var.copy(cx.asm, span, Some(&"capture"), out)?;
        }
    }

    cx.asm.push(
        Inst::Closure {
            hash: hir.hash,
            addr: linear.addr(),
            count: hir.captures.len(),
            out,
        },
        span,
    )?;

    linear.free()?;
    Ok(Asm::new(span, ()))
}

/// Assemble a continue expression.
#[instrument(span = span)]
fn expr_continue<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::ExprContinue<'hir>,
    span: &'hir dyn Spanned,
    _: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let last_loop = if let Some(label) = hir.label {
        cx.breaks.find_label(span, label)?
    } else {
        let Some(current_loop) = cx.breaks.last() else {
            return Err(compile::Error::new(span, ErrorKind::ContinueUnsupported));
        };

        current_loop
    };

    let Some(label) = &last_loop.continue_label else {
        return Err(compile::Error::new(
            span,
            ErrorKind::ContinueUnsupportedBlock,
        ));
    };

    cx.asm.jump(label, span)?;
    Ok(Asm::new(span, ()))
}

/// Assemble an expr field access, like `<value>.<field>`.
#[instrument(span = span)]
fn expr_field_access<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ExprFieldAccess<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
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
                out: needs.alloc_output()?,
            },
            span,
            &var,
        )?;

        return Ok(Asm::new(span, ()));
    }

    let mut addr = cx.scopes.defer(span);

    if expr(cx, &hir.expr, &mut addr)?.converging() {
        let addr = addr.addr()?;

        match hir.expr_field {
            hir::ExprField::Index(index) => {
                cx.asm.push(
                    Inst::TupleIndexGetAt {
                        addr: addr.addr(),
                        index,
                        out: needs.alloc_output()?,
                    },
                    span,
                )?;
            }
            hir::ExprField::Ident(field) => {
                let slot = cx.q.unit.new_static_string(span, field)?;

                cx.asm.push(
                    Inst::ObjectIndexGetAt {
                        addr: addr.addr(),
                        slot,
                        out: needs.alloc_output()?,
                    },
                    span,
                )?;
            }
            _ => return Err(compile::Error::new(span, ErrorKind::BadFieldAccess)),
        }
    }

    addr.free()?;
    Ok(Asm::new(span, ()))
}

/// Assemble an expression for loop.
#[instrument(span = span)]
fn expr_for<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ExprFor<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let mut iter = cx.scopes.defer(span).with_name("iter");

    if !expr(cx, &hir.iter, &mut iter)?.converging() {
        iter.free()?;
        cx.q.diagnostics
            .unreachable(cx.source_id, &hir.body, &hir.iter)?;
        return Ok(Asm::diverge(span));
    }

    let continue_label = cx.asm.new_label("for_continue");
    let end_label = cx.asm.new_label("for_end");
    let break_label = cx.asm.new_label("for_break");

    // Variables.
    let iter = iter.into_addr()?;
    let into_iter = cx.scopes.alloc(span)?.with_name("into_iter");
    let binding = cx.scopes.alloc(&hir.binding)?.with_name("binding");

    cx.asm.push_with_comment(
        Inst::CallAssociated {
            addr: iter.addr(),
            hash: *Protocol::INTO_ITER,
            args: 1,
            out: into_iter.output(),
        },
        &hir.iter,
        &"Protocol::INTO_ITER",
    )?;

    // Declare storage for memoized `next` instance fn.
    let next_offset = if cx.options.memoize_instance_fn {
        let offset = cx.scopes.alloc(&hir.iter)?.with_name("memoized next");

        cx.asm.push_with_comment(
            Inst::LoadInstanceFn {
                addr: into_iter.addr(),
                hash: *Protocol::NEXT,
                out: offset.output(),
            },
            &hir.iter,
            &"Protocol::NEXT",
        )?;

        Some(offset)
    } else {
        None
    };

    cx.asm.label(&continue_label)?;

    cx.breaks.push(Break {
        label: hir.label,
        continue_label: Some(continue_label.try_clone()?),
        break_label: break_label.try_clone()?,
        output: None,
        drop: Some(into_iter.addr()),
    })?;

    // Use the memoized loop variable.
    if let Some(next_offset) = &next_offset {
        cx.asm.push(
            Inst::CallFn {
                function: next_offset.addr(),
                addr: into_iter.addr(),
                args: 1,
                out: binding.output(),
            },
            span,
        )?;
    } else {
        cx.asm.push_with_comment(
            Inst::CallAssociated {
                addr: into_iter.addr(),
                hash: *Protocol::NEXT,
                args: 1,
                out: binding.output(),
            },
            span,
            &"Protocol::NEXT",
        )?;
    }

    // Test loop condition and unwrap the option, or jump to `end_label` if the current value is `None`.
    cx.asm
        .iter_next(binding.addr(), &end_label, &hir.binding, binding.output())?;

    let inner_loop_scope = cx.scopes.child(&hir.body)?;
    let mut bindings = cx.scopes.linear(&hir.binding, hir.binding.names.len())?;

    let mut load = |cx: &mut Ctxt<'a, 'hir, '_>, needs: &mut dyn Needs<'a, 'hir>| {
        needs.assign_addr(cx, binding.addr())?;
        Ok(Asm::new(&hir.binding, ()))
    };

    let asm = pattern_panic(cx, &hir.binding, |cx, false_label| {
        pat_binding_with(
            cx,
            &hir.binding,
            &hir.binding.pat,
            hir.binding.names,
            false_label,
            &mut load,
            &mut bindings,
        )
    })?;

    asm.ignore();

    let asm = block(cx, &hir.body, &mut Any::ignore(span))?;
    bindings.free()?;
    cx.scopes.pop(span, inner_loop_scope)?;

    if asm.converging() {
        cx.asm.jump(&continue_label, span)?;
    }

    cx.asm.label(&end_label)?;

    // NB: Dropping has to happen before the break label. When breaking,
    // the break statement is responsible for ensuring that active
    // iterators are dropped.
    cx.asm.push(
        Inst::Drop {
            addr: into_iter.addr(),
        },
        span,
    )?;

    cx.asm.label(&break_label)?;

    if let Some(out) = needs.try_alloc_output()? {
        cx.asm.push(Inst::unit(out), span)?;
    }

    if let Some(next_offset) = next_offset {
        next_offset.free()?;
    }

    binding.free()?;
    into_iter.free()?;
    iter.free()?;

    cx.breaks.pop();

    Ok(Asm::new(span, ()))
}

/// Assemble an if expression.
#[instrument(span = span)]
fn expr_if<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::Conditional<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let output_addr = if hir.fallback.is_none() {
        needs.try_alloc_output()?
    } else {
        None
    };

    let end_label = cx.asm.new_label("if_end");

    let values = hir
        .branches
        .iter()
        .flat_map(|c| c.condition.count())
        .max()
        .unwrap_or(0);

    let mut linear = cx.scopes.linear(span, values)?;
    let mut branches = Vec::new();

    for branch in hir.branches {
        let then_label = cx.asm.new_label("if_branch");
        let false_label = cx.asm.new_label("if_false");

        if let Some((scope, pat)) =
            condition(cx, branch.condition, &then_label, &false_label, &mut linear)?
                .into_converging()
        {
            if matches!(pat, Pattern::Refutable) {
                cx.asm.label(&false_label)?;
            }

            let scope = cx.scopes.dangle(branch, scope)?;
            branches.try_push((branch, then_label, scope))?;
        }
    }

    // use fallback as fall through.
    let asm = if let Some(b) = hir.fallback {
        block(cx, b, needs)?
    } else if let Some(out) = output_addr {
        cx.asm.push(Inst::unit(out), span)?;
        Asm::new(span, ())
    } else {
        Asm::new(span, ())
    };

    if asm.converging() {
        cx.asm.jump(&end_label, span)?;
    }

    let mut it = branches.into_iter().peekable();

    while let Some((branch, label, scope)) = it.next() {
        cx.asm.label(&label)?;

        let scope = cx.scopes.restore(scope);

        let asm = if hir.fallback.is_none() {
            let asm = block(cx, &branch.block, &mut Any::ignore(branch))?;

            if asm.converging() {
                if let Some(out) = output_addr {
                    cx.asm.push(Inst::unit(out), span)?;
                }

                Asm::new(span, ())
            } else {
                Asm::diverge(span)
            }
        } else {
            block(cx, &branch.block, needs)?
        };

        cx.scopes.pop(branch, scope)?;

        if !asm.converging() && it.peek().is_some() {
            cx.asm.jump(&end_label, branch)?;
        }
    }

    cx.asm.label(&end_label)?;
    linear.free()?;
    Ok(Asm::new(span, ()))
}

/// Assemble an expression.
#[instrument(span = span)]
fn expr_index<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ExprIndex<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let mut target = cx.scopes.defer(span);
    let mut index = cx.scopes.defer(span);

    if let Some([target, index]) = expr_array(
        cx,
        span,
        [(&hir.target, &mut target), (&hir.index, &mut index)],
    )?
    .into_converging()
    {
        cx.asm.push(
            Inst::IndexGet {
                index: index.addr(),
                target: target.addr(),
                out: needs.alloc_output()?,
            },
            span,
        )?;
    }

    index.free()?;
    target.free()?;
    Ok(Asm::new(span, ()))
}

/// Assemble a let expression.
#[instrument(span = hir)]
fn expr_let<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ExprLet<'hir>,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let mut load =
        |cx: &mut Ctxt<'a, 'hir, '_>, needs: &mut dyn Needs<'a, 'hir>| expr(cx, &hir.expr, needs);

    converge!(pattern_panic(cx, &hir.pat, move |cx, false_label| {
        pat_binding(cx, &hir.pat, false_label, &mut load)
    })?);

    // If a value is needed for a let expression, it is evaluated as a unit.
    if let Some(out) = needs.try_alloc_output()? {
        cx.asm.push(Inst::unit(out), hir)?;
    }

    Ok(Asm::new(hir, ()))
}

#[instrument(span = span)]
fn expr_match<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ExprMatch<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let mut value = cx.scopes.defer(span);
    converge!(expr(cx, &hir.expr, &mut value)?, free(value));
    let value = value.into_addr()?;

    let end_label = cx.asm.new_label("match_end");
    let mut branches = Vec::new();

    let count = hir
        .branches
        .iter()
        .map(|b| b.pat.names.len())
        .max()
        .unwrap_or_default();

    let mut linear = cx.scopes.linear(span, count)?;
    let mut is_irrefutable = false;

    for branch in hir.branches {
        let span = branch;

        let branch_label = cx.asm.new_label("match_branch");
        let match_false = cx.asm.new_label("match_false");

        let pattern_scope = cx.scopes.child(span)?;

        let mut load = |cx: &mut Ctxt<'a, 'hir, '_>, needs: &mut dyn Needs<'a, 'hir>| {
            needs.assign_addr(cx, value.addr())?;
            Ok(Asm::new(branch, ()))
        };

        let asm = pat_binding_with(
            cx,
            &branch.pat,
            &branch.pat.pat,
            branch.pat.names,
            &match_false,
            &mut load,
            &mut linear,
        )?;

        if let Some(pat) = asm.into_converging() {
            let mut converges = true;

            if let Some(condition) = branch.condition {
                let span = condition;
                let mut cond = cx.scopes.defer(condition);

                let scope = cx.scopes.child(span)?;

                if expr(cx, condition, &mut cond)?.converging() {
                    cx.asm
                        .jump_if_not(cond.addr()?.addr(), &match_false, span)?;
                    cx.asm.jump(&branch_label, span)?;
                } else {
                    converges = false;
                }

                cond.free()?;
                cx.scopes.pop(span, scope)?;
            } else {
                // If there is no branch condition, and the branch is
                // irrefutable, there is no point in assembling the additional
                // branches.
                is_irrefutable = matches!(pat, Pattern::Irrefutable);
            }

            if converges {
                cx.asm.jump(&branch_label, span)?;
                let pattern_scope = cx.scopes.dangle(span, pattern_scope)?;
                branches.try_push((branch_label, pattern_scope))?;
            } else {
                // If the branch condition diverges, there is no reason to
                // assemble the other branches if this one is irrefutable.
                is_irrefutable = matches!(pat, Pattern::Irrefutable);
                cx.scopes.pop(span, pattern_scope)?;
            }
        }

        if is_irrefutable {
            break;
        }

        cx.asm.label(&match_false)?;
    }

    if !is_irrefutable {
        if let Some(out) = needs.try_alloc_output()? {
            cx.asm.push(Inst::unit(out), span)?;
        }

        cx.asm.jump(&end_label, span)?;
    }

    let mut it = hir.branches.iter().zip(branches).peekable();

    while let Some((branch, (label, scope))) = it.next() {
        let span = branch;

        cx.asm.label(&label)?;
        let scope = cx.scopes.restore(scope);

        if expr(cx, &branch.body, needs)?.converging() && it.peek().is_some() {
            cx.asm.jump(&end_label, span)?;
        }

        cx.scopes.pop(span, scope)?;
    }

    cx.asm.label(&end_label)?;

    value.free()?;
    linear.free()?;
    Ok(Asm::new(span, ()))
}

/// Compile a literal object.
#[instrument(span = span)]
fn expr_object<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::ExprObject<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    if let Some(linear) =
        exprs_with(cx, span, hir.assignments, |hir| &hir.assign)?.into_converging()
    {
        let slot =
            cx.q.unit
                .new_static_object_keys_iter(span, hir.assignments.iter().map(|a| a.key.1))?;

        match hir.kind {
            hir::ExprObjectKind::EmptyStruct { hash } => {
                cx.asm.push(
                    Inst::EmptyStruct {
                        hash,
                        out: needs.alloc_output()?,
                    },
                    span,
                )?;
            }
            hir::ExprObjectKind::Struct { hash } => {
                cx.asm.push(
                    Inst::Struct {
                        addr: linear.addr(),
                        hash,
                        slot,
                        out: needs.alloc_output()?,
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
                        out: needs.alloc_output()?,
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
                        out: needs.alloc_output()?,
                    },
                    span,
                )?;
            }
            hir::ExprObjectKind::Anonymous => {
                cx.asm.push(
                    Inst::Object {
                        addr: linear.addr(),
                        slot,
                        out: needs.alloc_output()?,
                    },
                    span,
                )?;
            }
        }

        linear.free()?;
    }

    Ok(Asm::new(span, ()))
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
fn expr_range<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ExprRange<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let a: Option<&hir::Expr<'hir>>;
    let b: Option<&hir::Expr<'hir>>;

    let range = match hir {
        hir::ExprRange::RangeFrom { start } => {
            a = Some(start);
            b = None;
            InstRange::RangeFrom
        }
        hir::ExprRange::RangeFull => {
            a = None;
            b = None;
            InstRange::RangeFull
        }
        hir::ExprRange::RangeInclusive { start, end } => {
            a = Some(start);
            b = Some(end);
            InstRange::RangeInclusive
        }
        hir::ExprRange::RangeToInclusive { end } => {
            a = Some(end);
            b = None;
            InstRange::RangeToInclusive
        }
        hir::ExprRange::RangeTo { end } => {
            a = Some(end);
            b = None;
            InstRange::RangeTo
        }
        hir::ExprRange::Range { start, end } => {
            a = Some(start);
            b = Some(end);
            InstRange::Range
        }
    };

    let a = a.map(slice::from_ref).unwrap_or_default();
    let b = b.map(slice::from_ref).unwrap_or_default();

    if let Some(linear) = exprs_2(cx, span, a, b)?.into_converging() {
        if let Some(out) = needs.try_alloc_output()? {
            cx.asm.push(
                Inst::Range {
                    addr: linear.addr(),
                    range,
                    out,
                },
                span,
            )?;
        }

        linear.free()?;
    }

    Ok(Asm::new(span, ()))
}

/// Assemble a return expression.
#[instrument(span = span)]
fn expr_return<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: Option<&'hir hir::Expr<'hir>>,
    span: &'hir dyn Spanned,
) -> compile::Result<Asm<'hir>> {
    if let Some(e) = hir {
        converge!(return_(cx, span, e, expr)?);
    } else {
        cx.asm.push(Inst::ReturnUnit, span)?;
    }

    Ok(Asm::diverge(span))
}

/// Assemble a select expression.
fn expr_select_inner<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::ExprSelect<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let mut default_branch = None;

    let end_label = cx.asm.new_label("select_end");

    for branch in hir.branches {
        let label = cx.asm.new_label("select_branch");
        cx.select_branches.try_push((label, branch))?;
    }

    if let Some(def) = hir.default {
        let label = cx.asm.new_label("select_default");
        default_branch = Some((def, label));
    }

    let linear = converge!(exprs(cx, span, hir.exprs)?);

    let mut value_addr = cx.scopes.alloc(span)?;

    let select_label = cx.asm.new_label("select");
    cx.asm.label(&select_label)?;

    cx.asm.push(
        Inst::Select {
            addr: linear.addr(),
            len: hir.exprs.len(),
            value: value_addr.output(),
        },
        span,
    )?;

    for (label, _) in &cx.select_branches {
        cx.asm.jump(label, span)?;
    }

    if let Some((_, label)) = &default_branch {
        cx.asm.jump(label, span)?;
    } else {
        if let Some(out) = needs.try_alloc_output()? {
            cx.asm.push(
                Inst::Copy {
                    addr: value_addr.addr(),
                    out,
                },
                span,
            )?;
        }

        if !cx.select_branches.is_empty() || default_branch.is_some() {
            cx.asm.jump(&end_label, span)?;
        }
    }

    let mut branches = take(&mut cx.select_branches);

    for (label, branch) in branches.drain(..) {
        cx.asm.label(&label)?;

        let scope = cx.scopes.child(&branch.body)?;

        if fn_arg_pat(cx, &branch.pat, &mut value_addr, &select_label)?.converging()
            && expr(cx, &branch.body, needs)?.converging()
        {
            cx.asm.jump(&end_label, span)?;
        }

        cx.scopes.pop(&branch.body, scope)?;
    }

    cx.select_branches = branches;

    if let Some((branch, label)) = default_branch {
        cx.asm.label(&label)?;
        expr(cx, branch, needs)?.ignore();
    }

    cx.asm.label(&end_label)?;

    // Drop futures we are currently using.
    for addr in &linear {
        cx.asm.push(Inst::Drop { addr: addr.addr() }, span)?;
    }

    value_addr.free()?;
    linear.free()?;
    Ok(Asm::new(span, ()))
}

#[instrument(span = span)]
fn expr_select<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::ExprSelect<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    cx.contexts.try_push(span.span())?;
    cx.select_branches.clear();

    let asm = expr_select_inner(cx, hir, span, needs)?;

    cx.contexts
        .pop()
        .ok_or("Missing parent context")
        .with_span(span)?;

    Ok(asm)
}

/// Assemble a try expression.
#[instrument(span = span)]
fn expr_try<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::Expr<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let mut e = cx.scopes.defer(span);
    converge!(expr(cx, hir, &mut e)?);

    cx.asm.push(
        Inst::Try {
            addr: e.addr()?.addr(),
            out: needs.alloc_output()?,
        },
        span,
    )?;

    e.free()?;
    Ok(Asm::new(span, ()))
}

/// Assemble a literal tuple.
#[instrument(span = span)]
fn expr_tuple<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::ExprSeq<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    macro_rules! tuple {
        ($variant:ident $(, $var:ident, $expr:ident)* $(,)?) => {{
            $(let mut $var = cx.scopes.defer(span);)*

            let asm = expr_array(cx, span, [$(($expr, &mut $var)),*])?;

            let [$($expr),*] = converge!(asm, free($($var),*));

            cx.asm.push(
                Inst::$variant {
                    args: [$($expr.addr(),)*],
                    out: needs.alloc_output()?,
                },
                span,
            )?;

            $($var.free()?;)*
        }};
    }

    match hir.items {
        [] => {
            cx.asm.push(Inst::unit(needs.alloc_output()?), span)?;
        }
        [e1] => tuple!(Tuple1, v1, e1),
        [e1, e2] => tuple!(Tuple2, v1, e1, v2, e2),
        [e1, e2, e3] => tuple!(Tuple3, v1, e1, v2, e2, v3, e3),
        [e1, e2, e3, e4] => tuple!(Tuple4, v1, e1, v2, e2, v3, e3, v4, e4),
        _ => {
            let linear = converge!(exprs(cx, span, hir.items)?);

            if let Some(out) = needs.try_alloc_output()? {
                cx.asm.push(
                    Inst::Tuple {
                        addr: linear.addr(),
                        count: hir.items.len(),
                        out,
                    },
                    span,
                )?;
            }

            linear.free()?;
        }
    }

    Ok(Asm::new(span, ()))
}

/// Assemble a unary expression.
#[instrument(span = span)]
fn expr_unary<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ExprUnary<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    converge!(expr(cx, &hir.expr, needs)?);

    let Some(addr) = needs.try_as_addr()? else {
        return Ok(Asm::new(span, ()));
    };

    match hir.op {
        ast::UnOp::Not(..) => {
            cx.asm.push(
                Inst::Not {
                    addr: addr.addr(),
                    out: addr.output(),
                },
                span,
            )?;
        }
        ast::UnOp::Neg(..) => {
            cx.asm.push(
                Inst::Neg {
                    addr: addr.addr(),
                    out: addr.output(),
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

    Ok(Asm::new(span, ()))
}

/// Assemble a literal vector.
#[instrument(span = span)]
fn expr_vec<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::ExprSeq<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let mut linear = cx.scopes.linear(span, hir.items.len())?;
    let count = hir.items.len();

    for (e, needs) in hir.items.iter().zip(&mut linear) {
        converge!(expr(cx, e, needs)?, free(linear));
    }

    if let Some(out) = needs.try_alloc_addr()? {
        cx.asm.push(
            Inst::Vec {
                addr: linear.addr(),
                count,
                out: out.output(),
            },
            span,
        )?;
    }

    linear.free()?;
    Ok(Asm::new(span, ()))
}

/// Assemble a while loop.
#[instrument(span = span)]
fn expr_loop<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ExprLoop<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let continue_label = cx.asm.new_label("while_continue");
    let then_label = cx.asm.new_label("while_then");
    let end_label = cx.asm.new_label("while_end");
    let break_label = cx.asm.new_label("while_break");

    cx.breaks.push(Break {
        label: hir.label,
        continue_label: Some(continue_label.try_clone()?),
        break_label: break_label.try_clone()?,
        output: Some(needs.alloc_output()?),
        drop: None,
    })?;

    cx.asm.label(&continue_label)?;

    let count = hir.condition.and_then(|c| c.count()).unwrap_or_default();
    let mut linear = cx.scopes.linear(span, count)?;

    let condition_scope = if let Some(hir) = hir.condition {
        if let Some((scope, _)) =
            condition(cx, hir, &then_label, &end_label, &mut linear)?.into_converging()
        {
            cx.asm.jump(&end_label, span)?;
            cx.asm.label(&then_label)?;
            Some(scope)
        } else {
            None
        }
    } else {
        None
    };

    // Divergence should be ignored, since there are labels which might jump over it.
    block(cx, &hir.body, &mut Any::ignore(span))?.ignore();

    if let Some(scope) = condition_scope {
        cx.scopes.pop(span, scope)?;
    }

    cx.asm.jump(&continue_label, span)?;
    cx.asm.label(&end_label)?;

    if let Some(out) = needs.try_alloc_output()? {
        cx.asm.push(Inst::unit(out), span)?;
    }

    // NB: breaks produce their own value / perform their own cleanup.
    cx.asm.label(&break_label)?;
    linear.free()?;
    cx.breaks.pop();
    Ok(Asm::new(span, ()))
}

/// Assemble a `yield` expression.
#[instrument(span = span)]
fn expr_yield<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: Option<&'hir hir::Expr<'hir>>,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let out = needs.alloc_output()?;

    if let Some(e) = hir {
        let mut addr = cx.scopes.alloc(span)?.with_name("yield argument");
        converge!(expr(cx, e, &mut addr)?, free(addr));

        cx.asm.push(
            Inst::Yield {
                addr: addr.addr(),
                out,
            },
            span,
        )?;

        addr.free()?;
    } else {
        cx.asm.push(Inst::YieldUnit { out }, span)?;
    }

    Ok(Asm::new(span, ()))
}

/// Assemble a literal value.
#[instrument(span = span)]
fn lit<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: hir::Lit<'_>,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    // Elide the entire literal if it's not needed.
    let Some(addr) = needs.try_alloc_addr()? else {
        cx.q.diagnostics
            .not_used(cx.source_id, span, cx.context())?;
        return Ok(Asm::new(span, ()));
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

    Ok(Asm::new(span, ()))
}

/// Assemble a local expression.
#[instrument(span = hir)]
fn local<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::Local<'hir>,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let mut load =
        |cx: &mut Ctxt<'a, 'hir, '_>, needs: &mut dyn Needs<'a, 'hir>| expr(cx, &hir.expr, needs);

    converge!(pattern_panic(cx, &hir.pat, |cx, false_label| {
        pat_binding(cx, &hir.pat, false_label, &mut load)
    })?);

    // If a value is needed for a let expression, it is evaluated as a unit.
    if let Some(out) = needs.try_alloc_output()? {
        cx.asm.push(Inst::unit(out), hir)?;
    }

    Ok(Asm::new(hir, ()))
}
