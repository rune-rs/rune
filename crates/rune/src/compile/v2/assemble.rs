use core::fmt;
use core::mem::take;

use tracing::instrument_ast;

use crate::alloc::prelude::*;
use crate::alloc::{BTreeMap, Box};
use crate::ast::{self, Spanned};
use crate::compile::const_eval;
use crate::compile::{self, Assembly, ErrorKind, ItemId, Location, Options, WithSpan};
use crate::hir;
use crate::query::Query;
use crate::runtime::ConstInstance;
use crate::runtime::{
    self, inst, ConstValue, ConstValueKind, Inline, InstArithmeticOp, InstBitwiseOp, InstOp,
    InstRange, InstShiftOp, InstTarget, InstValue, Label, Output, PanicReason, Protocol, TypeHash,
};
use crate::{Hash, ItemBuf, SourceId};

use super::{Address, Any, Break, Breaks, DanglingScope, Linear, Needs, ScopeHandle, Scopes};

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
    /// The expressions making up the item being assembled.
    ///
    /// Expressions refer to their children by [`hir::ExprId`], which is
    /// resolved through here.
    pub(crate) exprs: &'hir hir::Exprs<'hir>,
    /// Scopes defined in the compiler.
    pub(crate) scopes: &'a Scopes<'hir>,
    /// Context for which to emit warnings.
    pub(crate) contexts: Vec<&'hir dyn Spanned>,
    /// The nesting of loop we are currently in.
    pub(crate) breaks: Breaks<'hir>,
    /// Enabled optimizations.
    pub(crate) options: &'a Options,
    /// Work buffer for select branches.
    pub(crate) select_branches: Vec<(Label, &'hir hir::ExprSelectBranch<'hir>)>,
    /// Values to drop.
    pub(crate) drop: Vec<inst::Address>,
    /// Whether we are assembling into the interior unit used for constant
    /// evaluation.
    ///
    /// A call to a constant function is folded into its value when it appears
    /// in ordinary code, but assembled as a real call when it appears inside of
    /// another constant, so that the virtual machine performs the recursion
    /// rather than the compiler.
    pub(crate) const_eval: bool,
}

impl<'hir> Ctxt<'_, 'hir, '_> {
    fn drop_dangling(&mut self, span: &dyn Spanned) -> compile::Result<()> {
        self.scopes
            .drain_dangling_into(&mut self.drop)
            .with_span(span)?;

        let mut drop_set = self.q.unit.drop_set();

        for addr in self.drop.drain(..).rev() {
            drop_set.push(addr)?;
        }

        if let Some(set) = drop_set.finish()? {
            self.asm.push(inst::Kind::Drop { set }, span)?;
        }

        Ok(())
    }

    /// Get the latest relevant warning context.
    pub(crate) fn context(&self) -> Option<&'hir dyn Spanned> {
        self.contexts.last().copied()
    }

    /// Call a constant function by item, evaluating its arguments as constants
    /// first, and return the value it produced.
    pub(crate) fn call_const_fn(
        &mut self,
        span: &dyn Spanned,
        id: ItemId,
        args: &[hir::ExprId],
    ) -> compile::Result<ConstValue> {
        let mut values = Vec::try_with_capacity(args.len())?;

        for &arg in args {
            let hir = self.exprs.get(arg);
            let location = Location::new(self.source_id, hir.span());
            let entry = const_eval::Entry::Expr(self.exprs, hir);
            let value = const_eval::eval(&mut self.q, location, hir, entry, Vec::new())?;
            values.try_push(crate::from_value(value).with_span(span)?)?;
        }

        self.q.const_eval_call(span, id, &values)
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

impl<T> Asm<'_, T> {
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
#[instrument_ast(span = hir)]
pub(crate) fn fn_from_item_fn<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::ItemFn<'hir>,
) -> compile::Result<()> {
    let mut arguments = cx.scopes.linear(hir, hir.args.len())?;

    for (arg, needs) in hir.args.iter().zip(&mut arguments) {
        let hir::FnArg::Pat(pat) = arg;

        let asm = pattern_panic(cx, pat, move |cx, false_label| {
            fn_arg_pat(cx, pat, needs, false_label)
        })?;

        asm.ignore();
    }

    if hir.body.value.is_some() {
        return_(cx, hir, &hir.body, block_without_scope)?.ignore();
    } else {
        let mut needs = Any::ignore(&hir.body);

        if block_without_scope(cx, &hir.body, &mut needs)?.converging() {
            cx.asm.push(inst::Kind::ReturnUnit, hir)?;
        }
    }

    arguments.free()?;
    cx.scopes.pop_last(hir)?;
    Ok(())
}

/// Assemble an expression as the body of a constant entry point.
#[instrument_ast(span = hir)]
pub(crate) fn const_expr<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::Expr<'hir>,
) -> compile::Result<()> {
    return_(cx, hir, hir, expr)?.ignore();
    cx.scopes.pop_last(hir)?;
    Ok(())
}

/// Assemble a block as the body of a constant entry point.
#[instrument_ast(span = hir)]
pub(crate) fn const_block<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::Block<'hir>,
) -> compile::Result<()> {
    return_(cx, hir, hir, block_without_scope)?.ignore();
    cx.scopes.pop_last(hir)?;
    Ok(())
}

/// Assemble an async block.
#[instrument_ast(span = hir.block.span)]
pub(crate) fn async_block_secondary<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::AsyncBlock<'hir>,
) -> compile::Result<()> {
    let linear = cx.scopes.linear(&hir.block, hir.captures.len())?;

    for (name, needs) in hir.captures.iter().copied().zip(&linear) {
        cx.scopes.define(&hir.block, name, needs)?;
    }

    return_(cx, &hir.block, hir.block, block_without_scope)?.ignore();

    linear.free()?;
    cx.scopes.pop_last(&hir.block)?;
    Ok(())
}

/// Assemble the body of a closure function.
#[instrument_ast(span = hir)]
pub(crate) fn expr_closure_secondary<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &'hir hir::ExprClosure<'hir>,
) -> compile::Result<()> {
    let mut arguments = cx.scopes.linear(hir, hir.args.len())?;
    let environment = cx.scopes.linear(hir, hir.captures.len())?;

    if !hir.captures.is_empty() {
        cx.asm.push(
            inst::Kind::Environment {
                addr: environment.addr(),
                count: hir.captures.len(),
                out: environment.addr().output(),
            },
            hir,
        )?;

        for (capture, needs) in hir.captures.iter().copied().zip(&environment) {
            cx.scopes.define(hir, capture, needs)?;
        }
    }

    for (arg, needs) in hir.args.iter().zip(&mut arguments) {
        match arg {
            hir::FnArg::Pat(pat) => {
                let asm = pattern_panic(cx, pat, move |cx, false_label| {
                    fn_arg_pat(cx, pat, needs, false_label)
                })?;

                asm.ignore();
            }
        }
    }

    return_(cx, hir, cx.exprs.get(hir.body), expr)?.ignore();

    environment.free()?;
    arguments.free()?;
    cx.scopes.pop_last(hir)?;
    Ok(())
}

#[instrument_ast(span = pat)]
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

/// Look up the storage slot assigned to a static item.
///
/// A static is runtime storage whose value is only known once the unit is
/// running, so it has nothing to contribute to a constant and is rejected when
/// one is being assembled.
fn global_slot(
    cx: &mut Ctxt<'_, '_, '_>,
    hash: Hash,
    span: &dyn Spanned,
) -> compile::Result<usize> {
    if cx.const_eval {
        let item = match cx.q.pool.item_for_hash(hash) {
            Some(item) => item.try_to_owned()?,
            None => ItemBuf::new(),
        };

        return Err(compile::Error::new(
            span,
            ErrorKind::StaticInConstContext { item },
        ));
    }

    Ok(cx.q.unit.global_slot(hash).with_span(span)?)
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
        inst::Kind::Return {
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
            inst::Kind::Panic {
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
#[instrument_ast(span = hir)]
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

#[instrument_ast(span = span)]
fn pat_binding_with<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    span: &'hir dyn Spanned,
    pat: &'hir hir::Pat<'hir>,
    names: &[hir::Variable],
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

#[instrument_ast(span = span)]
fn pat_binding_with_single<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    span: &'hir dyn Spanned,
    pat: &'hir hir::Pat<'hir>,
    name: hir::Variable,
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

/// Where a pattern's value is loaded from.
#[derive(Clone, Copy)]
enum PatLoad {
    /// From the caller supplied load, used for the outermost pattern.
    Root,
    /// From a tuple index of an already loaded container.
    Tuple { addr: inst::Address, index: usize },
    /// From a field of an already loaded container.
    Field { addr: inst::Address, slot: usize },
}

/// A partially assembled pattern, waiting for its remaining children.
enum PatStep<'a, 'hir> {
    /// A sequence pattern, `(a, b)` or `[a, b]`.
    Sequence {
        addr: Address<'a, 'hir>,
        items: &'hir [hir::Pat<'hir>],
        at: usize,
    },
    /// An object pattern.
    Object {
        addr: Address<'a, 'hir>,
        bindings: &'hir [hir::Binding<'hir>],
        slots: Vec<usize>,
        at: usize,
    },
}

impl<'a, 'hir> PatStep<'a, 'hir> {
    fn free(self) -> compile::Result<()> {
        match self {
            PatStep::Sequence { addr, .. } | PatStep::Object { addr, .. } => addr.free(),
        }
    }
}

/// Load the value a pattern matches against.
fn pat_load<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    src: PatLoad,
    root: &mut dyn FnMut(
        &mut Ctxt<'a, 'hir, '_>,
        &mut dyn Needs<'a, 'hir>,
    ) -> compile::Result<Asm<'hir>>,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    match src {
        PatLoad::Root => root(cx, needs),
        PatLoad::Tuple { addr, index } => {
            cx.asm.push(
                inst::Kind::TupleIndexGetAt {
                    addr,
                    index,
                    out: needs.alloc_output()?,
                },
                span,
            )?;

            Ok(Asm::new(span, ()))
        }
        PatLoad::Field { addr, slot } => {
            cx.asm.push(
                inst::Kind::ObjectIndexGetAt {
                    addr,
                    slot,
                    out: needs.alloc_output()?,
                },
                span,
            )?;

            Ok(Asm::new(span, ()))
        }
    }
}

/// Assemble a pattern.
///
/// Patterns nest through sequence and object patterns, which is walked over an
/// explicit stack rather than recursively. `bindings` and `false_label` are
/// shared by the whole walk, so they stay parameters rather than being parked
/// in a frame.
fn pat<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::Pat<'hir>,
    false_label: &Label,
    load: &mut dyn FnMut(
        &mut Ctxt<'a, 'hir, '_>,
        &mut dyn Needs<'a, 'hir>,
    ) -> compile::Result<Asm<'hir>>,
    bindings: &mut dyn Bindings<hir::Variable, &mut dyn Needs<'a, 'hir>>,
) -> compile::Result<Asm<'hir, Pattern>> {
    let mut stack = Vec::new();
    let mut root_pattern = None;

    let mut next = Some((hir, PatLoad::Root));

    'outer: loop {
        if let Some((hir, src)) = next.take() {
            let is_root = matches!(src, PatLoad::Root);

            let asm = pat_one(cx, hir, src, false_label, load, bindings, &mut stack)?;

            let Some(pattern) = asm else {
                // The pattern diverged, so release everything still pending.
                for step in stack.drain(..).rev() {
                    step.free()?;
                }

                return Ok(Asm::diverge(hir));
            };

            if is_root {
                root_pattern = Some(pattern);
            }

            continue;
        }

        loop {
            let Some(step) = stack.pop() else {
                break 'outer;
            };

            match step {
                PatStep::Sequence { addr, items, at } => {
                    if let Some(p) = items.get(at) {
                        let src = PatLoad::Tuple {
                            addr: addr.addr(),
                            index: at,
                        };

                        stack.try_push(PatStep::Sequence {
                            addr,
                            items,
                            at: at + 1,
                        })?;

                        next = Some((p, src));
                        continue 'outer;
                    }

                    addr.free()?;
                }
                PatStep::Object {
                    addr,
                    bindings: hir_bindings,
                    slots,
                    at,
                } => {
                    if let Some(binding) = hir_bindings.get(at) {
                        let slot = slots[at];

                        match *binding {
                            hir::Binding::Binding(_, _, p) => {
                                let src = PatLoad::Field {
                                    addr: addr.addr(),
                                    slot,
                                };

                                stack.try_push(PatStep::Object {
                                    addr,
                                    bindings: hir_bindings,
                                    slots,
                                    at: at + 1,
                                })?;

                                next = Some((p, src));
                                continue 'outer;
                            }
                            hir::Binding::Ident(span, name, id) => {
                                let Some(binding) = bindings.remove(&id) else {
                                    addr.free()?;

                                    return Err(compile::Error::msg(
                                        span,
                                        format!("No binding for {name:?}"),
                                    ));
                                };

                                cx.asm.push(
                                    inst::Kind::ObjectIndexGetAt {
                                        addr: addr.addr(),
                                        slot,
                                        out: binding.output()?,
                                    },
                                    &span,
                                )?;

                                stack.try_push(PatStep::Object {
                                    addr,
                                    bindings: hir_bindings,
                                    slots,
                                    at: at + 1,
                                })?;

                                continue;
                            }
                        }
                    }

                    addr.free()?;
                }
            }
        }
    }

    let pattern = root_pattern.unwrap_or(Pattern::Irrefutable);
    Ok(Asm::new(hir, pattern))
}

/// Assemble a single pattern node, pushing a step if it has children.
///
/// Returns `None` if assembly diverged.
#[allow(clippy::too_many_arguments)]
fn pat_one<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::Pat<'hir>,
    src: PatLoad,
    false_label: &Label,
    root: &mut dyn FnMut(
        &mut Ctxt<'a, 'hir, '_>,
        &mut dyn Needs<'a, 'hir>,
    ) -> compile::Result<Asm<'hir>>,
    bindings: &mut dyn Bindings<hir::Variable, &mut dyn Needs<'a, 'hir>>,
    stack: &mut Vec<PatStep<'a, 'hir>>,
) -> compile::Result<Option<Pattern>> {
    let span = hir;

    match hir.kind {
        hir::PatKind::Ignore => {
            // ignore binding, but might still have effects, so must call load.
            if pat_load(cx, src, root, span, &mut Any::ignore(hir))?.diverging() {
                return Ok(None);
            }

            Ok(Some(Pattern::Irrefutable))
        }
        hir::PatKind::Path(kind) => match *kind {
            hir::PatPathKind::Kind(kind) => {
                let mut needs = cx.scopes.defer(hir);

                if pat_load(cx, src, root, span, &mut needs)?.diverging() {
                    needs.free()?;
                    return Ok(None);
                }

                let cond = cx.scopes.alloc(hir)?;
                let inst = pat_sequence_kind_to_inst(*kind, needs.addr()?.addr(), cond.output())?;

                cx.asm.push(inst, hir)?;
                cx.asm.jump_if_not(cond.addr(), false_label, hir)?;

                cond.free()?;
                needs.free()?;
                Ok(Some(Pattern::Refutable))
            }
            hir::PatPathKind::Ident(name) => {
                let Some(binding) = bindings.remove(&name) else {
                    return Err(compile::Error::msg(hir, format!("No binding for {name:?}")));
                };

                if pat_load(cx, src, root, span, binding)?.diverging() {
                    return Ok(None);
                }

                Ok(Some(Pattern::Irrefutable))
            }
        },
        hir::PatKind::Lit(e) => {
            let mut load = |cx: &mut Ctxt<'a, 'hir, '_>, needs: &mut dyn Needs<'a, 'hir>| {
                pat_load(cx, src, root, span, needs)
            };

            let asm = pat_lit(cx, cx.exprs.get(e), false_label, &mut load)?;
            Ok(asm.into_converging())
        }
        hir::PatKind::Sequence(seq) => {
            let mut needs = cx.scopes.defer(span);

            if pat_load(cx, src, root, span, &mut needs)?.diverging() {
                needs.free()?;
                return Ok(None);
            }

            let addr = needs.into_addr()?;
            let cond = cx.scopes.alloc(span)?.with_name("loaded pattern condition");

            let inst = pat_sequence_kind_to_inst(seq.kind, addr.addr(), cond.output())?;

            cx.asm.push(inst, span)?;
            cx.asm.jump_if_not(cond.addr(), false_label, span)?;
            cond.free()?;

            stack.try_push(PatStep::Sequence {
                addr,
                items: seq.items,
                at: 0,
            })?;

            Ok(Some(Pattern::Refutable))
        }
        hir::PatKind::Object(object) => {
            let mut needs = cx.scopes.defer(span);

            if pat_load(cx, src, root, span, &mut needs)?.diverging() {
                needs.free()?;
                return Ok(None);
            }

            let addr = needs.into_addr()?;
            let cond = cx.scopes.alloc(span)?;

            let mut slots = Vec::new();

            for binding in object.bindings {
                slots.try_push(cx.q.unit.new_static_string(span, binding.key())?)?;
            }

            let inst = match object.kind {
                hir::PatSequenceKind::Type { hash, variant_hash } => inst::Kind::MatchType {
                    hash,
                    variant_hash,
                    addr: addr.addr(),
                    out: cond.output(),
                },
                hir::PatSequenceKind::Sequence {
                    hash: runtime::Object::HASH,
                    is_open,
                    ..
                } => {
                    let mut entries = object
                        .bindings
                        .iter()
                        .map(|b| b.key())
                        .try_collect::<Vec<_>>()?;

                    entries.sort();

                    let keys = cx.q.unit.new_static_object_keys_iter(span, entries)?;

                    inst::Kind::MatchObject {
                        slot: keys,
                        exact: !is_open,
                        addr: addr.addr(),
                        out: cond.output(),
                    }
                }
                hir::PatSequenceKind::Sequence {
                    hash,
                    count,
                    is_open,
                } => inst::Kind::MatchSequence {
                    hash,
                    len: count,
                    exact: !is_open,
                    addr: addr.addr(),
                    out: cond.output(),
                },
            };

            // Copy the temporary and check that its length matches the pattern
            // and that it is indeed a vector.
            cx.asm.push(inst, span)?;
            cx.asm.jump_if_not(cond.addr(), false_label, span)?;
            cond.free()?;

            stack.try_push(PatStep::Object {
                addr,
                bindings: object.bindings,
                slots,
                at: 0,
            })?;

            Ok(Some(Pattern::Refutable))
        }
    }
}

