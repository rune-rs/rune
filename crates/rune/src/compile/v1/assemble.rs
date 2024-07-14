use core::fmt;
use core::slice;

use crate::alloc::prelude::*;
use crate::alloc::BTreeMap;
use crate::ast::{self, Span, Spanned};
use crate::compile::ir;
use crate::compile::{self, Assembly, ErrorKind, ItemId, ModId, Options, WithSpan};
use crate::hir;
use crate::query::{ConstFn, Query, Used};
use crate::runtime::{
    ConstValue, Inst, InstAddress, InstAssignOp, InstOp, InstRange, InstTarget, InstValue,
    InstVariant, Label, Output, PanicReason, Protocol, TypeCheck,
};
use crate::shared::FixedVec;
use crate::{Hash, SourceId};

use super::{Linear, Loop, Loops, Needs, NeedsAddress, ScopeId, Scopes};

use rune_macros::instrument;

macro_rules! converge {
    ($expr:expr $(, $method:ident($cx:expr $(, $needs:expr)* $(,)?))?) => {
        match $expr {
            Asm {
                outcome: Outcome::Converge(data),
                ..
            } => data,
            Asm {
                span,
                outcome: Outcome::Diverge,
            } => {
                $(
                    $($needs.$method($cx.scopes)?;)*
                )*

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

trait NeedsLike<'hir> {
    /// Access the span for the needs.
    fn span(&self) -> &'hir dyn Spanned;

    /// Get output of the needs or error.
    fn output(&self) -> compile::Result<Output>;

    /// Get the need as an output.
    ///
    /// Returns `None` if `Needs::None` is set.
    fn try_alloc_output(&mut self, cx: &mut Ctxt<'_, 'hir, '_>) -> compile::Result<Option<Output>>;

    fn assign_addr(
        &mut self,
        cx: &mut Ctxt<'_, 'hir, '_>,
        from: InstAddress,
    ) -> compile::Result<()>;

    /// Forcibly allocate as an output, if possible.
    fn alloc_output(&mut self, scopes: &Scopes<'hir>) -> compile::Result<Output>;

    /// Try to allocate an address from this needs.
    fn try_alloc_addr(
        &mut self,
        cx: &mut Ctxt<'_, 'hir, '_>,
    ) -> compile::Result<Option<InstAddress>>;

    /// Get the need as an address.
    fn as_addr(&self) -> Option<&NeedsAddress<'hir>>;

    /// If the needs has a value.
    fn value(&self) -> bool;
}

impl<'hir> NeedsLike<'hir> for Needs<'hir> {
    #[inline]
    fn span(&self) -> &'hir dyn Spanned {
        self.span
    }

    #[inline]
    fn output(&self) -> compile::Result<Output> {
        Needs::output(self)
    }

    #[inline]
    fn assign_addr(
        &mut self,
        cx: &mut Ctxt<'_, 'hir, '_>,
        from: InstAddress,
    ) -> compile::Result<()> {
        Needs::assign_addr(self, cx, from)
    }

    #[inline]
    fn alloc_output(&mut self, scopes: &Scopes<'hir>) -> compile::Result<Output> {
        Needs::alloc_output(self, scopes)
    }

    #[inline]
    fn try_alloc_output(&mut self, cx: &mut Ctxt<'_, 'hir, '_>) -> compile::Result<Option<Output>> {
        Needs::try_alloc_output(self, cx.scopes)
    }

    #[inline]
    fn try_alloc_addr(
        &mut self,
        cx: &mut Ctxt<'_, 'hir, '_>,
    ) -> compile::Result<Option<InstAddress>> {
        Needs::try_alloc_addr(self, cx.scopes)
    }

    #[inline]
    fn as_addr(&self) -> Option<&NeedsAddress<'hir>> {
        Needs::as_addr(self)
    }

    #[inline]
    fn value(&self) -> bool {
        Needs::value(self)
    }
}

impl<'hir> NeedsLike<'hir> for NeedsAddress<'hir> {
    #[inline]
    fn span(&self) -> &'hir dyn Spanned {
        self.span
    }

    #[inline]
    fn output(&self) -> compile::Result<Output> {
        Ok(NeedsAddress::output(self))
    }

    #[inline]
    fn assign_addr(
        &mut self,
        cx: &mut Ctxt<'_, 'hir, '_>,
        from: InstAddress,
    ) -> compile::Result<()> {
        NeedsAddress::assign_addr(self, cx, from)
    }

    #[inline]
    fn alloc_output(&mut self, _: &Scopes<'hir>) -> compile::Result<Output> {
        NeedsAddress::alloc_output(self)
    }

    #[inline]
    fn try_alloc_output(&mut self, _: &mut Ctxt<'_, 'hir, '_>) -> compile::Result<Option<Output>> {
        Ok(Some(NeedsAddress::alloc_output(self)?))
    }

    #[inline]
    fn try_alloc_addr(
        &mut self,
        _: &mut Ctxt<'_, 'hir, '_>,
    ) -> compile::Result<Option<InstAddress>> {
        Ok(Some(NeedsAddress::alloc_addr(self)?))
    }

    #[inline]
    fn as_addr(&self) -> Option<&NeedsAddress<'hir>> {
        Some(self)
    }

    #[inline]
    fn value(&self) -> bool {
        true
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
    pub(crate) scopes: &'a Scopes<'hir>,
    /// Context for which to emit warnings.
    pub(crate) contexts: Vec<Span>,
    /// The nesting of loop we are currently in.
    pub(crate) loops: Loops<'hir>,
    /// Enabled optimizations.
    pub(crate) options: &'a Options,
    /// Work buffer for select branches.
    pub(crate) select_branches: Vec<(Label, &'hir hir::ExprSelectBranch<'hir>)>,
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
    fn converge(self) -> Option<T> {
        match self.outcome {
            Outcome::Converge(data) => Some(data),
            Outcome::Diverge => None,
        }
    }

    /// Test if the assembly diverges.
    #[inline]
    fn is_diverging(self) -> bool {
        matches!(self.outcome, Outcome::Diverge)
    }

    /// Test if the assembly converges.
    #[inline]
    fn is_converging(self) -> bool {
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

                cx.scopes.define(span, hir::Name::SelfValue, needs.addr())?;
            }
            hir::FnArg::Pat(pat) => {
                pattern_panic(cx, pat, move |cx, false_label| {
                    fn_arg_pat(cx, *pat, needs, false_label)
                })?;
            }
        }

        first = false;
    }

    if hir.body.value.is_some() {
        return_(cx, hir, &hir.body, block)?.ignore();
    } else {
        let mut needs = Needs::none(&hir.body);

        if let Some(()) = block_without_scope(cx, &hir.body, &mut needs)?.converge() {
            cx.asm.push(Inst::ReturnUnit, hir)?;
        }
    }

    cx.scopes.free_linear(arguments)?;
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
        cx.scopes.define(&hir.block, name, needs.addr())?;
    }

    return_(cx, &hir.block, &hir.block, block_without_scope)?;
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
            cx.scopes.define(span, capture, needs.addr())?;
        }
    }

    for (arg, needs) in hir.args.iter().zip(&mut arguments) {
        match arg {
            hir::FnArg::SelfValue(span) => {
                return Err(compile::Error::new(span, ErrorKind::UnsupportedSelf))
            }
            hir::FnArg::Pat(pat) => {
                pattern_panic(cx, pat, move |cx, false_label| {
                    fn_arg_pat(cx, *pat, needs, false_label)
                })?;
            }
        }
    }

    return_(cx, span, &hir.body, expr)?;
    cx.scopes.pop_last(span)?;
    Ok(())
}

#[instrument(span = pat)]
fn fn_arg_pat<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    pat: &'hir hir::PatBinding<'hir>,
    needs: &mut dyn NeedsLike<'hir>,
    false_label: &Label,
) -> compile::Result<Asm<'hir, Pattern>> {
    let Some(addr) = needs.as_addr() else {
        return Err(compile::Error::msg(
            needs.span(),
            "Expected need to be populated outside of pattern",
        ));
    };

    let addr = addr.addr();

    let load = move |cx: &mut Ctxt<'a, 'hir, '_>, needs: &mut dyn NeedsLike<'hir>| {
        needs.assign_addr(cx, addr)?;
        Ok(Asm::new(pat, ()))
    };

    let out = match pat.names {
        [name] => pat_binding_with_single(cx, pat, &pat.pat, *name, false_label, &load, needs)?,
        _ => pat_binding(cx, pat, &false_label, &load)?,
    };

    Ok(out)
}

/// Assemble a return statement from the given Assemble.
fn return_<'a, 'hir, T>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    span: &'hir dyn Spanned,
    hir: T,
    asm: impl FnOnce(&mut Ctxt<'a, 'hir, '_>, T, &mut dyn NeedsLike<'hir>) -> compile::Result<Asm<'hir>>,
) -> compile::Result<Asm<'hir>> {
    let mut needs = Needs::alloc(cx, span);
    converge!(asm(cx, hir, &mut needs)?, free(cx, needs));

    if let Some(addr) = needs.as_addr() {
        cx.asm.push(Inst::Return { addr: addr.addr() }, span)?;
    };

    needs.free(cx.scopes)?;
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

/// Compile a pattern with bindings based on the given offset.
#[instrument(span = hir)]
fn pat_binding_with_addr<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::PatBinding<'hir>,
    addr: InstAddress,
) -> compile::Result<()> {
    let load = |cx: &mut Ctxt<'a, 'hir, '_>, needs: &mut dyn NeedsLike<'hir>| {
        needs.assign_addr(cx, addr)?;
        Ok(Asm::new(hir, ()))
    };

    pattern_panic(cx, hir, |cx, false_label| {
        pat_binding(cx, hir, &false_label, &load)
    })?;

    Ok(())
}

/// Encode a pattern from a known set of bindings.
///
/// Returns a boolean indicating if the label was used.
#[instrument(span = hir)]
fn pat_binding<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::PatBinding<'hir>,
    false_label: &Label,
    load: &dyn Fn(&mut Ctxt<'a, 'hir, '_>, &mut dyn NeedsLike<'hir>) -> compile::Result<Asm<'hir>>,
) -> compile::Result<Asm<'hir, Pattern>> {
    let mut linear = cx.scopes.linear(hir, hir.names.len())?;
    pat_binding_with(cx, hir, &hir.pat, hir.names, false_label, load, &mut linear)
}

#[instrument(span = span)]
fn pat_binding_with<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    span: &'hir dyn Spanned,
    pat: &'hir hir::Pat<'hir>,
    names: &[hir::Name<'hir>],
    false_label: &Label,
    load: &dyn Fn(&mut Ctxt<'a, 'hir, '_>, &mut dyn NeedsLike<'hir>) -> compile::Result<Asm<'hir>>,
    linear: &mut [NeedsAddress<'hir>],
) -> compile::Result<Asm<'hir, Pattern>> {
    let bound;

    {
        let mut bindings = BTreeMap::<_, &mut dyn NeedsLike<'hir>>::new();

        for (name, needs) in names.iter().copied().zip(linear.iter_mut()) {
            bindings.try_insert(name, needs).with_span(span)?;
        }

        bound = self::pat(cx, pat, false_label, load, &mut bindings)?;

        if !bindings.is_empty() {
            let names = bindings.keys().try_collect::<Vec<_>>()?;

            return Err(compile::Error::msg(
                span,
                format!("Unbound names in pattern: {names:?}"),
            ));
        }
    }

    for (name, needs) in names.iter().copied().zip(linear.iter()) {
        cx.scopes.define(needs.span, name, needs.addr())?;
    }

    Ok(bound)
}

#[instrument(span = span)]
fn pat_binding_with_single<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    span: &'hir dyn Spanned,
    pat: &'hir hir::Pat<'hir>,
    name: hir::Name<'hir>,
    false_label: &Label,
    load: &dyn Fn(&mut Ctxt<'a, 'hir, '_>, &mut dyn NeedsLike<'hir>) -> compile::Result<Asm<'hir>>,
    needs: &mut dyn NeedsLike<'hir>,
) -> compile::Result<Asm<'hir, Pattern>> {
    let asm;

    {
        let mut bindings = BTreeMap::<_, &mut dyn NeedsLike<'hir>>::new();
        bindings.try_insert(name, needs).with_span(span)?;

        asm = self::pat(cx, pat, false_label, load, &mut bindings)?;

        if !bindings.is_empty() {
            let names = bindings.keys().try_collect::<Vec<_>>()?;

            return Err(compile::Error::msg(
                span,
                format!("Unbound names in pattern: {names:?}"),
            ));
        }
    }

    let Some(addr) = needs.as_addr() else {
        return Err(compile::Error::msg(
            needs.span(),
            "Expected need to be populated by pattern",
        ));
    };

    cx.scopes.define(needs.span(), name, addr.addr())?;
    Ok(asm)
}