/// Assemble a pattern literal.
#[instrument_ast(span = hir)]
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
    let cond = cx.scopes.alloc(hir)?;

    let Some(inst) = pat_lit_inst(cx, hir, needs.addr()?.addr(), cond.addr())? else {
        return Err(compile::Error::new(hir, ErrorKind::UnsupportedPatternExpr));
    };

    cx.asm.push(inst, hir)?;
    cx.asm.jump_if_not(cond.addr(), false_label, hir)?;
    cond.free()?;
    needs.free()?;
    Ok(Asm::new(hir, Pattern::Refutable))
}

#[instrument_ast(span = hir)]
fn pat_lit_inst(
    cx: &mut Ctxt<'_, '_, '_>,
    hir: &hir::Expr<'_>,
    addr: inst::Address,
    cond: inst::Address,
) -> compile::Result<Option<inst::Kind>> {
    let hir::ExprKind::Lit(lit) = hir.kind else {
        return Ok(None);
    };

    let out = cond.output();

    let inst = match lit {
        hir::Lit::Char(value) => inst::Kind::EqChar { addr, value, out },
        hir::Lit::Str(string) => inst::Kind::EqString {
            addr,
            slot: cx.q.unit.new_static_string(hir, string)?,
            out,
        },
        hir::Lit::ByteStr(bytes) => inst::Kind::EqBytes {
            addr,
            slot: cx.q.unit.new_static_bytes(hir, bytes)?,
            out,
        },
        hir::Lit::Unsigned(value) => inst::Kind::EqUnsigned { addr, value, out },
        hir::Lit::Signed(value) => inst::Kind::EqSigned { addr, value, out },
        hir::Lit::Bool(value) => inst::Kind::EqBool { addr, value, out },
        _ => return Ok(None),
    };

    Ok(Some(inst))
}

fn pat_sequence_kind_to_inst(
    kind: hir::PatSequenceKind,
    addr: inst::Address,
    out: Output,
) -> compile::Result<inst::Kind> {
    let inst = match kind {
        hir::PatSequenceKind::Type { hash, variant_hash } => inst::Kind::MatchType {
            hash,
            variant_hash,
            addr,
            out,
        },
        hir::PatSequenceKind::Sequence {
            hash,
            count,
            is_open,
        } => inst::Kind::MatchSequence {
            hash,
            len: count,
            exact: !is_open,
            addr,
            out,
        },
    };

    Ok(inst)
}

/// Call a block.
#[instrument_ast(span = hir)]
fn block_without_scope<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::Block<'hir>,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let mut diverge = None;
    cx.contexts.try_push(hir)?;

    for stmt in hir.statements {
        let mut needs = Any::ignore(hir).with_name("statement ignore");

        if let Some(cause) = diverge {
            cx.q.diagnostics.unreachable(cx.source_id, stmt, cause)?;
            continue;
        }

        let asm = match stmt {
            hir::Stmt::Local(hir) => local(cx, hir, &mut needs)?,
            hir::Stmt::Expr(_, hir) => expr(cx, cx.exprs.get(*hir), &mut needs)?,
        };

        if asm.diverging() && diverge.is_none() {
            diverge = Some(stmt);
        }
    }

    if let Some(cause) = diverge {
        if let Some(e) = hir.value {
            let e = cx.exprs.get(e);
            cx.q.diagnostics.unreachable(cx.source_id, e, cause)?;
        }
    } else if let Some(e) = hir.value {
        let e = cx.exprs.get(e);

        if expr(cx, e, needs)?.diverging() {
            diverge = Some(e);
        }
    } else if let Some(out) = needs.try_alloc_output()? {
        cx.asm.push(inst::Kind::unit(out), hir)?;
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

/// Assemble a constant value.
#[instrument_ast(span = span)]
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

    match value.as_kind() {
        ConstValueKind::Inline(value) => match *value {
            Inline::Empty => {
                return Err(compile::Error::msg(
                    span,
                    "Empty inline constant value is not supported",
                ));
            }
            Inline::Unit => {
                cx.asm.push(inst::Kind::unit(out), span)?;
            }
            Inline::Char(v) => {
                cx.asm.push(inst::Kind::char(v, out), span)?;
            }
            Inline::Signed(v) => {
                cx.asm.push(inst::Kind::signed(v, out), span)?;
            }
            Inline::Unsigned(v) => {
                cx.asm.push(inst::Kind::unsigned(v, out), span)?;
            }
            Inline::Float(v) => {
                cx.asm.push(inst::Kind::float(v, out), span)?;
            }
            Inline::Bool(v) => {
                cx.asm.push(inst::Kind::bool(v, out), span)?;
            }
            Inline::Type(v) => {
                cx.asm.push(inst::Kind::ty(v, out), span)?;
            }
            Inline::Ordering(v) => {
                cx.asm.push(inst::Kind::ordering(v, out), span)?;
            }
            Inline::Hash(v) => {
                cx.asm.push(inst::Kind::hash(v, out), span)?;
            }
        },
        ConstValueKind::String(s) => {
            let slot = cx.q.unit.new_static_string(span, s)?;
            cx.asm.push(inst::Kind::String { slot, out }, span)?;
        }
        ConstValueKind::Bytes(b) => {
            let slot = cx.q.unit.new_static_bytes(span, b)?;
            cx.asm.push(inst::Kind::Bytes { slot, out }, span)?;
        }
        ConstValueKind::Instance(instance) => match &**instance {
            ConstInstance {
                hash: runtime::Object::HASH,
                variant_hash: Hash::EMPTY,
                fields,
            } => {
                let mut entries = Vec::try_with_capacity(fields.len())?;

                for value in fields.iter() {
                    let (key, value) = value.as_pair().with_span(span)?;
                    let key = key.as_string().with_span(span)?;
                    entries.try_push((key, value))?;
                }

                entries.sort_by_key(|&(k, _)| k);

                let mut linear = cx.scopes.linear(span, entries.len())?;

                for ((_, value), needs) in entries.iter().copied().zip(&mut linear) {
                    const_(cx, value, span, needs)?;
                }

                let slot =
                    cx.q.unit
                        .new_static_object_keys_iter(span, entries.iter().map(|e| e.0))?;

                cx.asm.push(
                    inst::Kind::Object {
                        addr: linear.addr(),
                        slot,
                        out,
                    },
                    span,
                )?;

                linear.free_non_dangling()?;
            }
            ConstInstance {
                hash,
                variant_hash: Hash::EMPTY,
                fields,
            } => {
                let mut linear = cx.scopes.linear(span, fields.len())?;

                for (value, needs) in fields.iter().zip(&mut linear) {
                    const_(cx, value, span, needs)?;
                }

                match *hash {
                    runtime::Vec::HASH => {
                        cx.asm.push(
                            inst::Kind::Vec {
                                addr: linear.addr(),
                                count: fields.len(),
                                out,
                            },
                            span,
                        )?;
                    }
                    runtime::OwnedTuple::HASH => {
                        cx.asm.push(
                            inst::Kind::Tuple {
                                addr: linear.addr(),
                                count: fields.len(),
                                out,
                            },
                            span,
                        )?;
                    }
                    _ => {
                        cx.asm.push(
                            inst::Kind::ConstConstruct {
                                addr: linear.addr(),
                                hash: *hash,
                                count: fields.len(),
                                out,
                            },
                            span,
                        )?;
                    }
                }

                linear.free_non_dangling()?;
            }
            ConstInstance {
                variant_hash,
                fields,
                ..
            } => {
                let mut linear = cx.scopes.linear(span, fields.len())?;

                for (value, needs) in fields.iter().zip(&mut linear) {
                    const_(cx, value, span, needs)?;
                }

                cx.asm.push(
                    inst::Kind::Call {
                        addr: linear.addr(),
                        hash: *variant_hash,
                        args: fields.len(),
                        out,
                    },
                    span,
                )?;

                linear.free_non_dangling()?;
            }
        },
    }

    Ok(())
}