/// Encode a pattern.
///
/// Returns a boolean indicating if the label was used.
#[instrument(span = hir)]
fn pat<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::Pat<'hir>,
    false_label: &Label,
    load: &dyn Fn(&mut Ctxt<'a, 'hir, '_>, &mut dyn NeedsLike<'hir>) -> compile::Result<Asm<'hir>>,
    bindings: &mut BTreeMap<hir::Name<'hir>, &mut dyn NeedsLike<'hir>>,
) -> compile::Result<Asm<'hir, Pattern>> {
    let span = hir;

    match hir.kind {
        hir::PatKind::Ignore => {
            // ignore binding, but might still have effects, so must call load.
            converge!(load(cx, &mut Needs::none(hir))?);
            Ok(Asm::new(span, Pattern::Irrefutable))
        }
        hir::PatKind::Path(kind) => match *kind {
            hir::PatPathKind::Kind(kind) => {
                let mut needs = Needs::alloc(cx, hir);
                converge!(load(cx, &mut needs)?, free(cx, needs));
                let addr = needs.addr()?;

                let cond = cx.scopes.alloc(hir)?;

                cx.asm.push(
                    pat_sequence_kind_to_inst(*kind, addr.addr(), cond.output()),
                    hir,
                )?;

                cx.asm.jump_if_not(cond.addr(), false_label, hir)?;
                cx.scopes.free(cond)?;
                addr.free(cx.scopes)?;
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
    load: &dyn Fn(&mut Ctxt<'a, 'hir, '_>, &mut dyn NeedsLike<'hir>) -> compile::Result<Asm<'hir>>,
) -> compile::Result<Asm<'hir, Pattern>> {
    let mut needs = Needs::alloc(cx, hir);
    converge!(load(cx, &mut needs)?, free(cx, needs));

    let addr = needs.addr()?;
    let cond = cx.scopes.alloc(hir)?;

    let Some(inst) = pat_lit_inst(cx, hir, addr.addr(), cond.addr())? else {
        return Err(compile::Error::new(hir, ErrorKind::UnsupportedPatternExpr));
    };

    cx.asm.push(inst, hir)?;
    cx.asm.jump_if_not(cond.addr(), false_label, hir)?;
    cx.scopes.free(cond)?;
    addr.free(cx.scopes)?;
    Ok(Asm::new(hir, Pattern::Refutable))
}

#[instrument(span = hir)]
fn pat_lit_inst<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::Expr<'_>,
    addr: InstAddress,
    cond: InstAddress,
) -> compile::Result<Option<Inst>> {
    let hir::ExprKind::Lit(lit) = hir.kind else {
        return Ok(None);
    };

    let out = cond.output();

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
#[instrument(span = hir)]
fn condition<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::Condition<'hir>,
    then_label: &Label,
    false_label: &Label,
    linear: &mut [NeedsAddress<'hir>],
) -> compile::Result<Asm<'hir, (ScopeId, Pattern)>> {
    match *hir {
        hir::Condition::Expr(hir) => {
            let scope = cx.scopes.child(hir)?;
            let mut addr = cx.scopes.alloc(hir)?;

            if let Some(()) = expr(cx, hir, &mut addr)?.converge() {
                cx.asm.jump_if(addr.addr(), then_label, hir)?;
                Ok(Asm::new(hir, (scope, Pattern::Irrefutable)))
            } else {
                cx.scopes.pop(hir, scope)?;
                Ok(Asm::diverge(hir))
            }
        }
        hir::Condition::ExprLet(hir) => {
            let span = hir;

            let scope = cx.scopes.child(span)?;

            let load = |cx: &mut Ctxt<'a, 'hir, '_>, needs: &mut dyn NeedsLike<'hir>| {
                expr(cx, &hir.expr, needs)
            };

            let asm = pat_binding_with(
                cx,
                &hir.pat,
                &hir.pat.pat,
                &hir.pat.names,
                &false_label,
                &load,
                linear,
            )?;

            if let Some(pat) = asm.converge() {
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
    load: &dyn Fn(&mut Ctxt<'a, 'hir, '_>, &mut dyn NeedsLike<'hir>) -> compile::Result<Asm<'hir>>,
    bindings: &mut BTreeMap<hir::Name<'hir>, &mut dyn NeedsLike<'hir>>,
) -> compile::Result<Asm<'hir, Pattern>> {
    let mut needs = Needs::alloc(cx, span);
    converge!(load(cx, &mut needs)?, free(cx, needs));

    let addr = needs.addr()?;
    let cond = cx.scopes.alloc(span)?;

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
        return Ok(Asm::new(span, Pattern::Refutable));
    }

    let inst = pat_sequence_kind_to_inst(hir.kind, addr.addr(), cond.output());
    cx.asm.push(inst, span)?;
    cx.asm.jump_if_not(cond.addr(), false_label, span)?;

    for (index, p) in hir.items.iter().enumerate() {
        let load = move |cx: &mut Ctxt<'a, 'hir, '_>, n: &mut dyn NeedsLike<'hir>| {
            cx.asm.push(
                Inst::TupleIndexGetAt {
                    addr: addr.addr(),
                    index,
                    out: n.alloc_output(cx.scopes)?,
                },
                p,
            )?;
            Ok(Asm::new(p, ()))
        };

        converge!(
            self::pat(cx, p, false_label, &load, bindings)?,
            free(cx, cond, addr)
        );
    }

    cond.free(cx.scopes)?;
    addr.free(cx.scopes)?;
    Ok(Asm::new(span, Pattern::Refutable))
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
fn pat_object<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::PatObject<'hir>,
    span: &'hir dyn Spanned,
    false_label: &Label,
    load: &dyn Fn(&mut Ctxt<'a, 'hir, '_>, &mut dyn NeedsLike<'hir>) -> compile::Result<Asm<'hir>>,
    bindings: &mut BTreeMap<hir::Name<'hir>, &mut dyn NeedsLike<'hir>>,
) -> compile::Result<Asm<'hir, Pattern>> {
    let mut needs = Needs::alloc(cx, span);
    converge!(load(cx, &mut needs)?, free(cx, needs));

    let addr = needs.addr()?;

    let cond = cx.scopes.alloc(span)?;

    let mut string_slots = Vec::new();

    for binding in hir.bindings {
        string_slots.try_push(cx.q.unit.new_static_string(span, binding.key())?)?;
    }

    let inst = match hir.kind {
        hir::PatSequenceKind::Type { hash } => Inst::MatchType {
            hash,
            addr: addr.addr(),
            out: cond.output(),
        },
        hir::PatSequenceKind::BuiltInVariant { type_check } => Inst::MatchBuiltIn {
            type_check,
            addr: addr.addr(),
            out: cond.output(),
        },
        hir::PatSequenceKind::Variant {
            variant_hash,
            enum_hash,
            index,
        } => Inst::MatchVariant {
            variant_hash,
            enum_hash,
            index,
            addr: addr.addr(),
            out: cond.output(),
        },
        hir::PatSequenceKind::Anonymous { is_open, .. } => {
            let keys =
                cx.q.unit
                    .new_static_object_keys_iter(span, hir.bindings.iter().map(|b| b.key()))?;

            Inst::MatchObject {
                slot: keys,
                exact: !is_open,
                addr: addr.addr(),
                out: cond.output(),
            }
        }
    };

    // Copy the temporary and check that its length matches the pattern and
    // that it is indeed a vector.
    cx.asm.push(inst, span)?;
    cx.asm.jump_if_not(cond.addr(), false_label, span)?;
    cx.scopes.free(cond)?;

    for (binding, slot) in hir.bindings.iter().zip(string_slots) {
        match binding {
            hir::Binding::Binding(span, _, p) => {
                let load = move |cx: &mut Ctxt<'a, 'hir, '_>, needs: &mut dyn NeedsLike<'hir>| {
                    cx.asm.push(
                        Inst::ObjectIndexGetAt {
                            addr: addr.addr(),
                            slot,
                            out: needs.alloc_output(cx.scopes)?,
                        },
                        span,
                    )?;
                    Ok(Asm::new(span, ()))
                };

                converge!(
                    self::pat(cx, p, false_label, &load, bindings)?,
                    free(cx, addr)
                );
            }
            hir::Binding::Ident(span, name) => {
                let addr = addr.addr();
                let name = hir::Name::Str(name);

                let Some(binding) = bindings.remove(&name) else {
                    return Err(compile::Error::msg(
                        binding,
                        format!("No binding for {name:?}"),
                    ));
                };

                cx.asm.push(
                    Inst::ObjectIndexGetAt {
                        addr,
                        slot,
                        out: binding.output()?,
                    },
                    &span,
                )?;
            }
        }
    }

    addr.free(cx.scopes)?;
    Ok(Asm::new(span, Pattern::Refutable))
}

/// Call a block.
#[instrument(span = hir)]
fn block<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::Block<'hir>,
    needs: &mut dyn NeedsLike<'hir>,
) -> compile::Result<Asm<'hir>> {
    let scope = cx.scopes.child(hir)?;
    let asm = block_without_scope(cx, hir, needs)?;
    cx.scopes.pop(hir, scope)?;
    Ok(asm)
}

/// Call a block.
#[instrument(span = hir)]
fn block_without_scope<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::Block<'hir>,
    needs: &mut dyn NeedsLike<'hir>,
) -> compile::Result<Asm<'hir>> {
    let mut diverge = false;
    cx.contexts.try_push(hir.span())?;

    for stmt in hir.statements {
        let mut needs = Needs::none(hir);

        if diverge {
            // TODO: Mark dead code.
            continue;
        }

        let asm = match stmt {
            hir::Stmt::Local(hir) => local(cx, hir, &mut needs)?,
            hir::Stmt::Expr(hir) => expr(cx, hir, &mut needs)?,
        };

        diverge |= asm.is_diverging();
    }

    if !diverge {
        if let Some(e) = hir.value {
            diverge |= expr(cx, e, needs)?.is_diverging();
        } else if let Some(out) = needs.try_alloc_output(cx)? {
            cx.asm.push(Inst::unit(out), hir)?;
        }
    }

    cx.contexts
        .pop()
        .ok_or("Missing parent context")
        .with_span(hir)?;

    if diverge {
        return Ok(Asm::diverge(hir));
    }

    Ok(Asm::new(hir, ()))
}

/// Assemble #[builtin] format_args!(...) macro.
#[instrument(span = format)]
fn builtin_format<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    format: &'hir hir::BuiltInFormat<'hir>,
    needs: &mut dyn NeedsLike<'hir>,
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

    if let Some(addr) = needs.try_alloc_addr(cx)? {
        cx.asm.push(
            Inst::Format {
                addr,
                spec,
                out: addr.output(),
            },
            format,
        )?;
    }

    Ok(Asm::new(format, ()))
}

/// Assemble #[builtin] template!(...) macro.
#[instrument(span = template)]
fn builtin_template<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    template: &'hir hir::BuiltInTemplate<'hir>,
    needs: &mut dyn NeedsLike<'hir>,
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
                        out: addr.output(),
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

    cx.asm.push(
        Inst::StringConcat {
            addr: linear.addr(),
            len: template.exprs.len(),
            size_hint,
            out: needs.alloc_output(cx.scopes)?,
        },
        span,
    )?;

    cx.scopes.pop(span, expected)?;
    Ok(Asm::new(span, ()))
}