/// A pending step on the work stack maintained by [`expr`].
///
/// Every variant corresponds to an expression which forms the *spine* of a
/// chain. Chains are the kind of nesting which grows with the width of
/// otherwise flat source code - `a + b + c + ..`, `a.b().c()`, `a[0][1]`, and
/// so on - so their depth cannot be bounded by any limit which is reasonable to
/// impose on lexical nesting. Walking them over a heap allocated stack instead
/// of the call stack is what keeps the assembler from overflowing on such
/// input.
///
/// A step owns the addresses which its pending children are assembled into.
/// They are released once the step is completed, in the same order as the
/// equivalent recursive implementation would drop them.
enum Step<'a, 'hir> {
    /// `!<expr>` or `-<expr>`, waiting for the operand.
    Unary {
        hir: &'hir hir::ExprUnary,
        span: &'hir dyn Spanned,
        addr: Any<'a, 'hir>,
    },
    /// `{ .. }`, waiting for a statement or for the block's value.
    ///
    /// Blocks are lexical nesting rather than a chain, but they are walked over
    /// the same stack so that the nesting limit can eventually be lifted.
    Block {
        hir: &'hir hir::Block<'hir>,
        scope: ScopeHandle,
        break_label: Option<Label>,
        /// Index of the statement which follows the one being assembled.
        at: usize,
        /// The cause of divergence, if any.
        diverge: Option<&'hir dyn Spanned>,
        /// The statement being assembled, absent while the block's value is.
        stmt: Option<&'hir hir::Stmt<'hir>>,
        /// Needs for the statement being assembled.
        needs: Option<Any<'a, 'hir>>,
    },
    /// `while .. { .. }`, waiting for the condition.
    LoopCondition {
        hir: &'hir hir::ExprLoop<'hir>,
        span: &'hir dyn Spanned,
        continue_label: Label,
        then_label: Label,
        end_label: Label,
        break_label: Label,
        linear: Linear<'a, 'hir>,
        scope: ScopeHandle,
        value: Any<'a, 'hir>,
    },
    /// `loop { .. }` or `while .. { .. }`, waiting for the body.
    Loop {
        span: &'hir dyn Spanned,
        continue_label: Label,
        end_label: Label,
        break_label: Label,
        condition_scope: Option<ScopeHandle>,
        linear: Linear<'a, 'hir>,
        body: Any<'a, 'hir>,
    },
    /// `if .. { .. }`, waiting for the condition of a branch.
    IfCondition {
        hir: &'hir hir::Conditional<'hir>,
        span: &'hir dyn Spanned,
        end_label: Label,
        linear: Linear<'a, 'hir>,
        branches: Vec<(&'hir hir::ConditionalBranch<'hir>, Label, DanglingScope)>,
        at: usize,
        then_label: Label,
        false_label: Label,
        scope: ScopeHandle,
        value: Any<'a, 'hir>,
    },
    /// `if .. { .. }`, waiting for the fallback or for a branch body.
    If {
        hir: &'hir hir::Conditional<'hir>,
        span: &'hir dyn Spanned,
        end_label: Label,
        linear: Linear<'a, 'hir>,
        /// Branches still to assemble, in reverse so they pop in order.
        branches: Vec<(&'hir hir::ConditionalBranch<'hir>, Label, DanglingScope)>,
        output_addr: Option<Output>,
        all_diverging: bool,
        /// The branch being assembled, along with the scope to pop after it.
        pending: Option<(&'hir hir::ConditionalBranch<'hir>, ScopeHandle)>,
        /// Needs a branch is assembled into when there is no fallback.
        ignore: Option<Any<'a, 'hir>>,
    },
    /// `for .. in .. { .. }`, waiting for the iterator.
    ForIter {
        hir: &'hir hir::ExprFor<'hir>,
        span: &'hir dyn Spanned,
        iter: Any<'a, 'hir>,
    },
    /// `for .. in .. { .. }`, waiting for the body.
    For(Box<StepFor<'a, 'hir>>),
    /// `match .. { .. }`, waiting for the value being matched.
    MatchValue {
        hir: &'hir hir::ExprMatch<'hir>,
        span: &'hir dyn Spanned,
        value: Any<'a, 'hir>,
    },
    /// `select { .. }`, waiting for the body of a branch or for the default.
    Select(Box<StepSelect<'a, 'hir>>),
    /// `match .. { .. }`, waiting for the guard of an arm.
    MatchGuard(Box<StepMatchGuard<'a, 'hir>>),
    /// `match .. { .. }`, waiting for the body of an arm.
    Match {
        span: &'hir dyn Spanned,
        end_label: Label,
        value: Address<'a, 'hir>,
        linear: Linear<'a, 'hir>,
        /// Arms still to assemble, in reverse so they pop in order.
        bodies: Vec<(&'hir hir::ExprMatchBranch<'hir>, Label, DanglingScope)>,
        all_diverge: bool,
        /// The arm being assembled, along with the scope to pop after it.
        pending: Option<(&'hir hir::ExprMatchBranch<'hir>, ScopeHandle)>,
    },
    /// `<target> <op>= <expr>`, waiting for the target or for the value.
    AssignBinop {
        hir: &'hir hir::ExprBinary,
        span: &'hir dyn Spanned,
        /// The target, once it is known.
        inst_target: Option<InstTarget>,
        /// A global slot the target has to be written back to.
        writeback: Option<(usize, Address<'a, 'hir>)>,
        pending: Any<'a, 'hir>,
    },
    /// `break <expr>`, waiting for the operand.
    Break {
        span: &'hir dyn Spanned,
        break_label: Label,
        needs: Any<'a, 'hir>,
    },
    /// `return <expr>`, waiting for the operand.
    Return {
        span: &'hir dyn Spanned,
        addr: Any<'a, 'hir>,
    },
    /// `yield <expr>`, waiting for the operand.
    Yield {
        span: &'hir dyn Spanned,
        addr: Address<'a, 'hir>,
        out: Output,
    },
    /// A range, waiting for one of its bounds.
    Range {
        hir: &'hir hir::ExprRange,
        span: &'hir dyn Spanned,
        /// The bound which has already been assembled.
        first: Option<Address<'a, 'hir>>,
        /// The bound being assembled.
        pending: Any<'a, 'hir>,
    },
    /// `<expr>(..)`, waiting for the function value.
    CallFunction {
        hir: &'hir hir::ExprCall<'hir>,
        span: &'hir dyn Spanned,
        function: Any<'a, 'hir>,
    },
    /// `#[builtin] format!(..)`, waiting for the value being formatted.
    ///
    /// The value is assembled into the needs of the surrounding expression and
    /// formatted in place, so this step owns no addresses.
    Format { format: &'hir hir::BuiltInFormat },
    /// `#[builtin] template!(..)`, waiting for one of its expansions.
    Template {
        hir: &'hir hir::BuiltInTemplate<'hir>,
        at: usize,
        linear: Linear<'a, 'hir>,
        size_hint: usize,
        expansions: usize,
    },
    /// A fixed set of expressions assembled into deferred addresses.
    ///
    /// Unlike [`Step::Seq`] the addresses are deferred rather than linear, so
    /// each value binds where it naturally lands instead of being copied into
    /// a run.
    Defers {
        span: &'hir dyn Spanned,
        ids: Vec<hir::ExprId>,
        at: usize,
        addrs: Vec<Any<'a, 'hir>>,
        tail: DefersTail<'hir>,
    },
    /// `<var> = <value>`, waiting for the value.
    AssignVar {
        span: &'hir dyn Spanned,
        addr: Address<'a, 'hir>,
    },
    /// `<static> = <value>`, waiting for the value.
    AssignStatic {
        span: &'hir dyn Spanned,
        slot: usize,
        value: Any<'a, 'hir>,
    },
    /// A sequence of expressions assembled into a linear run of addresses,
    /// waiting for one of its elements.
    Seq {
        span: &'hir dyn Spanned,
        ids: Vec<hir::ExprId>,
        at: usize,
        linear: Linear<'a, 'hir>,
        tail: SeqTail<'a, 'hir>,
    },
    /// `<expr>.await`, waiting for the operand.
    Await {
        span: &'hir dyn Spanned,
        addr: Any<'a, 'hir>,
    },
    /// `<expr>?`, waiting for the operand.
    Try {
        span: &'hir dyn Spanned,
        addr: Any<'a, 'hir>,
    },
    /// `<expr>.<field>`, waiting for the expression being accessed.
    FieldAccess {
        hir: &'hir hir::ExprFieldAccess<'hir>,
        span: &'hir dyn Spanned,
        addr: Any<'a, 'hir>,
    },
    /// `<target>[<index>]`, waiting for the target.
    IndexTarget {
        hir: &'hir hir::ExprIndex,
        span: &'hir dyn Spanned,
        target: Any<'a, 'hir>,
        index: Any<'a, 'hir>,
    },
    /// `<target>[<index>]`, waiting for the index.
    IndexIndex {
        span: &'hir dyn Spanned,
        target: Any<'a, 'hir>,
        index: Any<'a, 'hir>,
    },
    /// `<lhs> <op> <rhs>`, waiting for the left-hand side.
    BinaryLhs {
        hir: &'hir hir::ExprBinary,
        span: &'hir dyn Spanned,
        a: Any<'a, 'hir>,
        b: Any<'a, 'hir>,
    },
    /// `<lhs> <op> <rhs>`, waiting for the right-hand side.
    BinaryRhs {
        hir: &'hir hir::ExprBinary,
        span: &'hir dyn Spanned,
        a: Any<'a, 'hir>,
        b: Any<'a, 'hir>,
    },
    /// `<lhs> && <rhs>` or `<lhs> || <rhs>`, waiting for the left-hand side.
    ///
    /// Conditional operators assemble both of their operands into the needs of
    /// the surrounding expression, so this step owns no addresses and is
    /// transparent when the active needs is resolved.
    ConditionalLhs {
        hir: &'hir hir::ExprBinary,
        span: &'hir dyn Spanned,
    },
    /// `<lhs> && <rhs>` or `<lhs> || <rhs>`, waiting for the right-hand side.
    ConditionalRhs {
        span: &'hir dyn Spanned,
        end_label: Label,
    },
}

/// The state of a `for .. in .. { .. }` which is waiting for its body.
///
/// Stored behind a [`Box`] since loops are rare relative to the expressions
/// which dominate the work stack, and this is one of the largest states.
struct StepFor<'a, 'hir> {
    span: &'hir dyn Spanned,
    continue_label: Label,
    end_label: Label,
    break_label: Label,
    inner_loop_scope: ScopeHandle,
    bindings: Linear<'a, 'hir>,
    binding: Address<'a, 'hir>,
    into_iter: Address<'a, 'hir>,
    iter: Address<'a, 'hir>,
    next_offset: Option<Address<'a, 'hir>>,
    body: Any<'a, 'hir>,
}

/// The state of a `match .. { .. }` which is waiting for the guard of an arm.
///
/// Boxed for the same reason as [`StepFor`].
struct StepMatchGuard<'a, 'hir> {
    hir: &'hir hir::ExprMatch<'hir>,
    span: &'hir dyn Spanned,
    value: Address<'a, 'hir>,
    end_label: Label,
    linear: Linear<'a, 'hir>,
    branches: Vec<(Label, DanglingScope)>,
    at: usize,
    branch_label: Label,
    match_false: Label,
    pattern_scope: ScopeHandle,
    scope: ScopeHandle,
    pat: Pattern,
    cond: Any<'a, 'hir>,
}

/// The state of a `select { .. }` which is waiting for a branch body or the
/// default.
///
/// Boxed for the same reason as [`StepFor`].
struct StepSelect<'a, 'hir> {
    hir: &'hir hir::ExprSelect<'hir>,
    span: &'hir dyn Spanned,
    end_label: Label,
    select_label: Label,
    value_addr: Address<'a, 'hir>,
    linear: Linear<'a, 'hir>,
    /// Branches still to assemble, in reverse so they pop in order.
    branches: Vec<(Label, &'hir hir::ExprSelectBranch<'hir>)>,
    default_branch: Option<(hir::ExprId, Label)>,
    /// The scope to pop once the branch body being assembled is done.
    pending: Option<ScopeHandle>,
}

impl<'a, 'hir> Step<'a, 'hir> {
    /// The needs which the pending child of this step is assembled into.
    ///
    /// Transparent steps return `None`, in which case the needs of the
    /// enclosing step is used instead.
    fn needs<'this, 'o>(&'this mut self) -> Option<&'this mut (dyn Needs<'a, 'hir> + 'o)>
    where
        'a: 'o,
        'hir: 'o,
        'o: 'this,
    {
        let needs: &mut (dyn Needs<'a, 'hir> + 'o) = match self {
            Step::Unary { addr, .. }
            | Step::Await { addr, .. }
            | Step::Try { addr, .. }
            | Step::FieldAccess { addr, .. } => addr,
            Step::IndexTarget { target, .. } => target,
            Step::IndexIndex { index, .. } => index,
            Step::BinaryLhs { a, .. } => a,
            Step::BinaryRhs { b, .. } => b,
            Step::ConditionalLhs { .. } | Step::ConditionalRhs { .. } | Step::Format { .. } => {
                return None
            }
            Step::Block { needs, .. } => needs.as_mut()?,
            Step::LoopCondition { value, .. } => value,
            Step::Loop { body, .. } => body,
            Step::ForIter { iter, .. } => iter,
            Step::Return { addr, .. } => addr,
            Step::Break { needs, .. } => needs,
            Step::AssignBinop { pending, .. } => pending,
            Step::Seq { linear, at, .. } => &mut linear[*at],
            Step::Defers { addrs, at, .. } => &mut addrs[*at],
            Step::Template { linear, at, .. } => &mut linear[*at],
            Step::AssignVar { addr, .. } => addr,
            Step::AssignStatic { value, .. } => value,
            Step::CallFunction { function, .. } => function,
            Step::Range { pending, .. } => pending,
            Step::Yield { addr, .. } => addr,
            Step::MatchValue { value, .. } => value,
            Step::MatchGuard(s) => &mut s.cond,
            Step::Select(..) => return None,
            Step::Match { .. } => return None,
            Step::For(s) => &mut s.body,
            Step::IfCondition { value, .. } => value,
            Step::If { ignore, .. } => ignore.as_mut()?,
        };

        Some(needs)
    }

    /// Release every address owned by this step.
    ///
    /// Only used when assembly is abandoned due to an error, since the
    /// completion of a step frees its addresses in a specific order.
    fn free(self) -> compile::Result<()> {
        match self {
            Step::Unary { addr, .. }
            | Step::Await { addr, .. }
            | Step::Try { addr, .. }
            | Step::FieldAccess { addr, .. } => addr.free(),
            Step::IndexTarget { target, index, .. } | Step::IndexIndex { target, index, .. } => {
                index.free()?;
                target.free()
            }
            Step::BinaryLhs { a, b, .. } | Step::BinaryRhs { a, b, .. } => {
                a.free()?;
                b.free()
            }
            Step::ConditionalLhs { .. } | Step::ConditionalRhs { .. } | Step::Format { .. } => {
                Ok(())
            }
            Step::Block { needs, .. } => match needs {
                Some(needs) => needs.free(),
                None => Ok(()),
            },
            Step::LoopCondition { linear, value, .. } => {
                value.free()?;
                linear.free()
            }
            Step::Loop { linear, body, .. } => {
                body.free()?;
                linear.free()
            }
            Step::ForIter { iter, .. } => iter.free(),
            Step::Return { addr, .. } => addr.free(),
            Step::Break { needs, .. } => needs.free(),
            Step::AssignBinop {
                writeback, pending, ..
            } => {
                pending.free()?;

                if let Some((_, target)) = writeback {
                    target.free()?;
                }

                Ok(())
            }
            Step::CallFunction { function, .. } => function.free(),
            Step::Template { linear, .. } => linear.free(),
            Step::AssignVar { addr, .. } => addr.free(),
            Step::AssignStatic { value, .. } => value.free(),
            Step::Defers { addrs, .. } => {
                for addr in addrs.into_iter().rev() {
                    addr.free()?;
                }

                Ok(())
            }
            Step::Seq { linear, tail, .. } => {
                if let SeqTail::CallExpr { function, .. } = tail {
                    function.free()?;
                }

                linear.free()
            }
            Step::Range { first, pending, .. } => {
                pending.free()?;

                if let Some(first) = first {
                    first.free()?;
                }

                Ok(())
            }
            Step::Yield { addr, .. } => addr.free(),
            Step::MatchValue { value, .. } => value.free(),
            Step::Select(s) => {
                let StepSelect {
                    value_addr, linear, ..
                } = Box::into_inner(s);
                value_addr.free()?;
                linear.free()
            }
            Step::MatchGuard(s) => {
                let StepMatchGuard {
                    value,
                    linear,
                    cond,
                    ..
                } = Box::into_inner(s);
                cond.free()?;
                value.free()?;
                linear.free()
            }
            Step::Match { value, linear, .. } => {
                value.free()?;
                linear.free()
            }
            Step::For(s) => {
                let StepFor {
                    bindings,
                    binding,
                    into_iter,
                    iter,
                    next_offset,
                    body,
                    ..
                } = Box::into_inner(s);
                body.free()?;
                bindings.free()?;

                if let Some(next_offset) = next_offset {
                    next_offset.free()?;
                }

                binding.free()?;
                into_iter.free()?;
                iter.free()
            }
            Step::IfCondition { linear, value, .. } => {
                value.free()?;
                linear.free()
            }
            Step::If { linear, ignore, .. } => {
                if let Some(ignore) = ignore {
                    ignore.free()?;
                }

                linear.free()
            }
        }
    }
}

/// The result of completing a [`Step`].
///
/// The variants differ considerably in size, but this is only ever returned and
/// immediately destructured - never stored - so boxing the larger one would
/// trade a move for an allocation on every step which has a second child.
#[allow(clippy::large_enum_variant)]
enum Completed<'a, 'hir> {
    /// The step is done and produced the given assembly outcome.
    Done(Asm<'hir>),
    /// The step needs another child assembled first. The successor step is
    /// handed back together with what to assemble into it.
    Next(Step<'a, 'hir>, Current<'hir>),
}

/// What to do with a set of deferred addresses once they are assembled.
enum DefersTail<'hir> {
    /// A small tuple, which has a dedicated instruction per size.
    Tuple,
    /// `<expr>.<field> = <value>`.
    AssignField {
        hir: &'hir hir::ExprFieldAccess<'hir>,
    },
    /// `<target>[<index>] = <value>`.
    AssignIndex,
}

/// What to do with a sequence once all of its elements are assembled.
enum SeqTail<'a, 'hir> {
    /// A literal vector.
    Vec,
    /// A literal tuple.
    Tuple,
    /// A literal object.
    Object { hir: &'hir hir::ExprObject<'hir> },
    /// The futures of a `select`.
    Select {
        hir: &'hir hir::ExprSelect<'hir>,
        end_label: Label,
    },
    /// A call through a variable.
    CallVar { name: hir::Variable, args: usize },
    /// An associated call, whose target is the first element of the run.
    CallAssociated { hash: Hash, args: usize },
    /// A call to a known function.
    CallMeta { hash: Hash, args: usize },
    /// A call through an assembled function value.
    CallExpr {
        function: Any<'a, 'hir>,
        args: usize,
    },
}

/// What the work stack is currently assembling.
///
/// Blocks are not expressions in the HIR, but they nest through the same
/// constructs, so the stack has to be able to descend into either.
#[derive(Clone, Copy)]
enum Current<'hir> {
    Expr(&'hir hir::Expr<'hir>),
    Block(&'hir hir::Block<'hir>),
}

impl<'hir> Current<'hir> {
    fn span(self) -> &'hir dyn Spanned {
        match self {
            Current::Expr(hir) => hir,
            Current::Block(hir) => hir,
        }
    }
}

/// Assemble an expression.
///
/// Chained expressions are walked over an explicit work stack rather than
/// through recursion, so that the depth of a chain is bounded by
/// [`Options::max_depth`] and heap memory instead of by the size of the call
/// stack. Lexically nested expressions - blocks, branches, loops, closures -
/// still recurse through this function, but their depth is bounded by the
/// nesting limit imposed while parsing.
#[instrument_ast(span = hir)]
fn expr<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::Expr<'hir>,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let mut steps = Vec::new();

    let result = expr_stack(cx, hir, needs, &mut steps);

    // Steps only remain if assembly was abandoned, in which case the addresses
    // they own still have to be released.
    if result.is_err() {
        for step in steps.drain(..).rev() {
            _ = step.free();
        }
    }

    result
}

fn expr_stack<'a, 'hir, 'o>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::Expr<'hir>,
    outer: &mut (dyn Needs<'a, 'hir> + 'o),
    steps: &mut Vec<Step<'a, 'hir>>,
) -> compile::Result<Asm<'hir>>
where
    'a: 'o,
    'hir: 'o,
{
    let mut current = Current::Expr(hir);

    'work: loop {
        let mut asm = 'descend: loop {
            let span: &'hir dyn Spanned = current.span();

            // The needs which `current` is assembled into. Resolving it borrows
            // the work stack, which is why the arms below which push a step
            // never touch it.
            let needs: &mut (dyn Needs<'a, 'hir> + 'o) =
                match steps.iter_mut().rev().find_map(Step::needs) {
                    Some(needs) => needs,
                    None => outer,
                };

            macro_rules! descend {
                ($step:expr, $child:expr) => {{
                    let step = $step;

                    if steps.len() >= cx.options.max_depth {
                        step.free()?;

                        return Err(compile::Error::new(
                            span,
                            ErrorKind::MaxDepth {
                                max: cx.options.max_depth,
                            },
                        ));
                    }

                    steps.try_push(step).with_span(span)?;
                    current = Current::Expr($child);
                    continue 'descend;
                }};
            }

            let current_expr = match current {
                Current::Expr(hir) => hir,
                Current::Block(block_hir) => {
                    let needs: &mut (dyn Needs<'a, 'hir> + 'o) =
                        match steps.iter_mut().rev().find_map(Step::needs) {
                            Some(needs) => needs,
                            None => outer,
                        };

                    match block_start(cx, block_hir, needs)? {
                        Completed::Done(asm) => break asm,
                        Completed::Next(step, child) => {
                            if steps.len() >= cx.options.max_depth {
                                step.free()?;

                                return Err(compile::Error::new(
                                    span,
                                    ErrorKind::MaxDepth {
                                        max: cx.options.max_depth,
                                    },
                                ));
                            }

                            steps.try_push(step).with_span(span)?;
                            current = child;
                            continue 'descend;
                        }
                    }
                }
            };

            break match current_expr.kind {
                hir::ExprKind::Group(hir) => {
                    current = Current::Expr(cx.exprs.get(hir));
                    continue 'descend;
                }
                hir::ExprKind::Block(block_hir) => {
                    current = Current::Block(block_hir);
                    continue 'descend;
                }
                hir::ExprKind::If(if_hir) => {
                    let end_label = cx.asm.new_label("if_end");

                    let values = if_hir
                        .branches
                        .iter()
                        .flat_map(|c| c.condition.count())
                        .max()
                        .unwrap_or(0);

                    let linear = cx.scopes.linear(span, values)?;

                    match if_condition(cx, if_hir, span, end_label, linear, Vec::new(), 0, needs)? {
                        Completed::Done(asm) => break asm,
                        Completed::Next(step, child) => {
                            steps.try_push(step).with_span(span)?;
                            current = child;
                            continue 'descend;
                        }
                    }
                }
                hir::ExprKind::Break(break_hir) => {
                    let (break_label, output) = break_target(cx, break_hir, span)?;

                    let Some(e) = break_hir.expr else {
                        if let Some(out) = output {
                            cx.asm.push(inst::Kind::unit(out), span)?;
                        }

                        break break_finish(cx, span, break_label)?;
                    };

                    let Some(output) = output else {
                        return Err(compile::Error::new(span, ErrorKind::BreakUnsupportedValue));
                    };

                    let needs = match output.as_addr() {
                        Some(addr) => Any::assigned(span, cx.scopes, addr),
                        None => Any::ignore(span),
                    };

                    descend!(
                        Step::Break {
                            span,
                            break_label,
                            needs,
                        },
                        cx.exprs.get(e)
                    );
                }
                hir::ExprKind::Return(Some(e)) => {
                    descend!(
                        Step::Return {
                            span,
                            addr: cx.scopes.defer(span).with_name("return value"),
                        },
                        cx.exprs.get(e)
                    );
                }
                hir::ExprKind::Yield(Some(e)) => {
                    let out = needs.alloc_output()?;

                    descend!(
                        Step::Yield {
                            span,
                            addr: cx.scopes.alloc(span)?.with_name("yield argument"),
                            out,
                        },
                        cx.exprs.get(e)
                    );
                }
                hir::ExprKind::Range(range_hir) => {
                    let (a, _) = range_operands(range_hir);

                    let Some(a) = a else {
                        // `..` has no bounds to assemble.
                        if let Some(out) = needs.try_alloc_output()? {
                            cx.asm.push(
                                inst::Kind::Range {
                                    range: InstRange::RangeFull,
                                    out,
                                },
                                span,
                            )?;
                        }

                        break Asm::new(span, ());
                    };

                    let (_, b) = range_operands(range_hir);

                    // Both bounds are read once the second one has been
                    // assembled, so the first one only keeps the address it was
                    // assembled into when the second one cannot write to it.
                    let pending = match b {
                        Some(b) if !writes_nothing(cx, b) => cx.scopes.alloc_any(span)?,
                        _ => cx.scopes.defer(cx.exprs.get(a)),
                    };

                    let a = cx.exprs.get(a);

                    descend!(
                        Step::Range {
                            hir: range_hir,
                            span,
                            first: None,
                            pending,
                        },
                        a
                    );
                }
                hir::ExprKind::Vec(seq_hir) => {
                    match seq_start(cx, span, seq_hir.items, SeqTail::Vec, needs)? {
                        Completed::Done(asm) => break asm,
                        Completed::Next(step, child) => {
                            steps.try_push(step).with_span(span)?;
                            current = child;
                            continue 'descend;
                        }
                    }
                }
                // Small tuples have dedicated instructions and are assembled
                // by `expr_tuple`; larger ones go over the stack.
                hir::ExprKind::Tuple(seq_hir) if seq_hir.items.len() > 4 => {
                    match seq_start(cx, span, seq_hir.items, SeqTail::Tuple, needs)? {
                        Completed::Done(asm) => break asm,
                        Completed::Next(step, child) => {
                            steps.try_push(step).with_span(span)?;
                            current = child;
                            continue 'descend;
                        }
                    }
                }
                hir::ExprKind::Call(call_hir) => {
                    let args = call_hir.args.len();

                    let (ids, tail) = match call_hir.call {
                        hir::Call::Var { name, .. } => {
                            (call_ids(call_hir, None)?, SeqTail::CallVar { name, args })
                        }
                        hir::Call::Associated { target, hash } => (
                            call_ids(call_hir, Some(target))?,
                            SeqTail::CallAssociated {
                                hash,
                                args: args + 1,
                            },
                        ),
                        hir::Call::Meta { hash } => {
                            (call_ids(call_hir, None)?, SeqTail::CallMeta { hash, args })
                        }
                        hir::Call::Expr { expr: e } => {
                            descend!(
                                Step::CallFunction {
                                    hir: call_hir,
                                    span,
                                    function: cx.scopes.defer(span),
                                },
                                cx.exprs.get(e)
                            );
                        }
                        // Inside of a constant the call is assembled like any
                        // other, since the interior unit contains the constant
                        // function being called. Outside of one it is folded
                        // into the value it evaluates to.
                        hir::Call::ConstFn { id, .. } if cx.const_eval => {
                            let hash = cx.q.const_fn_hash(id).with_span(span)?;
                            (call_ids(call_hir, None)?, SeqTail::CallMeta { hash, args })
                        }
                        hir::Call::ConstFn { id, .. } => {
                            let value = cx.call_const_fn(span, id, call_hir.args)?;
                            const_(cx, &value, span, needs)?;
                            break Asm::new(span, ());
                        }
                    };

                    match seq_start_ids(cx, span, ids, tail, needs)? {
                        Completed::Done(asm) => break asm,
                        Completed::Next(step, child) => {
                            steps.try_push(step).with_span(span)?;
                            current = child;
                            continue 'descend;
                        }
                    }
                }
                hir::ExprKind::Tuple(seq_hir) if seq_hir.items.is_empty() => {
                    cx.asm.push(inst::Kind::unit(needs.alloc_output()?), span)?;
                    break Asm::new(span, ());
                }
                // Small tuples have dedicated instructions which take the
                // addresses of their elements directly.
                hir::ExprKind::Tuple(seq_hir) if seq_hir.items.len() <= 4 => {
                    let mut ids = Vec::try_with_capacity(seq_hir.items.len())?;
                    let mut addrs = Vec::try_with_capacity(seq_hir.items.len())?;

                    // Every element is read once the last one has been
                    // assembled, so an element only keeps the address it was
                    // assembled into when nothing after it can write to it.
                    for (n, id) in seq_hir.items.iter().enumerate() {
                        ids.try_push(*id)?;

                        let written = seq_hir.items[n + 1..]
                            .iter()
                            .any(|id| !writes_nothing(cx, *id));

                        addrs.try_push(if written {
                            cx.scopes.alloc_any(span)?
                        } else {
                            cx.scopes.defer(span)
                        })?;
                    }

                    let first = cx.exprs.get(ids[0]);

                    descend!(
                        Step::Defers {
                            span,
                            ids,
                            at: 0,
                            addrs,
                            tail: DefersTail::Tuple,
                        },
                        first
                    );
                }
                hir::ExprKind::Assign(assign_hir) => {
                    let lhs = cx.exprs.get(assign_hir.lhs);
                    let rhs = cx.exprs.get(assign_hir.rhs);

                    match lhs.kind {
                        hir::ExprKind::Variable(name) => {
                            let var = cx.scopes.get(&mut cx.q, span, name)?;

                            descend!(
                                Step::AssignVar {
                                    span,
                                    addr: Address::assigned(var.span, cx.scopes, var.addr),
                                },
                                rhs
                            );
                        }
                        hir::ExprKind::Static(hash) => {
                            let slot = global_slot(cx, hash, span)?;

                            descend!(
                                Step::AssignStatic {
                                    span,
                                    slot,
                                    value: cx.scopes.defer(rhs),
                                },
                                rhs
                            );
                        }
                        hir::ExprKind::FieldAccess(field_access) => {
                            let access_expr = cx.exprs.get(field_access.expr);

                            let mut ids = Vec::try_with_capacity(2)?;
                            ids.try_push(field_access.expr)?;
                            ids.try_push(assign_hir.rhs)?;

                            let mut addrs = Vec::try_with_capacity(2)?;
                            addrs.try_push(cx.scopes.defer(access_expr))?;
                            addrs.try_push(cx.scopes.defer(rhs))?;

                            descend!(
                                Step::Defers {
                                    span,
                                    ids,
                                    at: 0,
                                    addrs,
                                    tail: DefersTail::AssignField { hir: field_access },
                                },
                                access_expr
                            );
                        }
                        hir::ExprKind::Index(index_get) => {
                            let index_target = cx.exprs.get(index_get.target);
                            let index_index = cx.exprs.get(index_get.index);

                            let mut ids = Vec::try_with_capacity(3)?;
                            ids.try_push(index_get.target)?;
                            ids.try_push(index_get.index)?;
                            ids.try_push(assign_hir.rhs)?;

                            let mut addrs = Vec::try_with_capacity(3)?;
                            addrs.try_push(cx.scopes.defer(index_target))?;
                            addrs.try_push(cx.scopes.defer(index_index))?;
                            addrs.try_push(cx.scopes.defer(rhs))?;

                            descend!(
                                Step::Defers {
                                    span,
                                    ids,
                                    at: 0,
                                    addrs,
                                    tail: DefersTail::AssignIndex,
                                },
                                index_target
                            );
                        }
                        _ => {
                            return Err(compile::Error::new(
                                span,
                                ErrorKind::UnsupportedAssignExpr,
                            ));
                        }
                    }
                }
                hir::ExprKind::Template(template) => {
                    let linear = cx.scopes.linear(template, template.exprs.len())?;

                    match template_next(cx, template, 0, linear, 0, 0, needs)? {
                        Completed::Done(asm) => break asm,
                        Completed::Next(step, child) => {
                            steps.try_push(step).with_span(span)?;
                            current = child;
                            continue 'descend;
                        }
                    }
                }
                hir::ExprKind::Select(select_hir) => {
                    cx.contexts.try_push(span)?;
                    cx.select_branches.clear();

                    let end_label = cx.asm.new_label("select_end");

                    for branch in select_hir.branches {
                        let label = cx.asm.new_label("select_branch");
                        cx.select_branches.try_push((label, branch))?;
                    }

                    let mut ids = Vec::try_with_capacity(select_hir.exprs.len())?;

                    for id in select_hir.exprs {
                        ids.try_push(*id)?;
                    }

                    let tail = SeqTail::Select {
                        hir: select_hir,
                        end_label,
                    };

                    match seq_start_ids(cx, span, ids, tail, needs)? {
                        Completed::Done(asm) => break asm,
                        Completed::Next(step, child) => {
                            steps.try_push(step).with_span(span)?;
                            current = child;
                            continue 'descend;
                        }
                    }
                }
                hir::ExprKind::Format(format) => {
                    descend!(Step::Format { format }, cx.exprs.get(format.value));
                }
                hir::ExprKind::Object(object_hir) => {
                    let mut ids = Vec::try_with_capacity(object_hir.assignments.len())?;

                    for assign in object_hir.assignments {
                        ids.try_push(assign.assign)?;
                    }

                    match seq_start_ids(cx, span, ids, SeqTail::Object { hir: object_hir }, needs)?
                    {
                        Completed::Done(asm) => break asm,
                        Completed::Next(step, child) => {
                            steps.try_push(step).with_span(span)?;
                            current = child;
                            continue 'descend;
                        }
                    }
                }
                hir::ExprKind::Match(match_hir) => {
                    descend!(
                        Step::MatchValue {
                            hir: match_hir,
                            span,
                            value: cx.scopes.defer(span),
                        },
                        cx.exprs.get(match_hir.expr)
                    );
                }
                hir::ExprKind::For(for_hir) => {
                    descend!(
                        Step::ForIter {
                            hir: for_hir,
                            span,
                            iter: cx.scopes.defer(span).with_name("iter"),
                        },
                        cx.exprs.get(for_hir.iter)
                    );
                }
                hir::ExprKind::Loop(loop_hir) => {
                    let continue_label = cx.asm.new_label("while_continue");
                    let then_label = cx.asm.new_label("while_then");
                    let end_label = cx.asm.new_label("while_end");
                    let break_label = cx.asm.new_label("while_break");
                    let output = Some(needs.alloc_output()?);

                    cx.breaks.push(Break {
                        label: loop_hir.label,
                        continue_label: Some(continue_label.try_clone()?),
                        break_label: break_label.try_clone()?,
                        output,
                        drop: None,
                    })?;

                    cx.asm.label(&continue_label)?;

                    let count = loop_hir
                        .condition
                        .and_then(|c| c.count())
                        .unwrap_or_default();
                    let linear = cx.scopes.linear(span, count)?;

                    if let Some(condition_hir) = loop_hir.condition {
                        let (scope, value, child) = match *condition_hir {
                            hir::Condition::Expr(_, e) => {
                                let e = cx.exprs.get(e);
                                let scope = cx.scopes.child(e)?;
                                let value =
                                    cx.scopes.alloc_any(e)?.with_name("expression condition");
                                (scope, value, e)
                            }
                            hir::Condition::ExprLet(let_hir) => {
                                let scope = cx.scopes.child(let_hir)?;
                                let e = cx.exprs.get(let_hir.expr);
                                (scope, cx.scopes.defer(let_hir), e)
                            }
                        };

                        let step = Step::LoopCondition {
                            hir: loop_hir,
                            span,
                            continue_label,
                            then_label,
                            end_label,
                            break_label,
                            linear,
                            scope,
                            value,
                        };

                        steps.try_push(step).with_span(span)?;
                        current = Current::Expr(child);
                        continue 'descend;
                    }

                    let condition_scope = None;

                    let step = Step::Loop {
                        span,
                        continue_label,
                        end_label,
                        break_label,
                        condition_scope,
                        linear,
                        body: Any::ignore(span),
                    };

                    if steps.len() >= cx.options.max_depth {
                        step.free()?;

                        return Err(compile::Error::new(
                            span,
                            ErrorKind::MaxDepth {
                                max: cx.options.max_depth,
                            },
                        ));
                    }

                    steps.try_push(step).with_span(span)?;
                    current = Current::Block(&loop_hir.body);
                    continue 'descend;
                }
                hir::ExprKind::Unary(hir) => {
                    descend!(
                        Step::Unary {
                            hir,
                            span,
                            addr: cx.scopes.defer(span),
                        },
                        cx.exprs.get(hir.expr)
                    );
                }
                hir::ExprKind::Await(hir) => {
                    descend!(
                        Step::Await {
                            span,
                            addr: cx.scopes.defer(span),
                        },
                        cx.exprs.get(hir)
                    );
                }
                hir::ExprKind::Try(hir) => {
                    descend!(
                        Step::Try {
                            span,
                            addr: cx.scopes.defer(span),
                        },
                        cx.exprs.get(hir)
                    );
                }
                // Accessing a tuple field of a variable doesn't require any
                // sub-expression to be assembled first.
                hir::ExprKind::FieldAccess(hir)
                    if matches!(
                        (cx.exprs.get(hir.expr).kind, hir.expr_field),
                        (hir::ExprKind::Variable(..), hir::ExprField::Index(..))
                    ) =>
                {
                    let (hir::ExprKind::Variable(name), hir::ExprField::Index(index)) =
                        (cx.exprs.get(hir.expr).kind, hir.expr_field)
                    else {
                        unreachable!();
                    };

                    let var = cx.scopes.get(&mut cx.q, span, name)?;

                    cx.asm.push_with_comment(
                        inst::Kind::TupleIndexGetAt {
                            addr: var.addr,
                            index,
                            out: needs.alloc_output()?,
                        },
                        span,
                        &var,
                    )?;

                    Asm::new(span, ())
                }
                hir::ExprKind::FieldAccess(hir) => {
                    descend!(
                        Step::FieldAccess {
                            hir,
                            span,
                            addr: cx.scopes.defer(span),
                        },
                        cx.exprs.get(hir.expr)
                    );
                }
                hir::ExprKind::Index(hir) => {
                    descend!(
                        Step::IndexTarget {
                            hir,
                            span,
                            target: cx.scopes.defer(span),
                            index: cx.scopes.defer(span),
                        },
                        cx.exprs.get(hir.target)
                    );
                }
                // Assigning operators operate on the stack in special ways and
                // don't form a chain.
                hir::ExprKind::Binary(hir) if hir.op.is_assign() => {
                    let lhs = cx.exprs.get(hir.lhs);

                    match lhs.kind {
                        hir::ExprKind::Variable(name) => {
                            let var = cx.scopes.get(&mut cx.q, lhs, name)?;

                            descend!(
                                Step::AssignBinop {
                                    hir,
                                    span,
                                    inst_target: Some(InstTarget::Address(var.addr)),
                                    writeback: None,
                                    pending: cx.scopes.defer(cx.exprs.get(hir.rhs)),
                                },
                                cx.exprs.get(hir.rhs)
                            );
                        }
                        // A static isn't addressable, so it is read into a
                        // scratch address which is written back to the slot
                        // once the operation has been applied.
                        hir::ExprKind::Static(hash) => {
                            let slot = global_slot(cx, hash, span)?;

                            let mut target = cx.scopes.defer(lhs);

                            converge!(static_item(cx, hash, lhs, &mut target)?, free(target));

                            let target = target.into_addr()?;
                            let inst_target = InstTarget::Address(target.addr());

                            descend!(
                                Step::AssignBinop {
                                    hir,
                                    span,
                                    inst_target: Some(inst_target),
                                    writeback: Some((slot, target)),
                                    pending: cx.scopes.defer(cx.exprs.get(hir.rhs)),
                                },
                                cx.exprs.get(hir.rhs)
                            );
                        }
                        hir::ExprKind::FieldAccess(field_access) => {
                            let access_expr = cx.exprs.get(field_access.expr);

                            descend!(
                                Step::AssignBinop {
                                    hir,
                                    span,
                                    inst_target: None,
                                    writeback: None,
                                    pending: cx.scopes.defer(access_expr),
                                },
                                access_expr
                            );
                        }
                        _ => {
                            return Err(compile::Error::new(
                                span,
                                ErrorKind::UnsupportedBinaryExpr,
                            ));
                        }
                    }
                }
                hir::ExprKind::Binary(hir) if hir.op.is_conditional() => {
                    // Both sides write where the expression writes, so that has
                    // to be an address of its own.
                    //
                    // A side which is a variable hands the address of that
                    // variable over rather than writing anything, so leaving it
                    // to the sides would have the other one write into whichever
                    // variable this one named.
                    needs.try_alloc_output()?;
                    descend!(Step::ConditionalLhs { hir, span }, cx.exprs.get(hir.lhs));
                }
                hir::ExprKind::Binary(hir) => {
                    // Both operands are read once the second one has been
                    // assembled, so the first one only keeps the address it was
                    // assembled into when the second one cannot write to it.
                    let a = if writes_nothing(cx, hir.rhs) {
                        cx.scopes.defer(span)
                    } else {
                        cx.scopes.alloc_any(span)?
                    };

                    descend!(
                        Step::BinaryLhs {
                            hir,
                            span,
                            a,
                            b: cx.scopes.defer(span),
                        },
                        cx.exprs.get(hir.lhs)
                    );
                }
                _ => expr_leaf(cx, current_expr, span, needs)?,
            };
        };

        // Complete each pending step in turn, descending again whenever a step
        // has another child which has to be assembled first.
        loop {
            let Some(step) = steps.pop() else {
                return Ok(asm);
            };

            let needs: &mut (dyn Needs<'a, 'hir> + 'o) =
                match steps.iter_mut().rev().find_map(Step::needs) {
                    Some(needs) => needs,
                    None => outer,
                };

            match complete(cx, step, asm, needs)? {
                Completed::Done(next) => {
                    asm = next;
                }
                Completed::Next(step, child) => {
                    steps.try_push(step).with_span(child.span())?;
                    current = child;
                    continue 'work;
                }
            }
        }
    }
}

/// Assemble the next expansion of a template, or finish it.
///
/// Literal strings are emitted directly rather than assembled, so this walks
/// forward until it finds an expansion which needs a child.
#[allow(clippy::too_many_arguments)]
fn template_next<'a, 'hir, 'o>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::BuiltInTemplate<'hir>,
    mut at: usize,
    linear: Linear<'a, 'hir>,
    mut size_hint: usize,
    mut expansions: usize,
    needs: &mut (dyn Needs<'a, 'hir> + 'o),
) -> compile::Result<Completed<'a, 'hir>>
where
    'a: 'o,
    'hir: 'o,
{
    while let Some(&e) = hir.exprs.get(at) {
        let e = cx.exprs.get(e);

        if let hir::ExprKind::Lit(hir::Lit::Str(s)) = e.kind {
            size_hint += s.len();
            let slot = cx.q.unit.new_static_string(hir, s)?;

            cx.asm.push(
                inst::Kind::String {
                    slot,
                    out: linear[at].output(),
                },
                hir,
            )?;

            at += 1;
            continue;
        }

        expansions += 1;

        return Ok(Completed::Next(
            Step::Template {
                hir,
                at,
                linear,
                size_hint,
                expansions,
            },
            Current::Expr(e),
        ));
    }

    template_diagnostics(cx, hir, expansions)?;

    cx.asm.push(
        inst::Kind::StringConcat {
            addr: linear.addr(),
            len: hir.exprs.len(),
            size_hint,
            out: needs.alloc_output()?,
        },
        hir,
    )?;

    linear.free()?;
    Ok(Completed::Done(Asm::new(hir, ())))
}

/// Report a template literal which expands nothing.
fn template_diagnostics(
    cx: &mut Ctxt<'_, '_, '_>,
    hir: &hir::BuiltInTemplate<'_>,
    expansions: usize,
) -> compile::Result<()> {
    if hir.from_literal && expansions == 0 {
        cx.q.diagnostics
            .template_without_expansions(cx.source_id, hir, cx.context())?;
    }

    Ok(())
}

/// Resolve the loop a `break` targets, along with where its value goes.
fn break_target<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::ExprBreak<'hir>,
    span: &'hir dyn Spanned,
) -> compile::Result<(Label, Option<Output>)> {
    match hir.label {
        Some(label) => {
            let l = cx.breaks.walk_until_label(span, label, &mut cx.drop)?;
            Ok((l.break_label.try_clone()?, l.output))
        }
        None => {
            let Some(l) = cx.breaks.last() else {
                return Err(compile::Error::new(span, ErrorKind::BreakUnsupported));
            };

            cx.drop.clear();
            cx.drop.try_extend(l.drop).with_span(span)?;
            Ok((l.break_label.try_clone()?, l.output))
        }
    }
}

/// Drop the loop temporaries a `break` leaves behind and jump out of it.
fn break_finish<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    span: &'hir dyn Spanned,
    break_label: Label,
) -> compile::Result<Asm<'hir>> {
    let mut drop_set = cx.q.unit.drop_set();

    // Drop loop temporaries.
    for addr in cx.drop.drain(..) {
        drop_set.push(addr)?;
    }

    if let Some(set) = drop_set.finish()? {
        cx.asm.push(inst::Kind::Drop { set }, span)?;
    }

    cx.asm.jump(&break_label, span)?;
    Ok(Asm::diverge(span))
}