/// Assemble a constant value.
#[instrument(span = span)]
fn const_<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    value: &ConstValue,
    span: &'hir dyn Spanned,
    needs: &mut dyn NeedsLike<'hir>,
) -> compile::Result<()> {
    let Some(addr) = needs.try_alloc_addr(cx)? else {
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
                const_(cx, value, span, &mut Needs::with_local(span, addr))?;
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
fn expr<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::Expr<'hir>,
    needs: &mut dyn NeedsLike<'hir>,
) -> compile::Result<Asm<'hir>> {
    let span = hir;

    let asm = match hir.kind {
        hir::ExprKind::Variable(name) => {
            let var = cx.scopes.get(&mut cx.q, span, name)?;
            needs.assign_addr(cx, var.addr)?;
            Asm::new(span, ())
        }
        hir::ExprKind::Type(ty) => {
            if let Some(out) = needs.try_alloc_output(cx)? {
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
            if let Some(out) = needs.try_alloc_output(cx)? {
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
    needs: &mut dyn NeedsLike<'hir>,
) -> compile::Result<Asm<'hir>> {
    let supported = match hir.lhs.kind {
        // <var> = <value>
        hir::ExprKind::Variable(name) => {
            let var = cx.scopes.get(&mut cx.q, span, name)?;
            let mut needs = NeedsAddress::with_local(span, var.addr);
            expr(cx, &hir.rhs, &mut needs)?;
            true
        }
        // <expr>.<field> = <value>
        hir::ExprKind::FieldAccess(field_access) => {
            // field assignment
            match field_access.expr_field {
                hir::ExprField::Ident(ident) => {
                    let slot = cx.q.unit.new_static_string(span, ident)?;

                    let mut target = Needs::alloc(cx, &field_access.expr);
                    let mut value = Needs::alloc(cx, &hir.rhs);

                    let asm = expr_array(
                        cx,
                        span,
                        [(&field_access.expr, &mut target), (&hir.rhs, &mut value)],
                    )?;

                    if let Some([target, value]) = asm.converge() {
                        cx.asm.push(
                            Inst::ObjectIndexSet {
                                target: target.addr(),
                                slot,
                                value: value.addr(),
                            },
                            span,
                        )?;
                    }

                    target.free(cx.scopes)?;
                    value.free(cx.scopes)?;
                    true
                }
                hir::ExprField::Index(index) => {
                    let mut target = cx.scopes.alloc(span)?;
                    let mut value = cx.scopes.alloc(&hir.rhs)?;

                    expr(cx, &field_access.expr, &mut target)?;
                    expr(cx, &hir.rhs, &mut value)?;

                    cx.asm.push(
                        Inst::TupleIndexSet {
                            target: target.addr(),
                            index,
                            value: value.addr(),
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
                    target: target.addr(),
                    index: index.addr(),
                    value: value.addr(),
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

    if let Some(out) = needs.try_alloc_output(cx)? {
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
    needs: &mut dyn NeedsLike<'hir>,
) -> compile::Result<Asm<'hir>> {
    let mut addr = cx.scopes.alloc(span)?;
    expr(cx, hir, &mut addr)?;

    cx.asm.push(
        Inst::Await {
            addr: addr.addr(),
            out: needs.alloc_output(cx.scopes)?,
        },
        span,
    )?;

    cx.scopes.free(addr)?;
    Ok(Asm::new(span, ()))
}

/// Assemble a binary expression.
#[instrument(span = span)]
fn expr_binary<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ExprBinary<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn NeedsLike<'hir>,
) -> compile::Result<Asm<'hir>> {
    // Special expressions which operates on the stack in special ways.
    if hir.op.is_assign() {
        compile_assign_binop(cx, &hir.lhs, &hir.rhs, &hir.op, span, needs)?;
        return Ok(Asm::new(span, ()));
    }

    if hir.op.is_conditional() {
        compile_conditional_binop(cx, &hir.lhs, &hir.rhs, &hir.op, span, needs)?;
        return Ok(Asm::new(span, ()));
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

    let guard = cx.scopes.child(span)?;

    let mut a = Needs::alloc_in(guard, span)?;
    let mut b = Needs::alloc_in(guard, span)?;

    let asm = expr_array(cx, span, [(&hir.lhs, &mut a), (&hir.rhs, &mut b)])?;

    if let Some([a, b]) = asm.converge() {
        cx.asm.push(
            Inst::Op {
                op,
                a: a.addr(),
                b: b.addr(),
                out: needs.alloc_output(cx.scopes)?,
            },
            span,
        )?;
    }

    a.free(cx.scopes)?;
    b.free(cx.scopes)?;
    cx.scopes.pop(span, guard)?;
    return Ok(Asm::new(span, ()));

    fn compile_conditional_binop<'a, 'hir>(
        cx: &mut Ctxt<'a, 'hir, '_>,
        lhs: &'hir hir::Expr<'hir>,
        rhs: &'hir hir::Expr<'hir>,
        bin_op: &ast::BinOp,
        span: &dyn Spanned,
        needs: &mut dyn NeedsLike<'hir>,
    ) -> compile::Result<()> {
        let end_label = cx.asm.new_label("conditional_end");

        expr(cx, lhs, needs)?;

        let Some(addr) = needs.as_addr() else {
            return Ok(());
        };

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

        expr(cx, rhs, needs)?;
        cx.asm.label(&end_label)?;
        Ok(())
    }

    fn compile_assign_binop<'a, 'hir>(
        cx: &mut Ctxt<'a, 'hir, '_>,
        lhs: &'hir hir::Expr<'hir>,
        rhs: &'hir hir::Expr<'hir>,
        bin_op: &ast::BinOp,
        span: &dyn Spanned,
        needs: &mut dyn NeedsLike<'hir>,
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
                        Some(InstTarget::TupleField(field.addr(), index))
                    }
                    hir::ExprField::Ident(ident) => {
                        let n = cx.q.unit.new_static_string(&field_access.expr, ident)?;
                        Some(InstTarget::Field(field.addr(), n))
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
                value: value.addr(),
            },
            span,
        )?;

        if let Some(out) = needs.try_alloc_output(cx)? {
            cx.asm.push(Inst::unit(out), span)?;
        }

        Ok(())
    }
}

/// Assemble a block expression.
#[instrument(span = span)]
fn expr_async_block<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::ExprAsyncBlock<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn NeedsLike<'hir>,
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
            out: needs.alloc_output(cx.scopes)?,
        },
        span,
        &"async block",
    )?;

    cx.scopes.free_linear(linear)?;
    Ok(Asm::new(span, ()))
}

/// Assemble a constant item.
#[instrument(span = span)]
fn const_item<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hash: Hash,
    span: &'hir dyn Spanned,
    needs: &mut dyn NeedsLike<'hir>,
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
    let Some(current_loop) = cx.loops.last().try_cloned()? else {
        return Err(compile::Error::new(span, ErrorKind::BreakOutsideOfLoop));
    };

    let (last_loop, to_drop, has_value) = match (hir.label, hir.expr) {
        (None, Some(e)) => {
            let mut needs = match current_loop.output.as_addr() {
                Some(addr) => Needs::with_assigned(e, addr),
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
                Some(addr) => Needs::with_local(span, addr),
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
    Ok(Asm::diverge(span))
}

/// Assemble a call expression.
#[instrument(span = span)]
fn expr_call<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::ExprCall<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn NeedsLike<'hir>,
) -> compile::Result<Asm<'hir>> {
    let args = hir.args.len();

    match hir.call {
        hir::Call::Var { name, .. } => {
            let var = cx.scopes.get(&mut cx.q, span, name)?;
            let linear = converge!(exprs(cx, span, hir.args)?);

            cx.asm.push(
                Inst::CallFn {
                    function: var.addr,
                    addr: linear.addr(),
                    args: hir.args.len(),
                    out: needs.alloc_output(cx.scopes)?,
                },
                span,
            )?;

            cx.scopes.free_linear(linear)?;
        }
        hir::Call::Associated { target, hash } => {
            let linear = converge!(exprs_2(cx, span, slice::from_ref(target), hir.args)?);

            cx.asm.push(
                Inst::CallAssociated {
                    hash,
                    addr: linear.addr(),
                    args: args + 1,
                    out: needs.alloc_output(cx.scopes)?,
                },
                span,
            )?;

            cx.scopes.free_linear(linear)?;
        }
        hir::Call::Meta { hash } => {
            let linear = converge!(exprs(cx, span, hir.args)?);

            cx.asm.push(
                Inst::Call {
                    hash,
                    addr: linear.addr(),
                    args: hir.args.len(),
                    out: needs.alloc_output(cx.scopes)?,
                },
                span,
            )?;

            cx.scopes.free_linear(linear)?;
        }
        hir::Call::Expr { expr: e } => {
            let mut function = Needs::alloc(cx, span);
            converge!(expr(cx, e, &mut function)?);
            let function = function.addr()?;

            let linear = converge!(exprs(cx, span, hir.args)?, free(cx, function));

            cx.asm.push(
                Inst::CallFn {
                    function: function.addr(),
                    addr: linear.addr(),
                    args: hir.args.len(),
                    out: needs.alloc_output(cx.scopes)?,
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

    Ok(Asm::new(span, ()))
}

/// Assemble an array of expressions.
#[instrument(span = span)]
fn expr_array<'a, 'hir, const N: usize>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    span: &'hir dyn Spanned,
    array: [(&'hir hir::Expr<'hir>, &mut dyn NeedsLike<'hir>); N],
) -> compile::Result<Asm<'hir, [NeedsAddress<'hir>; N]>> {
    let mut out = FixedVec::new();

    for (expr, needs) in array {
        converge!(self::expr(cx, expr, needs)?);

        let Some(addr) = needs.as_addr() else {
            return Err(compile::Error::msg(
                expr,
                "Expected expression to populate address",
            ));
        };

        out.try_push(*addr).with_span(span)?;
    }

    Ok(Asm::new(span, out.into_inner()))
}

#[instrument(span = span)]
fn exprs<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    span: &'hir dyn Spanned,
    args: &'hir [hir::Expr<'hir>],
) -> compile::Result<Asm<'hir, Linear<'hir>>> {
    exprs_2(cx, span, args, &[])
}

#[instrument(span = span)]
fn exprs_with<'a, 'hir, T>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    span: &'hir dyn Spanned,
    args: &'hir [T],
    map: fn(&'hir T) -> &'hir hir::Expr,
) -> compile::Result<Asm<'hir, Linear<'hir>>> {
    exprs_2_with(cx, span, args, &[], map)
}

/// Assemble a linear sequence of expressions.
#[instrument(span = span)]
fn exprs_2<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    span: &'hir dyn Spanned,
    a: &'hir [hir::Expr<'hir>],
    b: &'hir [hir::Expr<'hir>],
) -> compile::Result<Asm<'hir, Linear<'hir>>> {
    exprs_2_with(cx, span, a, b, |e| e)
}

fn exprs_2_with<'a, 'hir, T>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    span: &'hir dyn Spanned,
    a: &'hir [T],
    b: &'hir [T],
    map: fn(&'hir T) -> &'hir hir::Expr,
) -> compile::Result<Asm<'hir, Linear<'hir>>> {
    let mut linear;

    match (a, b) {
        ([], []) => {
            linear = Linear::empty(InstAddress::INVALID);
        }
        ([e], []) | ([], [e]) => {
            let e = map(e);
            let mut needs = Needs::alloc(cx, e);

            converge!(expr(cx, e, &mut needs)?, free(cx, needs));

            let Some(addr) = needs.as_addr() else {
                return Err(compile::Error::msg(
                    e,
                    "Expected expression to populate address",
                ));
            };

            linear = Linear::empty(addr.addr());
        }
        _ => {
            let len = a.len() + b.len();
            linear = cx.scopes.linear(span, len)?;

            for (e, needs) in a.iter().chain(b.iter()).zip(&mut linear) {
                let Some(()) = expr(cx, map(e), needs)?.converge() else {
                    cx.scopes.free_linear(linear);
                    return Ok(Asm::diverge(span));
                };
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
    needs: &mut dyn NeedsLike<'hir>,
) -> compile::Result<Asm<'hir>> {
    if !needs.value() {
        cx.q.diagnostics
            .not_used(cx.source_id, span, cx.context())?;
        return Ok(Asm::new(span, ()));
    }

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
            out: needs.alloc_output(cx.scopes)?,
        },
        span,
    )?;

    Ok(Asm::new(span, ()))
}

/// Assemble a continue expression.
#[instrument(span = span)]
fn expr_continue<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::ExprContinue<'hir>,
    span: &'hir dyn Spanned,
    _: &mut dyn NeedsLike<'hir>,
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
    Ok(Asm::new(span, ()))
}

/// Assemble an expr field access, like `<value>.<field>`.
#[instrument(span = span)]
fn expr_field_access<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ExprFieldAccess<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn NeedsLike<'hir>,
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
                out: needs.alloc_output(cx.scopes)?,
            },
            span,
            &var,
        )?;

        return Ok(Asm::new(span, ()));
    }

    let mut addr = Needs::alloc(cx, span);
    expr(cx, &hir.expr, &mut addr)?;

    if let Some(addr) = addr.as_addr() {
        match hir.expr_field {
            hir::ExprField::Index(index) => {
                cx.asm.push(
                    Inst::TupleIndexGetAt {
                        addr: addr.addr(),
                        index,
                        out: needs.alloc_output(cx.scopes)?,
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
                        out: needs.alloc_output(cx.scopes)?,
                    },
                    span,
                )?;
            }
            _ => return Err(compile::Error::new(span, ErrorKind::BadFieldAccess)),
        }
    }

    addr.free(cx.scopes)?;
    Ok(Asm::new(span, ()))
}