/// An assignment evaluates to a unit if a value is needed from it.
fn assign_unit<'a, 'hir, 'o>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    span: &'hir dyn Spanned,
    needs: &mut (dyn Needs<'a, 'hir> + 'o),
) -> compile::Result<()>
where
    'a: 'o,
    'hir: 'o,
{
    if let Some(out) = needs.try_alloc_output()? {
        cx.asm.push(inst::Kind::unit(out), span)?;
    }

    Ok(())
}

/// The expressions a call assembles into its argument run, optionally led by
/// the target of an associated call.
fn call_ids(
    hir: &hir::ExprCall<'_>,
    target: Option<hir::ExprId>,
) -> compile::Result<Vec<hir::ExprId>> {
    let mut ids = Vec::try_with_capacity(hir.args.len() + usize::from(target.is_some()))?;

    if let Some(target) = target {
        ids.try_push(target)?;
    }

    for id in hir.args {
        ids.try_push(*id)?;
    }

    Ok(ids)
}

/// Whether assembling the given expression is known not to write to a
/// variable.
///
/// An operand which is a variable is assembled by handing the address of that
/// variable over rather than by writing anything, which is what keeps a common
/// shape like `a + b` from copying either of them. That only holds as long as
/// nothing assembled after it writes to that variable, since the operands are
/// read once every one of them has been assembled.
///
/// Only a literal and a variable are known here not to write, which is enough
/// to keep the shapes which are worth not copying from being copied. Anything
/// else is assumed to write, since finding out would mean walking the whole
/// expression - which is what the driver is here to avoid.
fn writes_nothing(cx: &Ctxt<'_, '_, '_>, id: hir::ExprId) -> bool {
    matches!(
        cx.exprs.get(id).kind,
        hir::ExprKind::Lit(..) | hir::ExprKind::Variable(..) | hir::ExprKind::Type(..)
    )
}

/// Start assembling a sequence of expressions into a linear run of addresses.
fn seq_start<'a, 'hir, 'o>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    span: &'hir dyn Spanned,
    items: &'hir [hir::ExprId],
    tail: SeqTail<'a, 'hir>,
    needs: &mut (dyn Needs<'a, 'hir> + 'o),
) -> compile::Result<Completed<'a, 'hir>>
where
    'a: 'o,
    'hir: 'o,
{
    let mut ids = Vec::try_with_capacity(items.len())?;

    for id in items {
        ids.try_push(*id)?;
    }

    seq_start_ids(cx, span, ids, tail, needs)
}

/// Start assembling the given expressions into a linear run of addresses.
fn seq_start_ids<'a, 'hir, 'o>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    span: &'hir dyn Spanned,
    ids: Vec<hir::ExprId>,
    tail: SeqTail<'a, 'hir>,
    needs: &mut (dyn Needs<'a, 'hir> + 'o),
) -> compile::Result<Completed<'a, 'hir>>
where
    'a: 'o,
    'hir: 'o,
{
    let Some(first) = ids.first().copied() else {
        let linear = Linear::empty();
        return seq_finish(cx, span, 0, linear, tail, needs);
    };

    let first = cx.exprs.get(first);

    let linear = if ids.len() == 1 {
        Linear::single(cx.scopes.alloc(first)?)
    } else {
        cx.scopes.linear(span, ids.len())?
    };

    Ok(Completed::Next(
        Step::Seq {
            span,
            ids,
            at: 0,
            linear,
            tail,
        },
        Current::Expr(first),
    ))
}

/// Emit the instruction a sequence of expressions was assembled for.
fn seq_finish<'a, 'hir, 'o>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    span: &'hir dyn Spanned,
    count: usize,
    linear: Linear<'a, 'hir>,
    tail: SeqTail<'a, 'hir>,
    needs: &mut (dyn Needs<'a, 'hir> + 'o),
) -> compile::Result<Completed<'a, 'hir>>
where
    'a: 'o,
    'hir: 'o,
{
    match tail {
        SeqTail::Vec => {
            if let Some(out) = needs.try_alloc_addr()? {
                cx.asm.push(
                    inst::Kind::Vec {
                        addr: linear.addr(),
                        count,
                        out: out.output(),
                    },
                    span,
                )?;

                linear.free_non_dangling()?;
            } else {
                linear.free()?;
            }
        }
        SeqTail::Tuple => {
            if count == 0 {
                cx.asm.push(inst::Kind::unit(needs.alloc_output()?), span)?;
                linear.free()?;
            } else if let Some(out) = needs.try_alloc_output()? {
                cx.asm.push(
                    inst::Kind::Tuple {
                        addr: linear.addr(),
                        count,
                        out,
                    },
                    span,
                )?;

                linear.free_non_dangling()?;
            } else {
                linear.free()?;
            }
        }
        SeqTail::CallVar { name, args } => {
            let var = cx.scopes.get(&mut cx.q, span, name)?;

            cx.asm.push(
                inst::Kind::CallFn {
                    function: var.addr,
                    addr: linear.addr(),
                    args,
                    out: needs.alloc_output()?,
                },
                span,
            )?;

            linear.free_non_dangling()?;
        }
        SeqTail::CallAssociated { hash, args } => {
            cx.asm.push(
                inst::Kind::CallAssociated {
                    hash,
                    addr: linear.addr(),
                    args,
                    out: needs.alloc_output()?,
                },
                span,
            )?;

            linear.free_non_dangling()?;
        }
        SeqTail::CallMeta { hash, args } => {
            cx.asm.push(
                inst::Kind::Call {
                    hash,
                    addr: linear.addr(),
                    args,
                    out: needs.alloc_output()?,
                },
                span,
            )?;

            linear.free_non_dangling()?;
        }
        SeqTail::CallExpr { function, args } => {
            cx.asm.push(
                inst::Kind::CallFn {
                    function: function.addr()?.addr(),
                    addr: linear.addr(),
                    args,
                    out: needs.alloc_output()?,
                },
                span,
            )?;

            linear.free_non_dangling()?;
            function.free()?;
        }
        SeqTail::Object { hir } => {
            match hir.kind {
                hir::ExprObjectKind::Struct { hash } => {
                    reorder_field_assignments(cx, hir, linear.addr(), span)?;

                    cx.asm.push(
                        inst::Kind::Struct {
                            addr: linear.addr(),
                            hash,
                            out: needs.alloc_output()?,
                        },
                        span,
                    )?;
                }
                hir::ExprObjectKind::ExternalType { hash, args } => {
                    reorder_field_assignments(cx, hir, linear.addr(), span)?;

                    cx.asm.push(
                        inst::Kind::Call {
                            hash,
                            addr: linear.addr(),
                            args,
                            out: needs.alloc_output()?,
                        },
                        span,
                    )?;
                }
                hir::ExprObjectKind::Anonymous => {
                    let slot = cx.q.unit.new_static_object_keys_iter(
                        span,
                        hir.assignments.iter().map(|a| a.key.1),
                    )?;

                    cx.asm.push(
                        inst::Kind::Object {
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
        SeqTail::Select { hir, end_label } => {
            return select_start(cx, hir, span, end_label, linear, needs);
        }
    }

    Ok(Completed::Done(Asm::new(span, ())))
}

/// The bounds a range assembles, in the order they are assembled.
fn range_operands(hir: &hir::ExprRange) -> (Option<hir::ExprId>, Option<hir::ExprId>) {
    match *hir {
        hir::ExprRange::RangeFrom { start } => (Some(start), None),
        hir::ExprRange::RangeFull => (None, None),
        hir::ExprRange::RangeInclusive { start, end } => (Some(start), Some(end)),
        hir::ExprRange::RangeToInclusive { end } => (Some(end), None),
        hir::ExprRange::RangeTo { end } => (Some(end), None),
        hir::ExprRange::Range { start, end } => (Some(start), Some(end)),
    }
}

/// Emit the select instruction and start assembling the branch bodies.
fn select_start<'a, 'hir, 'o>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ExprSelect<'hir>,
    span: &'hir dyn Spanned,
    end_label: Label,
    linear: Linear<'a, 'hir>,
    needs: &mut (dyn Needs<'a, 'hir> + 'o),
) -> compile::Result<Completed<'a, 'hir>>
where
    'a: 'o,
    'hir: 'o,
{
    let default_branch = match hir.default {
        Some(def) => Some((def, cx.asm.new_label("select_default"))),
        None => None,
    };

    let value_addr = cx.scopes.alloc(span)?;

    let select_label = cx.asm.new_label("select");
    cx.asm.label(&select_label)?;

    cx.asm.push(
        inst::Kind::Select {
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
                inst::Kind::Copy {
                    addr: value_addr.addr(),
                    out,
                },
                span,
            )?;
        }

        if !cx.select_branches.is_empty() {
            cx.asm.jump(&end_label, span)?;
        }
    }

    let mut branches = Vec::new();
    let mut taken = take(&mut cx.select_branches);

    for (label, branch) in taken.drain(..) {
        branches.try_push((label, branch))?;
    }

    cx.select_branches = taken;
    branches.reverse();

    select_next(
        cx,
        hir,
        span,
        end_label,
        select_label,
        value_addr,
        linear,
        branches,
        default_branch,
    )
}

/// Assemble the next branch body of a `select`, then its default.
#[allow(clippy::too_many_arguments)]
fn select_next<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ExprSelect<'hir>,
    span: &'hir dyn Spanned,
    end_label: Label,
    select_label: Label,
    mut value_addr: Address<'a, 'hir>,
    linear: Linear<'a, 'hir>,
    mut branches: Vec<(Label, &'hir hir::ExprSelectBranch<'hir>)>,
    default_branch: Option<(hir::ExprId, Label)>,
) -> compile::Result<Completed<'a, 'hir>> {
    while let Some((label, branch)) = branches.pop() {
        cx.asm.label(&label)?;

        let body = cx.exprs.get(branch.body);
        let scope = cx.scopes.child(body)?;

        if fn_arg_pat(cx, &branch.pat, &mut value_addr, &select_label)?.diverging() {
            cx.scopes.pop(body, scope)?;
            continue;
        }

        return Ok(Completed::Next(
            Step::Select(Box::try_new(StepSelect {
                hir,
                span,
                end_label,
                select_label,
                value_addr,
                linear,
                branches,
                default_branch,
                pending: Some(scope),
            })?),
            Current::Expr(body),
        ));
    }

    if let Some((branch, label)) = default_branch {
        cx.asm.label(&label)?;

        return Ok(Completed::Next(
            Step::Select(Box::try_new(StepSelect {
                hir,
                span,
                end_label,
                select_label,
                value_addr,
                linear,
                branches,
                default_branch: None,
                pending: None,
            })?),
            Current::Expr(cx.exprs.get(branch)),
        ));
    }

    Ok(Completed::Done(select_finish(
        cx, span, end_label, value_addr, linear,
    )?))
}

/// Close a `select` once every branch has been assembled.
fn select_finish<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    span: &'hir dyn Spanned,
    end_label: Label,
    value_addr: Address<'a, 'hir>,
    linear: Linear<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    cx.asm.label(&end_label)?;

    let mut drop_set = cx.q.unit.drop_set();

    // Drop futures we are currently using.
    for addr in &linear {
        drop_set.push(addr.addr())?;
    }

    if let Some(set) = drop_set.finish()? {
        cx.asm.push(inst::Kind::Drop { set }, span)?;
    }

    value_addr.free()?;
    linear.free()?;

    cx.contexts
        .pop()
        .ok_or("Missing parent context")
        .with_span(span)?;

    Ok(Asm::new(span, ()))
}

/// Assemble the patterns of a `match` once the value being matched is
/// available, suspending whenever an arm has a guard to assemble.
fn match_bodies<'a, 'hir, 'o>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ExprMatch<'hir>,
    span: &'hir dyn Spanned,
    value: Address<'a, 'hir>,
    needs: &mut (dyn Needs<'a, 'hir> + 'o),
) -> compile::Result<Completed<'a, 'hir>>
where
    'a: 'o,
    'hir: 'o,
{
    let end_label = cx.asm.new_label("match_end");

    let count = hir
        .branches
        .iter()
        .map(|b| b.pat.names.len())
        .max()
        .unwrap_or_default();

    let linear = cx.scopes.linear(span, count)?;

    match_pattern(
        cx,
        hir,
        span,
        value,
        end_label,
        linear,
        Vec::new(),
        0,
        false,
        needs,
    )
}

/// Assemble the pattern of the next arm of a `match`, or move on to its
/// bodies.
#[allow(clippy::too_many_arguments)]
fn match_pattern<'a, 'hir, 'o>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ExprMatch<'hir>,
    span: &'hir dyn Spanned,
    value: Address<'a, 'hir>,
    end_label: Label,
    mut linear: Linear<'a, 'hir>,
    mut branches: Vec<(Label, DanglingScope)>,
    mut at: usize,
    mut is_irrefutable: bool,
    needs: &mut (dyn Needs<'a, 'hir> + 'o),
) -> compile::Result<Completed<'a, 'hir>>
where
    'a: 'o,
    'hir: 'o,
{
    while !is_irrefutable {
        let Some(branch) = hir.branches.get(at) else {
            break;
        };

        at += 1;

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
            if let Some(condition) = branch.condition {
                let condition = cx.exprs.get(condition);
                let scope = cx.scopes.child(condition)?;
                let cond = cx.scopes.alloc_any(condition)?.with_name("match condition");

                return Ok(Completed::Next(
                    Step::MatchGuard(Box::try_new(StepMatchGuard {
                        hir,
                        span,
                        value,
                        end_label,
                        linear,
                        branches,
                        at,
                        branch_label,
                        match_false,
                        pattern_scope,
                        scope,
                        pat,
                        cond,
                    })?),
                    Current::Expr(condition),
                ));
            }

            // If there is no branch condition, and the branch is irrefutable,
            // there is no point in assembling the additional branches.
            is_irrefutable = matches!(pat, Pattern::Irrefutable);

            cx.asm.jump(&branch_label, span)?;
            let pattern_scope = cx.scopes.dangle(span, pattern_scope)?;
            branches.try_push((branch_label, pattern_scope))?;
        }

        if is_irrefutable {
            break;
        }

        cx.asm.label(&match_false)?;
    }

    match_finish(
        cx,
        hir,
        span,
        value,
        end_label,
        linear,
        branches,
        is_irrefutable,
        needs,
    )
}

/// Move on to the bodies of a `match` once every pattern has been assembled.
#[allow(clippy::too_many_arguments)]
fn match_finish<'a, 'hir, 'o>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ExprMatch<'hir>,
    span: &'hir dyn Spanned,
    value: Address<'a, 'hir>,
    end_label: Label,
    linear: Linear<'a, 'hir>,
    branches: Vec<(Label, DanglingScope)>,
    is_irrefutable: bool,
    needs: &mut (dyn Needs<'a, 'hir> + 'o),
) -> compile::Result<Completed<'a, 'hir>>
where
    'a: 'o,
    'hir: 'o,
{
    if !is_irrefutable {
        if let Some(out) = needs.try_alloc_output()? {
            cx.asm.push(inst::Kind::unit(out), span)?;
        }

        cx.asm.jump(&end_label, span)?;
    } else if branches.len() > 1 {
        // Every arm writes where the match writes, so that has to be an address
        // of its own once there is more than one of them.
        //
        // An arm whose body is a variable hands the address of that variable
        // over rather than writing anything, so leaving it to the arms would
        // have the arms which follow write into whichever variable the first
        // one named.
        needs.try_alloc_output()?;
    }

    let mut bodies = Vec::new();

    for (branch, (label, scope)) in hir.branches.iter().zip(branches) {
        bodies.try_push((branch, label, scope))?;
    }

    bodies.reverse();

    match_next(cx, span, end_label, value, linear, bodies, is_irrefutable)
}

/// Assemble the next arm body of a `match`, or finish it.
fn match_next<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    span: &'hir dyn Spanned,
    end_label: Label,
    value: Address<'a, 'hir>,
    linear: Linear<'a, 'hir>,
    mut bodies: Vec<(&'hir hir::ExprMatchBranch<'hir>, Label, DanglingScope)>,
    all_diverge: bool,
) -> compile::Result<Completed<'a, 'hir>> {
    if let Some((branch, label, scope)) = bodies.pop() {
        cx.asm.label(&label)?;
        let scope = cx.scopes.restore(scope);

        return Ok(Completed::Next(
            Step::Match {
                span,
                end_label,
                value,
                linear,
                bodies,
                all_diverge,
                pending: Some((branch, scope)),
            },
            Current::Expr(cx.exprs.get(branch.body)),
        ));
    }

    cx.asm.label(&end_label)?;

    value.free()?;
    linear.free()?;

    if all_diverge {
        return Ok(Completed::Done(Asm::diverge(span)));
    }

    Ok(Completed::Done(Asm::new(span, ())))
}

/// Assemble the prologue of a `for` loop and queue its body.
fn for_body<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::ExprFor<'hir>,
    span: &'hir dyn Spanned,
    iter: Any<'a, 'hir>,
) -> compile::Result<Completed<'a, 'hir>> {
    let hir_iter = cx.exprs.get(hir.iter);

    let continue_label = cx.asm.new_label("for_continue");
    let end_label = cx.asm.new_label("for_end");
    let break_label = cx.asm.new_label("for_break");

    // Variables.
    let iter = iter.into_addr()?;
    let into_iter = cx.scopes.alloc(span)?.with_name("into_iter");
    let binding = cx.scopes.alloc(&hir.binding)?.with_name("binding");

    // Copy the iterator, since CallAssociated will consume it.
    cx.asm.push_with_comment(
        inst::Kind::Copy {
            addr: iter.addr(),
            out: into_iter.output(),
        },
        span,
        &"Protocol::INTO_ITER",
    )?;

    cx.asm.push_with_comment(
        inst::Kind::CallAssociated {
            addr: into_iter.addr(),
            hash: Protocol::INTO_ITER.hash,
            args: 1,
            out: into_iter.output(),
        },
        hir_iter,
        &"Protocol::INTO_ITER",
    )?;

    // Declare storage for memoized `next` instance fn.
    let next_offset = if cx.options.memoize_instance_fn {
        let offset = cx.scopes.alloc(hir_iter)?.with_name("memoized next");

        cx.asm.push_with_comment(
            inst::Kind::LoadInstanceFn {
                addr: into_iter.addr(),
                hash: Protocol::NEXT.hash,
                out: offset.output(),
            },
            hir_iter,
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

    let into_iter_copy = cx.scopes.alloc(span)?.with_name("into_iter_copy");

    cx.asm.push(
        inst::Kind::Copy {
            addr: into_iter.addr(),
            out: into_iter_copy.output(),
        },
        span,
    )?;

    // Use the memoized loop variable.
    if let Some(next_offset) = &next_offset {
        cx.asm.push(
            inst::Kind::CallFn {
                function: next_offset.addr(),
                addr: into_iter_copy.addr(),
                args: 1,
                out: binding.output(),
            },
            span,
        )?;
    } else {
        cx.asm.push_with_comment(
            inst::Kind::CallAssociated {
                addr: into_iter_copy.addr(),
                hash: Protocol::NEXT.hash,
                args: 1,
                out: binding.output(),
            },
            span,
            &"Protocol::NEXT",
        )?;
    }

    into_iter_copy.free()?;

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

    Ok(Completed::Next(
        Step::For(Box::try_new(StepFor {
            span,
            continue_label,
            end_label,
            break_label,
            inner_loop_scope,
            bindings,
            binding,
            into_iter,
            iter,
            next_offset,
            body: Any::ignore(span),
        })?),
        Current::Block(&hir.body),
    ))
}

/// Assemble the condition of the next branch of an `if`, or move on to its
/// bodies once every condition has been assembled.
#[allow(clippy::too_many_arguments)]
fn if_condition<'a, 'hir, 'o>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::Conditional<'hir>,
    span: &'hir dyn Spanned,
    end_label: Label,
    linear: Linear<'a, 'hir>,
    mut branches: Vec<(&'hir hir::ConditionalBranch<'hir>, Label, DanglingScope)>,
    at: usize,
    needs: &mut (dyn Needs<'a, 'hir> + 'o),
) -> compile::Result<Completed<'a, 'hir>>
where
    'a: 'o,
    'hir: 'o,
{
    if let Some(branch) = hir.branches.get(at) {
        let then_label = cx.asm.new_label("if_branch");
        let false_label = cx.asm.new_label("if_false");

        let (scope, value, child) = match *branch.condition {
            hir::Condition::Expr(_, e) => {
                let e = cx.exprs.get(e);
                let scope = cx.scopes.child(e)?;
                let value = cx.scopes.alloc_any(e)?.with_name("expression condition");
                (scope, value, e)
            }
            hir::Condition::ExprLet(let_hir) => {
                let scope = cx.scopes.child(let_hir)?;
                let e = cx.exprs.get(let_hir.expr);
                (scope, cx.scopes.defer(let_hir), e)
            }
        };

        return Ok(Completed::Next(
            Step::IfCondition {
                hir,
                span,
                end_label,
                linear,
                branches,
                at,
                then_label,
                false_label,
                scope,
                value,
            },
            Current::Expr(child),
        ));
    }

    branches.reverse();

    // The fallback is assembled as fall through, so it goes first and the
    // branches follow it.
    if let Some(b) = hir.fallback {
        return Ok(Completed::Next(
            Step::If {
                hir,
                span,
                end_label,
                linear,
                branches,
                output_addr: None,
                all_diverging: false,
                pending: None,
                ignore: None,
            },
            Current::Block(b),
        ));
    }

    let (all_diverging, output_addr) = if branches.is_empty() {
        (true, None)
    } else {
        let output_addr = needs.try_alloc_output()?;

        if let Some(out) = output_addr {
            cx.asm.push(inst::Kind::unit(out), span)?;
        }

        (false, output_addr)
    };

    // The fall through path must not enter the first branch body, which is
    // emitted immediately after this.
    //
    // TODO: Is there a way to avoid emitting this jump if all branches
    // diverges?
    cx.asm.jump(&end_label, span)?;

    if_next(
        cx,
        hir,
        span,
        end_label,
        linear,
        branches,
        output_addr,
        all_diverging,
    )
}

/// Assemble the next branch of an `if`, or finish it.
#[allow(clippy::too_many_arguments)]
fn if_next<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::Conditional<'hir>,
    span: &'hir dyn Spanned,
    end_label: Label,
    linear: Linear<'a, 'hir>,
    mut branches: Vec<(&'hir hir::ConditionalBranch<'hir>, Label, DanglingScope)>,
    output_addr: Option<Output>,
    all_diverging: bool,
) -> compile::Result<Completed<'a, 'hir>> {
    if let Some((branch, label, scope)) = branches.pop() {
        cx.asm.label(&label)?;
        let scope = cx.scopes.restore(scope);

        let ignore = if hir.fallback.is_some() {
            None
        } else {
            Some(Any::ignore(branch))
        };

        return Ok(Completed::Next(
            Step::If {
                hir,
                span,
                end_label,
                linear,
                branches,
                output_addr,
                all_diverging,
                pending: Some((branch, scope)),
                ignore,
            },
            Current::Block(&branch.block),
        ));
    }

    linear.free()?;
    cx.asm.label(&end_label)?;

    if all_diverging {
        return Ok(Completed::Done(Asm::diverge(span)));
    }

    Ok(Completed::Done(Asm::new(span, ())))
}

/// Start assembling a block, pushing the step which finishes it.
fn block_start<'a, 'hir, 'o>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::Block<'hir>,
    needs: &mut (dyn Needs<'a, 'hir> + 'o),
) -> compile::Result<Completed<'a, 'hir>>
where
    'a: 'o,
    'hir: 'o,
{
    let break_label = if let Some(label) = hir.label {
        let break_label = cx.asm.new_label("block_break");
        let output = Some(needs.alloc_output()?);

        cx.breaks.push(Break {
            label: Some(label),
            continue_label: None,
            break_label: break_label.try_clone()?,
            output,
            drop: None,
        })?;

        Some(break_label)
    } else {
        None
    };

    // A block which produces a value has to produce it into an address which
    // outlives its own scope. Allocating the need up front is what guarantees
    // that: a deferred need is *assigned* the address of whatever produced it,
    // so a value which is a local of this block would be aliased rather than
    // copied, and the address is then cleared by the drop which closing the
    // scope emits - leaving the caller reading an empty slot.
    if hir.value.is_some() {
        needs.try_alloc_addr()?;
    }

    let scope = cx.scopes.child(hir)?;
    cx.contexts.try_push(hir)?;
    block_next(cx, hir, scope, break_label, 0, None, needs)
}

/// Assemble the next statement of a block, or its value, or finish it.
#[allow(clippy::too_many_arguments)]
fn block_next<'a, 'hir, 'o>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::Block<'hir>,
    scope: ScopeHandle,
    break_label: Option<Label>,
    mut at: usize,
    diverge: Option<&'hir dyn Spanned>,
    needs: &mut (dyn Needs<'a, 'hir> + 'o),
) -> compile::Result<Completed<'a, 'hir>>
where
    'a: 'o,
    'hir: 'o,
{
    while let Some(stmt) = hir.statements.get(at) {
        at += 1;

        if let Some(cause) = diverge {
            cx.q.diagnostics.unreachable(cx.source_id, stmt, cause)?;
            continue;
        }

        match stmt {
            hir::Stmt::Local(local_hir) => {
                // The initialiser is assembled into a deferred address which
                // the pattern is then bound against, so that it goes over the
                // work stack rather than through a callback.
                let value = cx.scopes.defer(&local_hir.pat);

                return Ok(Completed::Next(
                    Step::Block {
                        hir,
                        scope,
                        break_label,
                        at,
                        diverge,
                        stmt: Some(stmt),
                        needs: Some(value),
                    },
                    Current::Expr(cx.exprs.get(local_hir.expr)),
                ));
            }
            hir::Stmt::Expr(_, e) => {
                let stmt_needs = Any::ignore(hir).with_name("statement ignore");

                return Ok(Completed::Next(
                    Step::Block {
                        hir,
                        scope,
                        break_label,
                        at,
                        diverge,
                        stmt: Some(stmt),
                        needs: Some(stmt_needs),
                    },
                    Current::Expr(cx.exprs.get(*e)),
                ));
            }
        }
    }

    if let Some(cause) = diverge {
        if let Some(e) = hir.value {
            let e = cx.exprs.get(e);
            cx.q.diagnostics.unreachable(cx.source_id, e, cause)?;
        }
    } else if let Some(e) = hir.value {
        return Ok(Completed::Next(
            Step::Block {
                hir,
                scope,
                break_label,
                at,
                diverge,
                stmt: None,
                needs: None,
            },
            Current::Expr(cx.exprs.get(e)),
        ));
    } else if let Some(out) = needs.try_alloc_output()? {
        cx.asm.push(inst::Kind::unit(out), hir)?;
    }

    Ok(Completed::Done(block_finish(
        cx,
        hir,
        scope,
        break_label,
        diverge,
    )?))
}

/// Close the scope and label of a block which has been fully assembled.
fn block_finish<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::Block<'hir>,
    scope: ScopeHandle,
    break_label: Option<Label>,
    diverge: Option<&'hir dyn Spanned>,
) -> compile::Result<Asm<'hir>> {
    cx.contexts
        .pop()
        .ok_or("Missing parent context")
        .with_span(hir)?;

    cx.scopes.pop(hir, scope)?;
    cx.drop_dangling(hir)?;

    if let Some(break_label) = break_label {
        cx.asm.label(&break_label)?;
        cx.breaks.pop();

        // A block which can be broken out of converges at its label whatever
        // its body did, since `break` is a jump to here. Reporting it as
        // diverging left the label at the end of what was assembled, so
        // breaking out of it ran off the end of the instructions.
        return Ok(Asm::new(hir, ()));
    }

    if diverge.is_some() {
        return Ok(Asm::diverge(hir));
    }

    Ok(Asm::new(hir, ()))
}