/// Assemble an expression for loop.
#[instrument(span = span)]
fn expr_for<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ExprFor<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn NeedsLike<'hir>,
) -> compile::Result<Asm<'hir>> {
    let continue_label = cx.asm.new_label("for_continue");
    let end_label = cx.asm.new_label("for_end");
    let break_label = cx.asm.new_label("for_break");

    let loop_scope = cx.scopes.child(span)?;
    let mut iter = Needs::alloc(cx, span);

    expr(cx, &hir.iter, &mut iter)?;

    let Some(iter) = iter.as_addr() else {
        return Ok(Asm::new(span, ()));
    };

    cx.asm.push_with_comment(
        Inst::CallAssociated {
            addr: iter.addr(),
            hash: *Protocol::INTO_ITER,
            args: 1,
            out: iter.output(),
        },
        &hir.iter,
        &"Protocol::INTO_ITER",
    )?;

    // Declare needed loop variables.
    let binding = cx.scopes.alloc(&hir.binding)?;

    // Declare storage for memoized `next` instance fn.
    let next_offset = if cx.options.memoize_instance_fn {
        let offset = cx.scopes.alloc(&hir.iter)?;

        cx.asm.push_with_comment(
            Inst::LoadInstanceFn {
                addr: iter.addr(),
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

    cx.loops.push(Loop {
        label: hir.label,
        continue_label: continue_label.try_clone()?,
        break_label: break_label.try_clone()?,
        output: needs.alloc_output(cx.scopes)?,
        drop: Some(iter.addr()),
    })?;

    // Use the memoized loop variable.
    if let Some(next_offset) = next_offset {
        cx.asm.push(
            Inst::CallFn {
                function: next_offset.addr(),
                addr: iter.addr(),
                args: 1,
                out: binding.output(),
            },
            span,
        )?;
    } else {
        cx.asm.push_with_comment(
            Inst::CallAssociated {
                addr: iter.addr(),
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

    let guard = cx.scopes.child(&hir.body)?;
    let mut bindings = cx.scopes.linear(&hir.binding, hir.binding.names.len())?;

    pattern_panic(cx, &hir.binding, move |cx, false_label| {
        let addr = binding.addr();

        let load = move |cx: &mut Ctxt<'a, 'hir, '_>, needs: &mut dyn NeedsLike<'hir>| {
            needs.assign_addr(cx, addr)?;
            Ok(Asm::new(&hir.binding, ()))
        };

        pat_binding_with(
            cx,
            &hir.binding,
            &hir.binding.pat,
            hir.binding.names,
            false_label,
            &load,
            &mut bindings,
        )
    })?;

    block(cx, &hir.body, &mut Needs::none(span))?;
    cx.scopes.pop(span, guard)?;

    cx.asm.jump(&continue_label, span)?;
    cx.asm.label(&end_label)?;

    // NB: Dropping has to happen before the break label. When breaking, the
    // break statement is responsible for ensuring that active iterators are
    // dropped.
    cx.asm.push(Inst::Drop { addr: iter.addr() }, span)?;

    cx.scopes.pop(span, loop_scope)?;

    if let Some(out) = needs.try_alloc_output(cx)? {
        cx.asm.push(Inst::unit(out), span)?;
    }

    cx.asm.label(&break_label)?;
    cx.loops.pop();
    Ok(Asm::new(span, ()))
}

/// Assemble an if expression.
#[instrument(span = span)]
fn expr_if<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::Conditional<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn NeedsLike<'hir>,
) -> compile::Result<Asm<'hir>> {
    let output_addr = if hir.fallback.is_none() {
        needs.try_alloc_output(cx)?
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
            condition(cx, branch.condition, &then_label, &false_label, &mut linear)?.converge()
        {
            if matches!(pat, Pattern::Refutable) {
                cx.asm.label(&false_label);
            }

            cx.scopes.pop_id(branch, scope)?;
            branches.try_push((branch, then_label, scope))?;
        }
    }

    // use fallback as fall through.
    if let Some(b) = hir.fallback {
        block(cx, b, needs)?;
    } else if let Some(out) = output_addr {
        cx.asm.push(Inst::unit(out), span)?;
    }

    cx.asm.jump(&end_label, span)?;

    let mut it = branches.into_iter().peekable();

    while let Some((branch, label, scope)) = it.next() {
        cx.asm.label(&label)?;

        cx.scopes.push(scope);

        if hir.fallback.is_none() {
            block(cx, &branch.block, &mut Needs::none(branch))?;

            if let Some(out) = output_addr {
                cx.asm.push(Inst::unit(out), span)?;
            }
        } else {
            block(cx, &branch.block, needs)?;
        }

        cx.scopes.pop(branch, scope)?;

        if it.peek().is_some() {
            cx.asm.jump(&end_label, branch)?;
        }
    }

    cx.asm.label(&end_label)?;
    cx.scopes.free_linear(linear)?;
    Ok(Asm::new(span, ()))
}

/// Assemble an expression.
#[instrument(span = span)]
fn expr_index<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ExprIndex<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn NeedsLike<'hir>,
) -> compile::Result<Asm<'hir>> {
    let guard = cx.scopes.child(span)?;

    let mut target = cx.scopes.alloc(span)?;
    let mut index = cx.scopes.alloc(span)?;

    expr(cx, &hir.target, &mut target)?;
    expr(cx, &hir.index, &mut index)?;

    cx.asm.push(
        Inst::IndexGet {
            index: index.addr(),
            target: target.addr(),
            out: needs.alloc_output(cx.scopes)?,
        },
        span,
    )?;

    cx.scopes.pop(span, guard)?;
    Ok(Asm::new(span, ()))
}

/// Assemble a let expression.
#[instrument(span = hir)]
fn expr_let<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ExprLet<'hir>,
    needs: &mut dyn NeedsLike<'hir>,
) -> compile::Result<Asm<'hir>> {
    let load =
        |cx: &mut Ctxt<'a, 'hir, '_>, needs: &mut dyn NeedsLike<'hir>| expr(cx, &hir.expr, needs);

    pattern_panic(cx, &hir.pat, move |cx, false_label| {
        pat_binding(cx, &hir.pat, &false_label, &load)
    })?;

    // If a value is needed for a let expression, it is evaluated as a unit.
    if let Some(out) = needs.try_alloc_output(cx)? {
        cx.asm.push(Inst::unit(out), hir)?;
    }

    Ok(Asm::new(hir, ()))
}

#[instrument(span = span)]
fn expr_match<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ExprMatch<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn NeedsLike<'hir>,
) -> compile::Result<Asm<'hir>> {
    let mut offset = Needs::alloc(cx, span);
    converge!(expr(cx, &hir.expr, &mut offset)?, free(cx, offset));
    let value = offset.addr()?;

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

        let load = |cx: &mut Ctxt<'a, 'hir, '_>, needs: &mut dyn NeedsLike<'hir>| {
            needs.assign_addr(cx, value.addr())?;
            Ok(Asm::new(branch, ()))
        };

        let asm = pat_binding_with(
            cx,
            &branch.pat,
            &branch.pat.pat,
            branch.pat.names,
            &match_false,
            &load,
            &mut linear,
        )?;

        if let Some(pat) = asm.converge() {
            let mut converges = true;

            if let Some(condition) = branch.condition {
                let span = condition;
                let mut cond = cx.scopes.alloc(condition)?;

                let scope = cx.scopes.child(span)?;

                if expr(cx, condition, &mut cond)?.is_converging() {
                    cx.asm.jump_if_not(cond.addr(), &match_false, span)?;
                    cx.asm.jump(&branch_label, span)?;
                } else {
                    converges = false;
                }

                cx.scopes.pop(span, scope)?;
            } else {
                // If there is no branch condition, and the branch is
                // irrefutable, there is no point in assembling the additional
                // branches.
                is_irrefutable = matches!(pat, Pattern::Irrefutable);
            }

            if converges {
                cx.asm.jump(&branch_label, span)?;
                cx.scopes.pop_id(span, pattern_scope)?;
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
        if let Some(out) = needs.try_alloc_output(cx)? {
            cx.asm.push(Inst::unit(out), span)?;
        }

        cx.asm.jump(&end_label, span)?;
    }

    let mut it = hir.branches.iter().zip(branches).peekable();

    while let Some((branch, (label, scope))) = it.next() {
        let span = branch;

        cx.asm.label(&label)?;
        cx.scopes.push(scope);

        if expr(cx, &branch.body, needs)?.is_converging() && it.peek().is_some() {
            cx.asm.jump(&end_label, span)?;
        }

        cx.scopes.pop(span, scope)?;
    }

    cx.asm.label(&end_label)?;

    value.free(cx.scopes)?;
    cx.scopes.free_linear(linear)?;
    Ok(Asm::new(span, ()))
}

/// Compile a literal object.
#[instrument(span = span)]
fn expr_object<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::ExprObject<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn NeedsLike<'hir>,
) -> compile::Result<Asm<'hir>> {
    let guard = cx.scopes.child(span)?;

    if let Some(linear) = exprs_with(cx, span, hir.assignments, |hir| &hir.assign)?.converge() {
        let slot =
            cx.q.unit
                .new_static_object_keys_iter(span, hir.assignments.iter().map(|a| a.key.1))?;

        match hir.kind {
            hir::ExprObjectKind::EmptyStruct { hash } => {
                cx.asm.push(
                    Inst::EmptyStruct {
                        hash,
                        out: needs.alloc_output(cx.scopes)?,
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
                        out: needs.alloc_output(cx.scopes)?,
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
                        out: needs.alloc_output(cx.scopes)?,
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
                        out: needs.alloc_output(cx.scopes)?,
                    },
                    span,
                )?;
            }
            hir::ExprObjectKind::Anonymous => {
                cx.asm.push(
                    Inst::Object {
                        addr: linear.addr(),
                        slot,
                        out: needs.alloc_output(cx.scopes)?,
                    },
                    span,
                )?;
            }
        }

        cx.scopes.free_linear(linear)?;
    }

    cx.scopes.pop(span, guard)?;
    Ok(Asm::new(span, ()))
}

/// Reorder the position of the field assignments on the stack so that they
/// match the expected argument order when invoking the constructor function.
fn reorder_field_assignments<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
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
    needs: &mut dyn NeedsLike<'hir>,
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

    if let Some(linear) = exprs_2(cx, span, a, b)?.converge() {
        if let Some(out) = needs.try_alloc_output(cx)? {
            cx.asm.push(
                Inst::Range {
                    addr: linear.addr(),
                    range,
                    out,
                },
                span,
            )?;
        }

        cx.scopes.free_linear(linear)?;
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
    needs: &mut dyn NeedsLike<'hir>,
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

    let branch_addr = cx.scopes.alloc(span)?;
    let mut value_addr = cx.scopes.alloc(span)?;

    let select_label = cx.asm.new_label("select");

    cx.asm.label(&select_label)?;

    cx.asm.push(
        Inst::Select {
            addr: linear.addr(),
            len: hir.exprs.len(),
            branch: branch_addr.output(),
            value: value_addr.output(),
        },
        span,
    )?;

    for (branch, (label, _)) in cx.select_branches.iter().enumerate() {
        cx.asm
            .jump_if_branch(branch_addr.addr(), branch as i64, label, span)?;
    }

    cx.scopes.free(branch_addr)?;

    if let Some((_, label)) = &default_branch {
        cx.asm.jump(label, span)?;
    }

    cx.asm.jump(&end_label, span)?;

    let mut branches = core::mem::take(&mut cx.select_branches);

    for (label, branch) in branches.drain(..) {
        cx.asm.label(&label)?;

        let scope = cx.scopes.child(&branch.body)?;

        if let Some(..) = fn_arg_pat(cx, &branch.pat, &mut value_addr, &select_label)?.converge() {
            if let Some(()) = expr(cx, &branch.body, needs)?.converge() {
                cx.asm.jump(&end_label, span)?;
            }
        }

        cx.scopes.pop(&branch.body, scope)?;
    }

    cx.select_branches = branches;

    cx.scopes.free_linear(linear)?;
    cx.scopes.free(value_addr)?;

    if let Some((branch, label)) = default_branch {
        cx.asm.label(&label)?;
        expr(cx, branch, needs)?.ignore();
    }

    cx.asm.label(&end_label)?;
    Ok(Asm::new(span, ()))
}