/// Complete a pending step now that the child it was waiting for has been
/// assembled with the outcome in `child`.
fn complete<'a, 'hir, 'o>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    step: Step<'a, 'hir>,
    child: Asm<'hir>,
    needs: &mut (dyn Needs<'a, 'hir> + 'o),
) -> compile::Result<Completed<'a, 'hir>>
where
    'a: 'o,
    'hir: 'o,
{
    let diverging = child.diverging();

    let asm = match step {
        Step::Block {
            hir,
            scope,
            break_label,
            at,
            mut diverge,
            stmt,
            needs: stmt_needs,
        } => {
            if let Some(stmt_needs) = stmt_needs {
                match stmt {
                    Some(hir::Stmt::Local(local_hir)) if !diverging => {
                        let addr = stmt_needs.into_addr()?;

                        let mut load =
                            |cx: &mut Ctxt<'a, 'hir, '_>, needs: &mut dyn Needs<'a, 'hir>| {
                                needs.assign_addr(cx, addr.addr())?;
                                Ok(Asm::new(&local_hir.pat, ()))
                            };

                        let asm = pattern_panic(cx, &local_hir.pat, |cx, false_label| {
                            pat_binding(cx, &local_hir.pat, false_label, &mut load)
                        })?;

                        addr.free()?;

                        if asm.diverging() && diverge.is_none() {
                            diverge = Some(local_hir);
                        }
                    }
                    _ => {
                        stmt_needs.free()?;
                    }
                }
            }

            if diverging && diverge.is_none() {
                diverge = match stmt {
                    Some(stmt) => Some(stmt),
                    None => Some(hir),
                };
            }

            if stmt.is_some() {
                return block_next(cx, hir, scope, break_label, at, diverge, needs);
            }

            return Ok(Completed::Done(block_finish(
                cx,
                hir,
                scope,
                break_label,
                diverge,
            )?));
        }
        Step::LoopCondition {
            hir,
            span,
            continue_label,
            then_label,
            end_label,
            break_label,
            mut linear,
            scope,
            value,
        } => {
            let condition_hir = match hir.condition {
                Some(condition_hir) => condition_hir,
                None => {
                    value.free()?;
                    linear.free()?;
                    return Err(compile::Error::msg(span, "Loop without a condition"));
                }
            };

            let mut popped = false;

            let converging = if diverging {
                value.free()?;
                popped = true;
                false
            } else {
                match *condition_hir {
                    hir::Condition::Expr(_, _) => {
                        let addr = value.into_addr()?;
                        cx.asm.jump_if(addr.addr(), &then_label, span)?;
                        addr.free()?;
                        true
                    }
                    hir::Condition::ExprLet(let_hir) => {
                        let addr = value.into_addr()?;

                        let mut load =
                            |cx: &mut Ctxt<'a, 'hir, '_>, needs: &mut dyn Needs<'a, 'hir>| {
                                needs.assign_addr(cx, addr.addr())?;
                                Ok(Asm::new(&let_hir.pat, ()))
                            };

                        let asm = pat_binding_with(
                            cx,
                            &let_hir.pat,
                            &let_hir.pat.pat,
                            let_hir.pat.names,
                            &end_label,
                            &mut load,
                            &mut linear,
                        )?;

                        addr.free()?;

                        if asm.into_converging().is_some() {
                            cx.asm.jump(&then_label, span)?;
                            true
                        } else {
                            popped = true;
                            false
                        }
                    }
                }
            };

            let condition_scope = if converging {
                cx.asm.jump(&end_label, span)?;
                cx.asm.label(&then_label)?;
                Some(scope)
            } else {
                if popped {
                    cx.scopes.pop(span, scope)?;
                }

                None
            };

            return Ok(Completed::Next(
                Step::Loop {
                    span,
                    continue_label,
                    end_label,
                    break_label,
                    condition_scope,
                    linear,
                    body: Any::ignore(span),
                },
                Current::Block(&hir.body),
            ));
        }
        Step::Loop {
            span,
            continue_label,
            end_label,
            break_label,
            condition_scope,
            linear,
            body,
        } => {
            // Divergence is ignored, since there are labels which might jump
            // over it.
            body.free()?;

            if let Some(scope) = condition_scope {
                cx.scopes.pop(span, scope)?;
            }

            cx.asm.jump(&continue_label, span)?;
            cx.asm.label(&end_label)?;

            if let Some(out) = needs.try_alloc_output()? {
                cx.asm.push(inst::Kind::unit(out), span)?;
            }

            cx.asm.label(&break_label)?;

            linear.free()?;
            cx.breaks.pop();
            Asm::new(span, ())
        }
        Step::IfCondition {
            hir,
            span,
            end_label,
            mut linear,
            mut branches,
            at,
            then_label,
            false_label,
            scope,
            value,
        } => {
            let branch = &hir.branches[at];

            if diverging {
                value.free()?;
                cx.scopes.pop(branch, scope)?;
            } else {
                let addr = value.into_addr()?;

                let pat = match *branch.condition {
                    hir::Condition::Expr(..) => {
                        cx.asm.jump_if(addr.addr(), &then_label, branch)?;
                        addr.free()?;
                        Some(Pattern::Irrefutable)
                    }
                    hir::Condition::ExprLet(let_hir) => {
                        let mut load =
                            |cx: &mut Ctxt<'a, 'hir, '_>, needs: &mut dyn Needs<'a, 'hir>| {
                                needs.assign_addr(cx, addr.addr())?;
                                Ok(Asm::new(&let_hir.pat, ()))
                            };

                        let asm = pat_binding_with(
                            cx,
                            &let_hir.pat,
                            &let_hir.pat.pat,
                            let_hir.pat.names,
                            &false_label,
                            &mut load,
                            &mut linear,
                        )?;

                        addr.free()?;

                        match asm.into_converging() {
                            Some(pat) => {
                                cx.asm.jump(&then_label, branch)?;
                                Some(pat)
                            }
                            None => None,
                        }
                    }
                };

                match pat {
                    Some(pat) => {
                        if matches!(pat, Pattern::Refutable) {
                            cx.asm.label(&false_label)?;
                        }

                        let scope = cx.scopes.dangle(branch, scope)?;
                        branches.try_push((branch, then_label, scope))?;
                    }
                    None => {
                        cx.scopes.pop(branch, scope)?;
                    }
                }
            }

            return if_condition(cx, hir, span, end_label, linear, branches, at + 1, needs);
        }
        Step::If {
            hir,
            span,
            end_label,
            linear,
            branches,
            output_addr,
            mut all_diverging,
            pending,
            ignore,
        } => {
            if let Some(ignore) = ignore {
                ignore.free()?;
            }

            let output_addr = match pending {
                // A branch body was assembled.
                Some((branch, scope)) => {
                    if !diverging {
                        all_diverging = false;

                        if hir.fallback.is_none() {
                            if let Some(out) = output_addr {
                                cx.asm.push(inst::Kind::unit(out), span)?;
                            }
                        }

                        if !branches.is_empty() {
                            cx.asm.jump(&end_label, branch)?;
                        }
                    }

                    cx.scopes.pop(branch, scope)?;
                    output_addr
                }
                // The fallback was assembled.
                None => {
                    all_diverging = diverging;

                    // TODO: Is there a way to avoid emitting this jump if all
                    // branches diverges?
                    cx.asm.jump(&end_label, span)?;
                    output_addr
                }
            };

            return if_next(
                cx,
                hir,
                span,
                end_label,
                linear,
                branches,
                output_addr,
                all_diverging,
            );
        }
        Step::MatchValue { hir, span, value } => {
            if diverging {
                value.free()?;
                return Ok(Completed::Done(Asm::diverge(span)));
            }

            let value = value.into_addr()?;
            return match_bodies(cx, hir, span, value, needs);
        }
        Step::Select(s) => {
            let StepSelect {
                hir,
                span,
                end_label,
                select_label,
                value_addr,
                linear,
                branches,
                default_branch,
                pending,
            } = Box::into_inner(s);

            if let Some(scope) = pending {
                if !diverging {
                    cx.asm.jump(&end_label, span)?;
                }

                cx.scopes.pop(span, scope)?;

                return select_next(
                    cx,
                    hir,
                    span,
                    end_label,
                    select_label,
                    value_addr,
                    linear,
                    branches,
                    default_branch,
                );
            }

            // The default body was assembled; nothing more remains.
            return Ok(Completed::Done(select_finish(
                cx, span, end_label, value_addr, linear,
            )?));
        }
        Step::MatchGuard(s) => {
            let StepMatchGuard {
                hir,
                span,
                value,
                end_label,
                linear,
                mut branches,
                at,
                branch_label,
                match_false,
                pattern_scope,
                scope,
                pat,
                cond,
            } = Box::into_inner(s);

            let mut is_irrefutable = false;

            if diverging {
                cond.free()?;
                cx.scopes.pop(span, scope)?;

                // If the branch condition diverges, there is no reason to
                // assemble the other branches if this one is irrefutable.
                is_irrefutable = matches!(pat, Pattern::Irrefutable);
                cx.scopes.pop(span, pattern_scope)?;
            } else {
                let cond = cond.into_addr()?;
                cx.asm.jump_if_not(cond.addr(), &match_false, span)?;
                cx.asm.jump(&branch_label, span)?;
                cond.free()?;
                cx.scopes.pop(span, scope)?;

                cx.asm.jump(&branch_label, span)?;
                let pattern_scope = cx.scopes.dangle(span, pattern_scope)?;
                branches.try_push((branch_label, pattern_scope))?;
            }

            if !is_irrefutable {
                cx.asm.label(&match_false)?;
            }

            return match_pattern(
                cx,
                hir,
                span,
                value,
                end_label,
                linear,
                branches,
                at,
                is_irrefutable,
                needs,
            );
        }
        Step::Match {
            span,
            end_label,
            value,
            linear,
            bodies,
            mut all_diverge,
            pending,
        } => {
            if let Some((branch, scope)) = pending {
                if !diverging {
                    all_diverge = false;

                    if !bodies.is_empty() {
                        cx.asm.jump(&end_label, branch)?;
                    }
                }

                cx.scopes.pop(branch, scope)?;
            }

            return match_next(cx, span, end_label, value, linear, bodies, all_diverge);
        }
        Step::AssignBinop {
            hir,
            span,
            inst_target,
            writeback,
            pending,
        } => {
            if diverging {
                pending.free()?;

                if let Some((_, target)) = writeback {
                    target.free()?;
                }

                return Ok(Completed::Done(Asm::diverge(span)));
            }

            let Some(inst_target) = inst_target else {
                // The target of a field assignment was assembled; the value
                // follows it.
                let lhs = cx.exprs.get(hir.lhs);

                let hir::ExprKind::FieldAccess(field_access) = lhs.kind else {
                    pending.free()?;
                    return Err(compile::Error::new(span, ErrorKind::UnsupportedBinaryExpr));
                };

                let target = pending.into_addr()?;

                let inst_target = match field_access.expr_field {
                    hir::ExprField::Index(index) => InstTarget::TupleField(target.addr(), index),
                    hir::ExprField::Ident(ident) => {
                        let access_expr = cx.exprs.get(field_access.expr);
                        let slot = cx.q.unit.new_static_string(access_expr, ident)?;
                        InstTarget::Field(target.addr(), slot)
                    }
                    _ => {
                        target.free()?;
                        return Err(compile::Error::new(span, ErrorKind::BadFieldAccess));
                    }
                };

                target.free()?;

                return Ok(Completed::Next(
                    Step::AssignBinop {
                        hir,
                        span,
                        inst_target: Some(inst_target),
                        writeback: None,
                        pending: cx.scopes.defer(cx.exprs.get(hir.rhs)),
                    },
                    Current::Expr(cx.exprs.get(hir.rhs)),
                ));
            };

            let value = pending.into_addr()?;
            let inst = assign_binop_inst(&hir.op, inst_target, value.addr(), span)?;
            cx.asm.push(inst, span)?;

            if let Some((slot, target)) = writeback {
                cx.asm.push(
                    inst::Kind::GlobalSet {
                        slot,
                        value: target.addr(),
                    },
                    span,
                )?;

                target.free()?;
            }

            if let Some(out) = needs.try_alloc_output()? {
                cx.asm.push(inst::Kind::unit(out), span)?;
            }

            value.free()?;
            Asm::new(span, ())
        }
        Step::Break {
            span,
            break_label,
            needs: value,
        } => {
            if diverging {
                value.free()?;
                return Ok(Completed::Done(Asm::diverge(span)));
            }

            value.free()?;
            break_finish(cx, span, break_label)?
        }
        Step::Return { span, addr } => {
            if diverging {
                addr.free()?;
                return Ok(Completed::Done(Asm::diverge(span)));
            }

            cx.asm.push(
                inst::Kind::Return {
                    addr: addr.addr()?.addr(),
                },
                span,
            )?;

            addr.free()?;
            Asm::diverge(span)
        }
        Step::Yield { span, addr, out } => {
            if diverging {
                addr.free()?;
                return Ok(Completed::Done(Asm::diverge(span)));
            }

            cx.asm.push(
                inst::Kind::Yield {
                    addr: addr.addr(),
                    out,
                },
                span,
            )?;

            addr.free()?;
            Asm::new(span, ())
        }
        Step::Range {
            hir,
            span,
            first,
            pending,
        } => {
            if diverging {
                pending.free()?;

                if let Some(first) = first {
                    first.free()?;
                }

                return Ok(Completed::Done(Asm::diverge(span)));
            }

            let addr = pending.into_addr()?;

            if first.is_none() {
                if let (_, Some(b)) = range_operands(hir) {
                    let b = cx.exprs.get(b);

                    return Ok(Completed::Next(
                        Step::Range {
                            hir,
                            span,
                            first: Some(addr),
                            pending: cx.scopes.defer(b),
                        },
                        Current::Expr(b),
                    ));
                }
            }

            let (a, b) = match first {
                Some(first) => (first, Some(addr)),
                None => (addr, None),
            };

            let range = match *hir {
                hir::ExprRange::RangeFrom { .. } => InstRange::RangeFrom { start: a.addr() },
                hir::ExprRange::RangeFull => InstRange::RangeFull,
                hir::ExprRange::RangeInclusive { .. } => InstRange::RangeInclusive {
                    start: a.addr(),
                    end: b.as_ref().map(Address::addr).unwrap_or(a.addr()),
                },
                hir::ExprRange::RangeToInclusive { .. } => {
                    InstRange::RangeToInclusive { end: a.addr() }
                }
                hir::ExprRange::RangeTo { .. } => InstRange::RangeTo { end: a.addr() },
                hir::ExprRange::Range { .. } => InstRange::Range {
                    start: a.addr(),
                    end: b.as_ref().map(Address::addr).unwrap_or(a.addr()),
                },
            };

            if let Some(out) = needs.try_alloc_output()? {
                cx.asm.push(inst::Kind::Range { range, out }, span)?;
            }

            if let Some(b) = b {
                b.free()?;
            }

            a.free()?;
            Asm::new(span, ())
        }
        Step::CallFunction {
            hir,
            span,
            function,
        } => {
            if diverging {
                function.free()?;
                return Ok(Completed::Done(Asm::diverge(span)));
            }

            let args = hir.args.len();
            let ids = call_ids(hir, None)?;

            return seq_start_ids(cx, span, ids, SeqTail::CallExpr { function, args }, needs);
        }
        Step::AssignVar { span, addr } => {
            if diverging {
                addr.free()?;
                return Ok(Completed::Done(Asm::diverge(span)));
            }

            addr.free()?;
            assign_unit(cx, span, needs)?;
            Asm::new(span, ())
        }
        Step::AssignStatic { span, slot, value } => {
            if diverging {
                value.free()?;
                return Ok(Completed::Done(Asm::diverge(span)));
            }

            let value = value.into_addr()?;

            cx.asm.push(
                inst::Kind::GlobalSet {
                    slot,
                    value: value.addr(),
                },
                span,
            )?;

            value.free()?;
            assign_unit(cx, span, needs)?;
            Asm::new(span, ())
        }
        Step::Defers {
            span,
            ids,
            at,
            mut addrs,
            tail,
        } => {
            if diverging {
                for addr in addrs.into_iter().rev() {
                    addr.free()?;
                }

                return Ok(Completed::Done(Asm::diverge(span)));
            }

            let at = at + 1;

            if let Some(id) = ids.get(at).copied() {
                let child = cx.exprs.get(id);

                return Ok(Completed::Next(
                    Step::Defers {
                        span,
                        ids,
                        at,
                        addrs,
                        tail,
                    },
                    Current::Expr(child),
                ));
            }

            let mut resolved = Vec::try_with_capacity(addrs.len())?;

            for addr in &mut addrs {
                resolved.try_push(addr.addr()?.addr())?;
            }

            let kind = match tail {
                DefersTail::AssignField { hir } => match hir.expr_field {
                    hir::ExprField::Ident(ident) => {
                        let slot = cx.q.unit.new_static_string(span, ident)?;

                        inst::Kind::ObjectIndexSet {
                            target: resolved[0],
                            slot,
                            value: resolved[1],
                        }
                    }
                    hir::ExprField::Index(index) => inst::Kind::TupleIndexSet {
                        target: resolved[0],
                        index,
                        value: resolved[1],
                    },
                    _ => {
                        for addr in addrs.into_iter().rev() {
                            addr.free()?;
                        }

                        return Err(compile::Error::new(span, ErrorKind::BadFieldAccess));
                    }
                },
                DefersTail::AssignIndex => inst::Kind::IndexSet {
                    target: resolved[0],
                    index: resolved[1],
                    value: resolved[2],
                },
                DefersTail::Tuple => {
                    let out = needs.alloc_output()?;

                    match resolved.as_slice() {
                        [a] => inst::Kind::Tuple1 { addr: [*a], out },
                        [a, b] => inst::Kind::Tuple2 {
                            addr: [*a, *b],
                            out,
                        },
                        [a, b, c] => inst::Kind::Tuple3 {
                            addr: [*a, *b, *c],
                            out,
                        },
                        [a, b, c, d] => inst::Kind::Tuple4 {
                            addr: [*a, *b, *c, *d],
                            out,
                        },
                        _ => {
                            for addr in addrs.into_iter().rev() {
                                addr.free()?;
                            }

                            return Err(compile::Error::msg(span, "Unsupported tuple size"));
                        }
                    }
                }
            };

            cx.asm.push(kind, span)?;

            let assignment = !matches!(tail, DefersTail::Tuple);

            for addr in addrs.into_iter().rev() {
                addr.free()?;
            }

            if assignment {
                assign_unit(cx, span, needs)?;
            }

            Asm::new(span, ())
        }
        Step::Template {
            hir,
            at,
            linear,
            size_hint,
            expansions,
        } => {
            if diverging {
                template_diagnostics(cx, hir, expansions)?;
                linear.free()?;
                return Ok(Completed::Done(Asm::diverge(hir)));
            }

            return template_next(cx, hir, at + 1, linear, size_hint, expansions, needs);
        }
        Step::Format { format } => {
            if diverging {
                return Ok(Completed::Done(Asm::diverge(format)));
            }

            use crate::runtime::format;

            let fill = format.spec.fill.unwrap_or(' ');
            let align = format.spec.align.unwrap_or_default();
            let flags = format.spec.flags.unwrap_or_default();
            let width = format.spec.width;
            let precision = format.spec.precision;
            let format_type = format.spec.format_type.unwrap_or_default();

            let spec = format::FormatSpec::new(flags, fill, align, width, precision, format_type);

            if let Some(addr) = needs.try_alloc_addr()? {
                cx.asm.push(
                    inst::Kind::Format {
                        addr: addr.addr(),
                        spec,
                        out: addr.output(),
                    },
                    format,
                )?;
            }

            Asm::new(format, ())
        }
        Step::Seq {
            span,
            ids,
            at,
            linear,
            tail,
        } => {
            if diverging {
                linear.free()?;
                return Ok(Completed::Done(Asm::diverge(span)));
            }

            let at = at + 1;

            if let Some(id) = ids.get(at).copied() {
                let child = cx.exprs.get(id);

                return Ok(Completed::Next(
                    Step::Seq {
                        span,
                        ids,
                        at,
                        linear,
                        tail,
                    },
                    Current::Expr(child),
                ));
            }

            return seq_finish(cx, span, ids.len(), linear, tail, needs);
        }
        Step::ForIter { hir, span, iter } => {
            let hir_iter = cx.exprs.get(hir.iter);

            if diverging {
                iter.free()?;
                cx.q.diagnostics
                    .unreachable(cx.source_id, &hir.body, hir_iter)?;
                return Ok(Completed::Done(Asm::diverge(span)));
            }

            return for_body(cx, hir, span, iter);
        }
        Step::For(s) => {
            let StepFor {
                span,
                continue_label,
                end_label,
                break_label,
                inner_loop_scope,
                bindings,
                binding,
                into_iter,
                iter,
                next_offset,
                body,
            } = Box::into_inner(s);

            body.free()?;
            bindings.free()?;
            cx.scopes.pop(span, inner_loop_scope)?;

            if !diverging {
                cx.asm.jump(&continue_label, span)?;
            }

            cx.asm.label(&end_label)?;

            let mut drop_set = cx.q.unit.drop_set();
            drop_set.push(into_iter.addr())?;

            // NB: Dropping has to happen before the break label. When breaking,
            // the break statement is responsible for ensuring that active
            // iterators are dropped.
            if let Some(set) = drop_set.finish()? {
                cx.asm.push(inst::Kind::Drop { set }, span)?;
            }

            cx.asm.label(&break_label)?;

            if let Some(out) = needs.try_alloc_output()? {
                cx.asm.push(inst::Kind::unit(out), span)?;
            }

            if let Some(next_offset) = next_offset {
                next_offset.free()?;
            }

            binding.free()?;
            into_iter.free()?;
            iter.free()?;

            cx.breaks.pop();
            Asm::new(span, ())
        }
        Step::Unary { hir, span, addr } => {
            if diverging {
                addr.free()?;
                return Ok(Completed::Done(Asm::diverge(span)));
            }

            let addr = addr.into_addr()?;

            let inst = match hir.op {
                ast::UnOp::Not(..) => inst::Kind::Not {
                    addr: addr.addr(),
                    out: needs.alloc_output()?,
                },
                ast::UnOp::Neg(..) => inst::Kind::Neg {
                    addr: addr.addr(),
                    out: needs.alloc_output()?,
                },
                op => {
                    addr.free()?;

                    return Err(compile::Error::new(
                        span,
                        ErrorKind::UnsupportedUnaryOp { op },
                    ));
                }
            };

            cx.asm.push(inst, span)?;
            addr.free()?;
            Asm::new(span, ())
        }
        Step::Await { span, addr } => {
            if diverging {
                addr.free()?;
                return Ok(Completed::Done(Asm::diverge(span)));
            }

            let a = addr.addr()?.addr();

            cx.asm.push(
                inst::Kind::Await {
                    addr: a,
                    out: needs.alloc_output()?,
                },
                span,
            )?;

            addr.free()?;
            Asm::new(span, ())
        }
        Step::Try { span, addr } => {
            if diverging {
                addr.free()?;
                return Ok(Completed::Done(Asm::diverge(span)));
            }

            let a = addr.addr()?.addr();

            cx.asm.push(
                inst::Kind::Try {
                    addr: a,
                    out: needs.alloc_output()?,
                },
                span,
            )?;

            addr.free()?;
            Asm::new(span, ())
        }
        Step::FieldAccess { hir, span, addr } => {
            if diverging {
                addr.free()?;
                return Ok(Completed::Done(Asm::diverge(span)));
            }

            let a = addr.addr()?.addr();

            let inst = match hir.expr_field {
                hir::ExprField::Index(index) => inst::Kind::TupleIndexGetAt {
                    addr: a,
                    index,
                    out: needs.alloc_output()?,
                },
                hir::ExprField::Ident(field) => {
                    let slot = cx.q.unit.new_static_string(span, field)?;

                    inst::Kind::ObjectIndexGetAt {
                        addr: a,
                        slot,
                        out: needs.alloc_output()?,
                    }
                }
                _ => {
                    addr.free()?;
                    return Err(compile::Error::new(span, ErrorKind::BadFieldAccess));
                }
            };

            cx.asm.push(inst, span)?;
            addr.free()?;
            Asm::new(span, ())
        }
        Step::IndexTarget {
            hir,
            span,
            target,
            index,
        } => {
            if diverging {
                index.free()?;
                target.free()?;
                return Ok(Completed::Done(Asm::diverge(span)));
            }

            return Ok(Completed::Next(
                Step::IndexIndex {
                    span,
                    target,
                    index,
                },
                Current::Expr(cx.exprs.get(hir.index)),
            ));
        }
        Step::IndexIndex {
            span,
            target,
            index,
        } => {
            if diverging {
                index.free()?;
                target.free()?;
                return Ok(Completed::Done(Asm::diverge(span)));
            }

            let t = target.addr()?.addr();
            let i = index.addr()?.addr();

            cx.asm.push(
                inst::Kind::IndexGet {
                    index: i,
                    target: t,
                    out: needs.alloc_output()?,
                },
                span,
            )?;

            index.free()?;
            target.free()?;
            Asm::new(span, ())
        }
        Step::BinaryLhs { hir, span, a, b } => {
            if diverging {
                a.free()?;
                b.free()?;
                return Ok(Completed::Done(Asm::diverge(span)));
            }

            return Ok(Completed::Next(
                Step::BinaryRhs { hir, span, a, b },
                Current::Expr(cx.exprs.get(hir.rhs)),
            ));
        }
        Step::BinaryRhs { hir, span, a, b } => {
            if diverging {
                a.free()?;
                b.free()?;
                return Ok(Completed::Done(Asm::diverge(span)));
            }

            let lhs = a.addr()?.addr();
            let rhs = b.addr()?.addr();
            let out = needs.alloc_output()?;

            let inst = match binary_inst(hir.op, lhs, rhs, out, span) {
                Ok(inst) => inst,
                Err(error) => {
                    a.free()?;
                    b.free()?;
                    return Err(error);
                }
            };

            cx.asm.push(inst, span)?;
            a.free()?;
            b.free()?;
            Asm::new(span, ())
        }
        Step::ConditionalLhs { hir, span } => {
            if diverging {
                return Ok(Completed::Done(Asm::diverge(span)));
            }

            let end_label = cx.asm.new_label("conditional_end");
            let addr = needs.addr()?.addr();

            let lhs = cx.exprs.get(hir.lhs);

            match hir.op {
                ast::BinOp::And(..) => {
                    cx.asm.jump_if_not(addr, &end_label, lhs)?;
                }
                ast::BinOp::Or(..) => {
                    cx.asm.jump_if(addr, &end_label, lhs)?;
                }
                op => {
                    return Err(compile::Error::new(
                        span,
                        ErrorKind::UnsupportedBinaryOp { op },
                    ));
                }
            }

            return Ok(Completed::Next(
                Step::ConditionalRhs { span, end_label },
                Current::Expr(cx.exprs.get(hir.rhs)),
            ));
        }
        Step::ConditionalRhs { span, end_label } => {
            // The right-hand side is only evaluated for its effect on control
            // flow, so it diverging says nothing about the expression as a
            // whole - the left-hand side may still have short-circuited past
            // it.
            cx.asm.label(&end_label)?;
            Asm::new(span, ())
        }
    };

    Ok(Completed::Done(asm))
}