#[instrument(span = span)]
fn expr_select<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::ExprSelect<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn NeedsLike<'hir>,
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
    needs: &mut dyn NeedsLike<'hir>,
) -> compile::Result<Asm<'hir>> {
    let mut addr = Needs::alloc(cx, span);
    converge!(expr(cx, hir, &mut addr)?);

    let addr = addr.addr()?;

    cx.asm.push(
        Inst::Try {
            addr: addr.addr(),
            out: needs.alloc_output(cx.scopes)?,
        },
        span,
    )?;

    addr.free(cx.scopes)?;
    Ok(Asm::new(span, ()))
}

/// Assemble a literal tuple.
#[instrument(span = span)]
fn expr_tuple<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &hir::ExprSeq<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn NeedsLike<'hir>,
) -> compile::Result<Asm<'hir>> {
    macro_rules! tuple {
        ($variant:ident $(, $var:ident, $expr:ident)* $(,)?) => {{
            $(let mut $var = Needs::alloc(cx, span);)*

            let asm = expr_array(
                cx,
                span,
                [$(($expr, &mut $var)),*],
            )?;

            let asm = if let Some([$($var),*]) = asm.converge() {
                cx.asm.push(
                    Inst::$variant {
                        args: [$($var.addr(),)*],
                        out: needs.alloc_output(cx.scopes)?,
                    },
                    span,
                )?;

                Asm::new(span, ())
            } else {
                Asm::diverge(span)
            };

            $($var.free(cx.scopes)?;)*
            return Ok(asm);
        }};
    }

    match hir.items {
        [] => {
            cx.asm
                .push(Inst::unit(needs.alloc_output(cx.scopes)?), span)?;
        }
        [e1] => tuple!(Tuple1, v1, e1),
        [e1, e2] => tuple!(Tuple2, v1, e1, v2, e2),
        [e1, e2, e3] => tuple!(Tuple3, v1, e1, v2, e2, v3, e3),
        [e1, e2, e3, e4] => tuple!(Tuple4, v1, e1, v2, e2, v3, e3, v4, e4),
        _ => {
            let linear = converge!(exprs(cx, span, hir.items)?);

            if needs.value() {
                cx.asm.push(
                    Inst::Tuple {
                        addr: linear.addr(),
                        count: hir.items.len(),
                        out: needs.alloc_output(cx.scopes)?,
                    },
                    span,
                )?;
            }

            cx.scopes.free_linear(linear)?;
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
    needs: &mut dyn NeedsLike<'hir>,
) -> compile::Result<Asm<'hir>> {
    converge!(expr(cx, &hir.expr, needs)?);

    let Some(addr) = needs.as_addr() else {
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
    needs: &mut dyn NeedsLike<'hir>,
) -> compile::Result<Asm<'hir>> {
    let mut linear = cx.scopes.linear(span, hir.items.len())?;
    let count = hir.items.len();

    for (e, needs) in hir.items.iter().zip(&mut linear) {
        converge!(expr(cx, e, needs)?, free(cx, linear));
    }

    if let Some(out) = needs.try_alloc_addr(cx)? {
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
    Ok(Asm::new(span, ()))
}

/// Assemble a while loop.
#[instrument(span = span)]
fn expr_loop<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ExprLoop<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn NeedsLike<'hir>,
) -> compile::Result<Asm<'hir>> {
    let continue_label = cx.asm.new_label("while_continue");
    let then_label = cx.asm.new_label("while_then");
    let end_label = cx.asm.new_label("while_end");
    let break_label = cx.asm.new_label("while_break");

    cx.loops.push(Loop {
        label: hir.label,
        continue_label: continue_label.try_clone()?,
        break_label: break_label.try_clone()?,
        output: needs.alloc_output(cx.scopes)?,
        drop: None,
    })?;

    cx.asm.label(&continue_label)?;

    let count = hir.condition.and_then(|c| c.count()).unwrap_or_default();
    let mut linear = cx.scopes.linear(span, count)?;

    let condition_scope = if let Some(hir) = hir.condition {
        if let Some((scope, _)) =
            condition(cx, hir, &then_label, &end_label, &mut linear)?.converge()
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
    block(cx, &hir.body, &mut Needs::none(span))?.ignore();

    if let Some(scope) = condition_scope {
        cx.scopes.pop(span, scope)?;
    }

    cx.asm.jump(&continue_label, span)?;
    cx.asm.label(&end_label)?;

    if let Some(out) = needs.try_alloc_output(cx)? {
        cx.asm.push(Inst::unit(out), span)?;
    }

    // NB: breaks produce their own value / perform their own cleanup.
    cx.asm.label(&break_label)?;
    cx.scopes.free_linear(linear)?;
    cx.loops.pop();
    Ok(Asm::new(span, ()))
}

/// Assemble a `yield` expression.
#[instrument(span = span)]
fn expr_yield<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: Option<&'hir hir::Expr<'hir>>,
    span: &'hir dyn Spanned,
    needs: &mut dyn NeedsLike<'hir>,
) -> compile::Result<Asm<'hir>> {
    let out = needs.alloc_output(cx.scopes)?;

    if let Some(e) = hir {
        let mut addr = cx.scopes.alloc(span)?;
        converge!(expr(cx, e, &mut addr)?, free(cx, addr));

        cx.asm.push(
            Inst::Yield {
                addr: addr.addr(),
                out,
            },
            span,
        )?;
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
    needs: &mut dyn NeedsLike<'hir>,
) -> compile::Result<Asm<'hir>> {
    // Elide the entire literal if it's not needed.
    let Some(addr) = needs.try_alloc_addr(cx)? else {
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
    needs: &mut dyn NeedsLike<'hir>,
) -> compile::Result<Asm<'hir>> {
    let load =
        |cx: &mut Ctxt<'a, 'hir, '_>, needs: &mut dyn NeedsLike<'hir>| expr(cx, &hir.expr, needs);

    converge!(pattern_panic(cx, &hir.pat, |cx, false_label| {
        pat_binding(cx, &hir.pat, &false_label, &load)
    })?);

    // If a value is needed for a let expression, it is evaluated as a unit.
    if let Some(out) = needs.try_alloc_output(cx)? {
        cx.asm.push(Inst::unit(out), hir)?;
    }

    Ok(Asm::new(hir, ()))
}