/// Assemble an expression which does not form part of a chain.
///
/// Expressions handled here may still nest - a block contains statements which
/// contain expressions - but only lexically, so their depth is bounded by the
/// nesting limit imposed while parsing rather than by the shape of the
/// expression itself.
fn expr_leaf<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::Expr<'hir>,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let asm = match hir.kind {
        hir::ExprKind::Variable(name) => {
            let var = cx.scopes.get(&mut cx.q, span, name)?;
            needs.assign_addr(cx, var.addr)?;
            Asm::new(span, ())
        }
        hir::ExprKind::Type(ty) => {
            if let Some(out) = needs.try_alloc_output()? {
                cx.asm.push(
                    inst::Kind::Store {
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
                cx.asm.push(inst::Kind::LoadFn { hash, out }, span)?;
            }

            Asm::new(span, ())
        }
        hir::ExprKind::Continue(hir) => expr_continue(cx, hir, span, needs)?,
        hir::ExprKind::Yield(None) => {
            let out = needs.alloc_output()?;
            cx.asm.push(inst::Kind::YieldUnit { out }, span)?;
            Asm::new(span, ())
        }
        hir::ExprKind::Return(None) => {
            cx.asm.push(inst::Kind::ReturnUnit, span)?;
            Asm::diverge(span)
        }
        hir::ExprKind::CallClosure(hir) => expr_call_closure(cx, hir, span, needs)?,
        hir::ExprKind::Lit(hir) => lit(cx, hir, span, needs)?,
        hir::ExprKind::AsyncBlock(hir) => expr_async_block(cx, hir, span, needs)?,
        hir::ExprKind::Const(id) => const_item(cx, id, span, needs)?,
        hir::ExprKind::Static(hash) => static_item(cx, hash, span, needs)?,
        // Handled over the work stack in `expr_stack`.
        hir::ExprKind::Assign(..)
        | hir::ExprKind::Break(..)
        | hir::ExprKind::Tuple(..)
        | hir::ExprKind::Block(..)
        | hir::ExprKind::Select(..)
        | hir::ExprKind::Template(..)
        | hir::ExprKind::Format(..)
        | hir::ExprKind::Vec(..)
        | hir::ExprKind::Object(..)
        | hir::ExprKind::Range(..)
        | hir::ExprKind::Return(Some(..))
        | hir::ExprKind::Yield(Some(..))
        | hir::ExprKind::For(..)
        | hir::ExprKind::Match(..)
        | hir::ExprKind::If(..)
        | hir::ExprKind::Loop(..)
        | hir::ExprKind::Group(..)
        | hir::ExprKind::Unary(..)
        | hir::ExprKind::Await(..)
        | hir::ExprKind::Try(..)
        | hir::ExprKind::FieldAccess(..)
        | hir::ExprKind::Index(..)
        | hir::ExprKind::Binary(..)
        | hir::ExprKind::Call(..) => {
            return Err(compile::Error::msg(
                span,
                "Expression should be assembled over the work stack",
            ))
        }
    };

    Ok(asm)
}

/// The instruction which implements a non-conditional, non-assigning binary
/// operator over the operands in `a` and `b`.
fn binary_inst(
    op: ast::BinOp,
    a: inst::Address,
    b: inst::Address,
    out: Output,
    span: &dyn Spanned,
) -> compile::Result<inst::Kind> {
    let inst = match op {
        ast::BinOp::Eq(..) => inst::Kind::Op {
            op: InstOp::Eq,
            a,
            b,
            out,
        },
        ast::BinOp::Neq(..) => inst::Kind::Op {
            op: InstOp::Neq,
            a,
            b,
            out,
        },
        ast::BinOp::Lt(..) => inst::Kind::Op {
            op: InstOp::Lt,
            a,
            b,
            out,
        },
        ast::BinOp::Gt(..) => inst::Kind::Op {
            op: InstOp::Gt,
            a,
            b,
            out,
        },
        ast::BinOp::Lte(..) => inst::Kind::Op {
            op: InstOp::Le,
            a,
            b,
            out,
        },
        ast::BinOp::Gte(..) => inst::Kind::Op {
            op: InstOp::Ge,
            a,
            b,
            out,
        },
        ast::BinOp::As(..) => inst::Kind::Op {
            op: InstOp::As,
            a,
            b,
            out,
        },
        ast::BinOp::Is(..) => inst::Kind::Op {
            op: InstOp::Is,
            a,
            b,
            out,
        },
        ast::BinOp::IsNot(..) => inst::Kind::Op {
            op: InstOp::IsNot,
            a,
            b,
            out,
        },
        ast::BinOp::And(..) => inst::Kind::Op {
            op: InstOp::And,
            a,
            b,
            out,
        },
        ast::BinOp::Or(..) => inst::Kind::Op {
            op: InstOp::Or,
            a,
            b,
            out,
        },
        ast::BinOp::Add(..) => inst::Kind::Arithmetic {
            op: InstArithmeticOp::Add,
            a,
            b,
            out,
        },
        ast::BinOp::Sub(..) => inst::Kind::Arithmetic {
            op: InstArithmeticOp::Sub,
            a,
            b,
            out,
        },
        ast::BinOp::Div(..) => inst::Kind::Arithmetic {
            op: InstArithmeticOp::Div,
            a,
            b,
            out,
        },
        ast::BinOp::Mul(..) => inst::Kind::Arithmetic {
            op: InstArithmeticOp::Mul,
            a,
            b,
            out,
        },
        ast::BinOp::Rem(..) => inst::Kind::Arithmetic {
            op: InstArithmeticOp::Rem,
            a,
            b,
            out,
        },
        ast::BinOp::BitAnd(..) => inst::Kind::Bitwise {
            op: InstBitwiseOp::BitAnd,
            a,
            b,
            out,
        },
        ast::BinOp::BitXor(..) => inst::Kind::Bitwise {
            op: InstBitwiseOp::BitXor,
            a,
            b,
            out,
        },
        ast::BinOp::BitOr(..) => inst::Kind::Bitwise {
            op: InstBitwiseOp::BitOr,
            a,
            b,
            out,
        },
        ast::BinOp::Shl(..) => inst::Kind::Shift {
            op: InstShiftOp::Shl,
            a,
            b,
            out,
        },
        ast::BinOp::Shr(..) => inst::Kind::Shift {
            op: InstShiftOp::Shr,
            a,
            b,
            out,
        },

        op => {
            return Err(compile::Error::new(
                span,
                ErrorKind::UnsupportedBinaryOp { op },
            ));
        }
    };

    Ok(inst)
}

/// The instruction which applies a compound assignment operator.
fn assign_binop_inst(
    bin_op: &ast::BinOp,
    target: InstTarget,
    rhs: inst::Address,
    span: &dyn Spanned,
) -> compile::Result<inst::Kind> {
    let inst = match bin_op {
        ast::BinOp::AddAssign(..) => inst::Kind::AssignArithmetic {
            op: InstArithmeticOp::Add,
            target,
            rhs,
        },
        ast::BinOp::SubAssign(..) => inst::Kind::AssignArithmetic {
            op: InstArithmeticOp::Sub,
            target,
            rhs,
        },
        ast::BinOp::MulAssign(..) => inst::Kind::AssignArithmetic {
            op: InstArithmeticOp::Mul,
            target,
            rhs,
        },
        ast::BinOp::DivAssign(..) => inst::Kind::AssignArithmetic {
            op: InstArithmeticOp::Div,
            target,
            rhs,
        },
        ast::BinOp::RemAssign(..) => inst::Kind::AssignArithmetic {
            op: InstArithmeticOp::Rem,
            target,
            rhs,
        },
        ast::BinOp::BitAndAssign(..) => inst::Kind::AssignBitwise {
            op: InstBitwiseOp::BitAnd,
            target,
            rhs,
        },
        ast::BinOp::BitXorAssign(..) => inst::Kind::AssignBitwise {
            op: InstBitwiseOp::BitXor,
            target,
            rhs,
        },
        ast::BinOp::BitOrAssign(..) => inst::Kind::AssignBitwise {
            op: InstBitwiseOp::BitOr,
            target,
            rhs,
        },
        ast::BinOp::ShlAssign(..) => inst::Kind::AssignShift {
            op: InstShiftOp::Shl,
            target,
            rhs,
        },
        ast::BinOp::ShrAssign(..) => inst::Kind::AssignShift {
            op: InstShiftOp::Shr,
            target,
            rhs,
        },
        _ => {
            return Err(compile::Error::new(span, ErrorKind::UnsupportedBinaryExpr));
        }
    };

    Ok(inst)
}

/// Assemble a block expression.
#[instrument_ast(span = span)]
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
        inst::Kind::Call {
            hash: hir.hash,
            addr: linear.addr(),
            args: hir.captures.len(),
            out: needs.alloc_output()?,
        },
        span,
        &"async block",
    )?;

    linear.free_non_dangling()?;
    Ok(Asm::new(span, ()))
}

/// Assemble a constant item.
#[instrument_ast(span = span)]
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

/// Assemble reading a static item out of the global storage of the running vm.
#[instrument_ast(span = span)]
fn static_item<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hash: Hash,
    span: &'hir dyn Spanned,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let slot = global_slot(cx, hash, span)?;

    if let Some(out) = needs.try_alloc_output()? {
        cx.asm.push(inst::Kind::GlobalGet { slot, out }, span)?;
    }

    Ok(Asm::new(span, ()))
}

/// Assemble a closure expression.
#[instrument_ast(span = span)]
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
        inst::Kind::Closure {
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
#[instrument_ast(span = span)]
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

/// Reorder the position of the field assignments on the stack so that they
/// match the expected argument order when invoking the constructor function.
fn reorder_field_assignments<'hir>(
    cx: &mut Ctxt<'_, 'hir, '_>,
    hir: &hir::ExprObject<'hir>,
    base: inst::Address,
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

            let a = inst::Address::new(a);
            let b = inst::Address::new(b);
            cx.asm.push(inst::Kind::Swap { a, b }, span)?;
        }
    }

    Ok(())
}

/// Assemble a literal value.
#[instrument_ast(span = span)]
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
            cx.asm.push(inst::Kind::bool(v, out), span)?;
        }
        hir::Lit::Char(v) => {
            cx.asm.push(inst::Kind::char(v, out), span)?;
        }
        hir::Lit::Unsigned(v) => {
            cx.asm.push(inst::Kind::unsigned(v, out), span)?;
        }
        hir::Lit::Signed(v) => {
            cx.asm.push(inst::Kind::signed(v, out), span)?;
        }
        hir::Lit::Float(v) => {
            cx.asm.push(inst::Kind::float(v, out), span)?;
        }
        hir::Lit::Str(string) => {
            let slot = cx.q.unit.new_static_string(span, string)?;
            cx.asm.push(inst::Kind::String { slot, out }, span)?;
        }
        hir::Lit::ByteStr(bytes) => {
            let slot = cx.q.unit.new_static_bytes(span, bytes)?;
            cx.asm.push(inst::Kind::Bytes { slot, out }, span)?;
        }
    };

    Ok(Asm::new(span, ()))
}

/// Assemble a local expression.
#[instrument_ast(span = hir)]
fn local<'a, 'hir>(
    cx: &mut Ctxt<'a, 'hir, '_>,
    hir: &'hir hir::Local<'hir>,
    needs: &mut dyn Needs<'a, 'hir>,
) -> compile::Result<Asm<'hir>> {
    let mut load = |cx: &mut Ctxt<'a, 'hir, '_>, needs: &mut dyn Needs<'a, 'hir>| {
        expr(cx, cx.exprs.get(hir.expr), needs)
    };

    converge!(pattern_panic(cx, &hir.pat, |cx, false_label| {
        pat_binding(cx, &hir.pat, false_label, &mut load)
    })?);

    // If a value is needed for a let expression, it is evaluated as a unit.
    if let Some(out) = needs.try_alloc_output()? {
        cx.asm.push(inst::Kind::unit(out), hir)?;
    }

    Ok(Asm::new(hir, ()))
}
