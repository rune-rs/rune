use core::mem::replace;
use core::num::NonZero;

use rust_alloc::rc::Rc;

use tracing::instrument_ast;

use crate::alloc::prelude::*;
use crate::alloc::{self, Box, HashMap, HashSet};
use crate::ast::{self, Delimiter, Kind, NumberSize, Span, Spanned};
use crate::compile::{meta, Error, ErrorKind, ItemId, Result, WithSpan};
use crate::grammar::{
    classify, object_key, Ignore, MaybeNode, Node, NodeClass, Remaining, Stream, StreamBuf, Tree,
};
use crate::hash::ParametersBuilder;
use crate::hir::{self, alloc_with};
use crate::internal_macros::resolve_context;
use crate::parse::{NonZeroId, Resolve};
use crate::query::{self, GenericsParameters, Named2, Named2Kind, Used};
use crate::runtime::{
    self, format, ConstInstance, ConstValue, ConstValueKind, Inline, Type, TypeHash,
};
use crate::Hash;

use super::{Ctxt, Needs};

use Kind::*;

/// Lower a bare function.
#[instrument_ast(span = p)]
pub(crate) fn bare<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
    args: &'hir [String],
) -> Result<hir::ItemFn<'hir>> {
    alloc_with!(cx, p);

    let args = iter!(args, |name| named_arg(cx, name, p)?);
    let body = statements(cx, None, p)?;

    Ok(hir::ItemFn {
        span: p.span(),
        args,
        body,
    })
}

fn named_arg<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    name: &'hir str,
    span: &dyn Spanned,
) -> Result<hir::FnArg<'hir>> {
    alloc_with!(cx, span);

    let name = cx.scopes.define(hir::Name::Str(name), span)?;
    let names = iter!([name]);

    let pat = alloc!(hir::PatBinding {
        pat: hir::Pat {
            span: span.span(),
            kind: hir::PatKind::Path(alloc!(hir::PatPathKind::Ident(name))),
        },
        names,
    });

    Ok(hir::FnArg::Pat(pat))
}

/// Lower a function item.
#[instrument_ast(span = p)]
pub(crate) fn item_fn<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
    is_instance: bool,
) -> Result<hir::ItemFn<'hir>> {
    alloc_with!(cx, p);

    p.remaining(cx, Attribute)?.ignore(cx)?;
    p.eat(Modifiers);
    p.expect(K![fn])?;
    p.ast::<ast::Ident>()?;

    let mut args = Vec::new();

    p.expect(FnArgs)?.parse(|p| {
        p.expect(K!['('])?;

        let mut comma = Remaining::default();

        while let MaybeNode::Some(pat) = p.eat(Pat) {
            comma.exactly_one(cx)?;
            let pat = pat.parse(|p| self::pat_binding_with(cx, p, is_instance))?;
            args.try_push(hir::FnArg::Pat(alloc!(pat)))?;
            comma = p.one(K![,]);
        }

        comma.at_most_one(cx)?;
        p.expect(K![')'])?;
        Ok(())
    })?;

    let body = p.expect(Block)?.parse(|p| block(cx, None, p))?;

    Ok(hir::ItemFn {
        span: p.span(),
        args: iter!(args),
        body,
    })
}

/// Lower a block.
#[instrument_ast(span = p)]
pub(crate) fn block<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    label: Option<ast::Label>,
    p: &mut Stream<'_>,
) -> Result<hir::Block<'hir>> {
    p.expect(K!['{'])?;
    let block = p.expect(BlockBody)?.parse(|p| statements(cx, label, p))?;
    p.expect(K!['}'])?;
    Ok(block)
}

#[instrument_ast(span = p)]
fn statements<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    label: Option<ast::Label>,
    p: &mut Stream<'_>,
) -> Result<hir::Block<'hir>> {
    alloc_with!(cx, p);

    let label = match label {
        Some(label) => Some(alloc_str!(label.resolve(resolve_context!(cx.q))?)),
        None => None,
    };

    cx.scopes.push(label)?;

    let at = cx.statements.len();

    let mut must_be_last = None;

    // NB: This must start as true to avoid the last statement from being
    // included if none exists.
    let mut last_item = true;

    while let Some(node) = p.next() {
        let (needs_semi, class) = classify(&node);

        let span = node.span();

        match node.kind() {
            Local => {
                let stmt = hir::Stmt::Local(alloc!(node.parse(|p| local(cx, p))?));
                cx.statements.try_push(stmt)?;
            }
            Expr => {
                let expr = node.parse(|p| expr(cx, p))?;
                let stmt = hir::Stmt::Expr(expr.span, expr!(expr));
                cx.statements.try_push(stmt)?;
            }
            Item => {
                let semi = p.remaining(cx, K![;])?;

                if needs_semi {
                    semi.exactly_one(cx)?;
                } else {
                    semi.at_most_one(cx)?;
                }

                last_item = true;
                continue;
            }
            _ => {
                cx.error(node.expected("an expression or local"))?;
                continue;
            }
        };

        let semis = p.remaining(cx, K![;])?;

        last_item = semis.is_present();

        if let Some(span) = must_be_last {
            cx.error(Error::new(
                span,
                ErrorKind::ExpectedBlockSemiColon {
                    #[cfg(feature = "emit")]
                    followed_span: span,
                },
            ))?;
        }

        if matches!(class, NodeClass::Expr) && semis.is_absent() {
            must_be_last = Some(span);
        }

        if let Some(span) = semis.trailing() {
            cx.error(Error::msg(span, "unused semi-colons"))?;
        }

        if needs_semi {
            semis.at_least_one(cx)?;
        } else {
            semis.at_most_one(cx)?;
        }
    }

    let value = 'out: {
        if last_item {
            break 'out None;
        }

        debug_assert!(
            at < cx.statements.len(),
            "starting point for assertions must be prior to buffer size"
        );

        match cx.statements.pop() {
            Some(hir::Stmt::Expr(_, e)) => Some(e),
            Some(stmt) => {
                cx.statements.try_push(stmt).with_span(&*p)?;
                None
            }
            None => None,
        }
    };

    let statements = iter!(cx.statements.drain(at..));

    let layer = cx.scopes.pop().with_span(&*p)?;

    Ok(hir::Block {
        span: p.span(),
        label,
        statements,
        value,
        drop: iter!(layer.into_drop_order()),
    })
}

/// Reject attributes on a local declaration.
///
/// A statement is parsed without deciding up front whether it is an item, an
/// expression or a local, so attributes are accepted by the grammar and it
/// falls to whatever the statement turns out to be to reject them.
fn deny_local_attributes(p: &mut Stream<'_>) -> Result<()> {
    if let MaybeNode::Some(node) = p.eat(Attribute) {
        return Err(Error::msg(
            &node,
            "Attributes on local declarations are not supported",
        ));
    }

    Ok(())
}

/// Lower a local.
#[instrument_ast(span = p)]
pub(crate) fn local<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::Local<'hir>> {
    alloc_with!(cx, p);

    // Note: expression needs to be assembled before pattern, otherwise the
    // expression will see declarations in the pattern.

    deny_local_attributes(p)?;
    p.expect(K![let])?;
    let pat = p.expect(Pat)?;
    p.expect(K![=])?;
    let expr = p.expect(Expr)?;

    let expr = expr.parse(|p| self::expr(cx, p))?;
    let pat = pat.parse(|p| self::pat_binding(cx, p))?;

    Ok(hir::Local {
        span: p.span(),
        pat,
        expr: expr!(expr),
    })
}

/// Lower an expression.
#[instrument_ast(span = p)]
pub(crate) fn expr<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::Expr<'hir>> {
    let node = expr_prefix(cx, p)?;
    let kind = inner_node(cx, node)?;

    if let Some(label) = cx.label.take() {
        return Err(Error::msg(label, "labels are not supported for expression"));
    };

    Ok(hir::Expr {
        span: p.span(),
        kind,
    })
}

#[instrument_ast(span = p)]
fn expr_only<'hir>(cx: &mut Ctxt<'hir, '_, '_>, p: &mut Stream<'_>) -> Result<hir::Expr<'hir>> {
    let kind = expr_inner(cx, p)?.into_kind(cx)?;

    Ok(hir::Expr {
        span: p.span(),
        kind,
    })
}

/// An expression which is partially lowered, waiting on one of its children.
enum ExprStep<'hir, 'a> {
    /// Finish the `Expr` node wrapping an inner expression.
    Wrapper { buf: StreamBuf<'a> },
    /// Wrap the kind of an inner expression which had no `Expr` node around it.
    Only { span: Span },
    /// Finish a group, `( .. )`.
    Group { buf: StreamBuf<'a>, empty: bool },
    /// Finish a unary expression.
    Unary { buf: StreamBuf<'a>, op: ast::UnOp },
    /// Finish an assignment, having lowered its left-hand side.
    AssignRhs { buf: StreamBuf<'a> },
    /// Finish an assignment, having lowered its right-hand side.
    Assign {
        buf: StreamBuf<'a>,
        lhs: hir::ExprId,
    },
    /// Continue a sequence of expressions, `[a, b]` or `(a, b)`.
    Seq {
        buf: StreamBuf<'a>,
        items: Vec<hir::Expr<'hir>>,
        comma: Remaining<'a>,
        array: bool,
    },
    /// Finish a range which has one operand.
    Range1 { buf: StreamBuf<'a>, kind: RangeKind },
    /// Finish a block expression, `{ .. }`.
    BlockExpr { buf: StreamBuf<'a> },
    /// Finish the braces around a block body.
    Block { buf: StreamBuf<'a> },
    /// Finish a `return` or `yield` which has an operand.
    OptExpr { buf: StreamBuf<'a>, yielded: bool },
    /// Finish a `break` which has an operand.
    Break {
        buf: StreamBuf<'a>,
        label: Option<ast::Label>,
    },
    /// Finish a `loop`, having lowered its body.
    Loop {
        buf: StreamBuf<'a>,
        label: Option<&'hir str>,
        condition: Option<&'hir hir::Condition<'hir>>,
    },
    /// Continue a `for`, having lowered its iterator.
    ForIter {
        buf: StreamBuf<'a>,
        label: Option<&'hir str>,
        pat: Node<'a>,
        block: Node<'a>,
    },
    /// Finish a `for`, having lowered its body.
    For {
        buf: StreamBuf<'a>,
        label: Option<&'hir str>,
        binding: hir::PatBinding<'hir>,
        iter: hir::ExprId,
    },
    /// Finish a branch of an `if`, having lowered its block.
    IfBranch {
        buf: StreamBuf<'a>,
        start: Span,
        branches: Vec<hir::ConditionalBranch<'hir>>,
        condition: &'hir hir::Condition<'hir>,
        else_buf: Option<StreamBuf<'a>>,
    },
    /// Finish the fallback of an `if`, having lowered its block.
    IfElse {
        buf: StreamBuf<'a>,
        branches: Vec<hir::ConditionalBranch<'hir>>,
        else_buf: StreamBuf<'a>,
    },
    /// Finish the subject of a `match`.
    MatchSubject { buf: StreamBuf<'a> },
    /// Finish a match arm which has just lowered its guard.
    MatchGuard(Box<StepMatchGuard<'hir, 'a>>),
    /// Finish a match arm which has just lowered its body.
    MatchBody(Box<StepMatchBody<'hir, 'a>>),
    /// Finish the left-hand side of a binary chain.
    BinaryLhs { buf: StreamBuf<'a> },
    /// Finish an operand of a binary chain.
    BinaryRhs {
        buf: StreamBuf<'a>,
        lhs: hir::Expr<'hir>,
        op: ast::BinOp,
        needs: Needs,
    },
    /// Finish an async block, having lowered its body.
    AsyncBlock(Box<StepAsyncBlock<'a>>),
    /// Finish a const block, having lowered its body.
    ConstBlock { buf: StreamBuf<'a> },
    /// Finish a closure, having lowered its body.
    Closure(Box<StepClosure<'hir, 'a>>),
    /// Finish the base of a chain.
    ChainBase { buf: StreamBuf<'a> },
    /// Wrap the kind of a chain base into an inner expression.
    BaseInner { span: Span },
    /// Continue the arguments of a call in a chain.
    ChainCall(Box<StepChainCall<'hir, 'a>>),
    /// Finish an index in a chain.
    ChainIndex(Box<StepChainIndex<'hir, 'a>>),
    /// Finish the value of an object field.
    ObjectValue {
        buf: StreamBuf<'a>,
        key_node: Node<'a>,
        assignments: Vec<hir::FieldAssign<'hir>>,
        keys_dup: HashMap<&'hir str, Span>,
        key: (Span, &'hir str),
    },
    /// Finish the default arm of a `select`.
    SelectDefault(Box<StepSelectDefault<'hir, 'a>>),
    /// Finish the value of a `select` arm.
    SelectValue(Box<StepSelectValue<'hir, 'a>>),
    /// Finish the body of a `select` arm.
    SelectBody(Box<StepSelectBody<'hir, 'a>>),
    /// Finish a `let` condition, having lowered its expression.
    CondLet { buf: StreamBuf<'a>, pat: Node<'a> },
    /// Wrap a plain expression as a condition.
    CondExpr,
    /// Continue a `while`, having lowered its condition.
    WhileCond {
        buf: StreamBuf<'a>,
        label: Option<&'hir str>,
    },
    /// Continue an `if`, having lowered its condition.
    IfCond {
        buf: StreamBuf<'a>,
        start: Span,
        branches: Vec<hir::ConditionalBranch<'hir>>,
        else_buf: Option<StreamBuf<'a>>,
    },
    /// Continue the statements of a block body.
    Stmts {
        buf: StreamBuf<'a>,
        label: Option<&'hir str>,
        at: usize,
        must_be_last: Option<Span>,
        pending: Option<Pending>,
    },
    /// Finish a local declaration, having lowered its expression.
    Local { buf: StreamBuf<'a>, pat: Node<'a> },
    /// Finish a range which has two operands, having lowered its start.
    RangeEnd {
        buf: StreamBuf<'a>,
        start: Option<hir::ExprId>,
        inclusive: bool,
    },
}

/// Finish an async block, having lowered its body.
///
/// Stored behind a [`Box`] so that the size of a work stack entry is not
/// dominated by the largest construct.
struct StepAsyncBlock<'a> {
    buf: StreamBuf<'a>,
    meta: meta::Meta,
}

/// Finish a closure, having lowered its body.
///
/// Stored behind a [`Box`] so that the size of a work stack entry is not
/// dominated by the largest construct.
struct StepClosure<'hir, 'a> {
    buf: StreamBuf<'a>,
    meta: meta::Meta,
    args: &'hir [hir::FnArg<'hir>],
}

/// Finish a match arm which has just lowered its guard.
///
/// Stored behind a [`Box`] so that the size of a work stack entry is not
/// dominated by the largest construct.
struct StepMatchGuard<'hir, 'a> {
    buf: StreamBuf<'a>,
    arm: StreamBuf<'a>,
    subject: hir::ExprId,
    branches: Vec<hir::ExprMatchBranch<'hir>>,
    pat: hir::PatBinding<'hir>,
}

/// Finish a match arm which has just lowered its body.
///
/// Stored behind a [`Box`] so that the size of a work stack entry is not
/// dominated by the largest construct.
struct StepMatchBody<'hir, 'a> {
    buf: StreamBuf<'a>,
    arm: StreamBuf<'a>,
    subject: hir::ExprId,
    branches: Vec<hir::ExprMatchBranch<'hir>>,
    pat: hir::PatBinding<'hir>,
    condition: Option<hir::ExprId>,
    was_block: bool,
}

/// Continue the arguments of a call in a chain.
///
/// Stored behind a [`Box`] so that the size of a work stack entry is not
/// dominated by the largest construct.
struct StepChainCall<'hir, 'a> {
    buf: StreamBuf<'a>,
    call: StreamBuf<'a>,
    inner: ExprInner<'hir, 'a>,
    args: Vec<hir::Expr<'hir>>,
    comma: Remaining<'a>,
    start: Span,
}

/// Finish an index in a chain.
///
/// Stored behind a [`Box`] so that the size of a work stack entry is not
/// dominated by the largest construct.
struct StepChainIndex<'hir, 'a> {
    buf: StreamBuf<'a>,
    index: StreamBuf<'a>,
    inner: ExprInner<'hir, 'a>,
    start: Span,
}

/// Finish the default arm of a `select`.
///
/// Stored behind a [`Box`] so that the size of a work stack entry is not
/// dominated by the largest construct.
struct StepSelectDefault<'hir, 'a> {
    buf: StreamBuf<'a>,
    arm: StreamBuf<'a>,
    state: SelectState<'hir>,
    default_span: Span,
    was_block: bool,
}

/// Finish the value of a `select` arm.
///
/// Stored behind a [`Box`] so that the size of a work stack entry is not
/// dominated by the largest construct.
struct StepSelectValue<'hir, 'a> {
    buf: StreamBuf<'a>,
    arm: StreamBuf<'a>,
    state: SelectState<'hir>,
    pat: hir::PatBinding<'hir>,
}

/// Finish the body of a `select` arm.
///
/// Stored behind a [`Box`] so that the size of a work stack entry is not
/// dominated by the largest construct.
struct StepSelectBody<'hir, 'a> {
    buf: StreamBuf<'a>,
    arm: StreamBuf<'a>,
    state: SelectState<'hir>,
    pat: hir::PatBinding<'hir>,
    was_block: bool,
}

/// Which one-operand range is being lowered.
#[derive(Debug, Clone, Copy)]
enum RangeKind {
    From,
    To,
    ToInclusive,
}

/// What a `select` has accumulated so far.
struct SelectState<'hir> {
    exprs: Vec<hir::Expr<'hir>>,
    branches: Vec<hir::ExprSelectBranch<'hir>>,
    default: Option<(Span, hir::ExprId)>,
}

/// What lowering an expression produced.
enum ExprState<'hir, 'a> {
    /// An inner expression produced this kind.
    Kind(hir::ExprKind<'hir>),
    /// A complete expression, to be handed to the step below.
    Expr(hir::Expr<'hir>),
    /// A complete block, to be handed to the step below.
    Block(hir::Block<'hir>),
    /// A complete local declaration, to be handed to the step below.
    Local(hir::Local<'hir>),
    /// The base of a chain, which may still be an unresolved path.
    Inner(ExprInner<'hir, 'a>),
    /// A complete condition, to be handed to the step below.
    Condition(hir::Condition<'hir>),
    /// Park the step and lower the given node next.
    Child(ExprStep<'hir, 'a>, Node<'a>, Start),
}

/// How a child node should be started.
#[derive(Debug, Clone, Copy)]
enum Start {
    /// The child is an `Expr` node, as produced by `p.expect(Expr)`.
    Wrapped,
    /// The child is the inner expression, as produced by `p.pump()`.
    Inner,
    /// The child is a `BlockBody` node, lowered with the given label.
    Body { label: Option<ast::Label> },
    /// The child is a `Local` node.
    Local,
    /// The child is a `Block` node.
    Block,
    /// The child is the base of a chain.
    Base { span: Span },
    /// The child is the condition of an `if` or a `while`.
    Condition,
}

/// Lower the inner expression in the given node.
///
/// Expressions nest through more or less every construct in the language.
/// Rather than recursing, the nesting is kept on a heap allocated stack.
/// Constructs which have not been converted yet fall back to lowering
/// recursively, which is why this is safe to land in pieces.
fn inner_node<'hir, 'a>(
    cx: &mut Ctxt<'hir, '_, '_>,
    node: Node<'a>,
) -> Result<hir::ExprKind<'hir>> {
    // The driver is re-entered for constructs which are not driven over its
    // own stack, such as a block body, so it counts from wherever the caller
    // had got to and hands the depth back on the way out.
    let base = cx.const_depth;
    let result = inner_node_inner(cx, node, base);
    cx.const_depth = base;
    result
}

fn inner_node_inner<'hir, 'a>(
    cx: &mut Ctxt<'hir, '_, '_>,
    node: Node<'a>,
    base: usize,
) -> Result<hir::ExprKind<'hir>> {
    let span = node.span();
    let mut stack = Vec::new();
    let mut state = inner_start(cx, node)?;

    loop {
        cx.const_depth = base + stack.len();

        match state {
            ExprState::Kind(kind) => {
                let Some(step) = stack.pop() else {
                    return Ok(kind);
                };

                state = expr_resume(cx, step, ExprState::Kind(kind))?;
            }
            ExprState::Expr(expr) => {
                let Some(step) = stack.pop() else {
                    return Err(Error::msg(expr, "Expression without a parent"));
                };

                state = expr_resume(cx, step, ExprState::Expr(expr))?;
            }
            ExprState::Block(block) => {
                let Some(step) = stack.pop() else {
                    return Err(Error::msg(block, "Block without a parent"));
                };

                state = expr_resume(cx, step, ExprState::Block(block))?;
            }
            ExprState::Local(local) => {
                let Some(step) = stack.pop() else {
                    return Err(Error::msg(local, "Local without a parent"));
                };

                state = expr_resume(cx, step, ExprState::Local(local))?;
            }
            ExprState::Inner(inner) => {
                let Some(step) = stack.pop() else {
                    return Err(Error::msg(inner.span, "Chain base without a parent"));
                };

                state = expr_resume(cx, step, ExprState::Inner(inner))?;
            }
            ExprState::Condition(condition) => {
                let Some(step) = stack.pop() else {
                    return Err(Error::msg(Span::empty(), "Condition without a parent"));
                };

                state = expr_resume(cx, step, ExprState::Condition(condition))?;
            }
            ExprState::Child(step, node, start) => {
                cx.const_nesting(&node, base + stack.len())?;
                stack.try_push(step).with_span(span)?;

                state = match start {
                    Start::Wrapped => {
                        let mut buf = node.into_stream();
                        let inner = expr_prefix(cx, buf.stream())?;
                        stack.try_push(ExprStep::Wrapper { buf }).with_span(span)?;
                        inner_start(cx, inner)?
                    }
                    Start::Inner => {
                        stack
                            .try_push(ExprStep::Only { span: node.span() })
                            .with_span(span)?;
                        inner_start(cx, node)?
                    }
                    Start::Body { label } => stmts_start(cx, node.into_stream(), label)?,
                    Start::Condition => {
                        let span = node.span();

                        if matches!(node.kind(), Condition) {
                            let mut buf = node.into_stream();

                            let (pat, node) = {
                                let p = buf.stream();
                                p.expect(K![let])?;
                                let pat = p.expect(Pat)?;
                                p.expect(K![=])?;
                                (pat, p.expect(Expr)?)
                            };

                            stack
                                .try_push(ExprStep::CondLet { buf, pat })
                                .with_span(span)?;

                            let mut buf = node.into_stream();
                            let inner = expr_prefix(cx, buf.stream())?;
                            stack.try_push(ExprStep::Wrapper { buf }).with_span(span)?;
                            inner_start(cx, inner)?
                        } else {
                            stack.try_push(ExprStep::CondExpr).with_span(span)?;
                            let mut buf = node.into_stream();
                            let inner = expr_prefix(cx, buf.stream())?;
                            stack.try_push(ExprStep::Wrapper { buf }).with_span(span)?;
                            inner_start(cx, inner)?
                        }
                    }
                    Start::Base { span } => {
                        stack
                            .try_push(ExprStep::BaseInner { span })
                            .with_span(span)?;
                        inner_start(cx, node)?
                    }
                    Start::Block => {
                        let mut buf = node.into_stream();

                        let node = {
                            let p = buf.stream();
                            p.expect(K!['{'])?;
                            p.expect(BlockBody)?
                        };

                        ExprState::Child(ExprStep::Block { buf }, node, Start::Body { label: None })
                    }
                    Start::Local => {
                        let mut buf = node.into_stream();

                        let (pat, node) = {
                            let p = buf.stream();
                            deny_local_attributes(p)?;
                            p.expect(K![let])?;
                            let pat = p.expect(Pat)?;
                            p.expect(K![=])?;
                            (pat, p.expect(Expr)?)
                        };

                        ExprState::Child(ExprStep::Local { buf, pat }, node, Start::Wrapped)
                    }
                };
            }
        }
    }
}

/// Consume the attributes, modifiers and labels which precede an expression,
/// returning the node holding the expression itself.
fn expr_prefix<'hir, 'a>(cx: &mut Ctxt<'hir, '_, '_>, p: &mut Stream<'a>) -> Result<Node<'a>> {
    p.remaining(cx, Attribute)?.ignore(cx)?;
    p.eat(Modifiers);

    while let MaybeNode::Some(label) = p.eat_matching(|k| matches!(k, K!['label])) {
        let label = label.ast::<ast::Label>()?;

        if let Some(existing) = &cx.label {
            cx.error(Error::new(
                label.span(),
                ErrorKind::ConflictingLabels {
                    existing: existing.span(),
                },
            ))?;
        } else {
            cx.label = Some(label);
        }

        p.one(K![:]).exactly_one(cx)?;
    }

    p.pump()
}

/// Start lowering an inner expression, parking a step if it has children.
fn inner_start<'hir, 'a>(
    cx: &mut Ctxt<'hir, '_, '_>,
    node: Node<'a>,
) -> Result<ExprState<'hir, 'a>> {
    let mut buf = node.into_stream();

    match buf.stream().kind() {
        ExprGroup => {
            {
                let p = buf.stream();
                p.expect(K!['('])?;
            }

            if let MaybeNode::Some(node) = buf.stream().eat(Expr) {
                return Ok(ExprState::Child(
                    ExprStep::Group { buf, empty: false },
                    node,
                    Start::Wrapped,
                ));
            }

            let kind = {
                let p = buf.stream();
                alloc_with!(cx, p);

                let expr = hir::Expr {
                    span: p.span(),
                    kind: hir::ExprKind::Tuple(&hir::ExprSeq { items: &[] }),
                };

                p.expect(K![')'])?;
                hir::ExprKind::Group(expr!(expr))
            };

            buf.end()?;
            Ok(ExprState::Kind(kind))
        }
        ExprEmptyGroup => {
            let node = {
                let p = buf.stream();
                p.expect(Kind::Open(Delimiter::Empty))?;
                p.expect(Expr)?
            };

            Ok(ExprState::Child(
                ExprStep::Group { buf, empty: true },
                node,
                Start::Wrapped,
            ))
        }
        ExprUnary => {
            let (op, node) = {
                let p = buf.stream();
                let op = p.ast::<ast::UnOp>()?;

                if let ast::UnOp::BorrowRef { .. } = op {
                    return Err(Error::new(op, ErrorKind::UnsupportedRef));
                }

                (op, p.pump()?)
            };

            Ok(ExprState::Child(
                ExprStep::Unary { buf, op },
                node,
                Start::Inner,
            ))
        }
        ExprAssign => {
            let node = buf.stream().expect(Expr)?;

            Ok(ExprState::Child(
                ExprStep::AssignRhs { buf },
                node,
                Start::Wrapped,
            ))
        }
        ExprArray => {
            {
                let p = buf.stream();
                p.expect(K!['['])?;
            }

            expr_seq_next(cx, buf, Vec::new(), Remaining::default(), true)
        }
        ExprTuple => {
            {
                let p = buf.stream();
                p.expect(K!['('])?;
            }

            expr_seq_next(cx, buf, Vec::new(), Remaining::default(), false)
        }
        ExprRangeFrom => {
            let node = {
                let p = buf.stream();
                let node = p.pump()?;
                p.expect(K![..])?;
                node
            };

            Ok(ExprState::Child(
                ExprStep::Range1 {
                    buf,
                    kind: RangeKind::From,
                },
                node,
                Start::Inner,
            ))
        }
        ExprRangeTo => {
            let node = {
                let p = buf.stream();
                p.expect(K![..])?;
                p.pump()?
            };

            Ok(ExprState::Child(
                ExprStep::Range1 {
                    buf,
                    kind: RangeKind::To,
                },
                node,
                Start::Inner,
            ))
        }
        ExprRangeToInclusive => {
            let node = {
                let p = buf.stream();
                p.expect(K![..=])?;
                p.pump()?
            };

            Ok(ExprState::Child(
                ExprStep::Range1 {
                    buf,
                    kind: RangeKind::ToInclusive,
                },
                node,
                Start::Inner,
            ))
        }
        ExprRange | ExprRangeInclusive => {
            let inclusive = matches!(buf.stream().kind(), ExprRangeInclusive);
            let node = buf.stream().pump()?;

            Ok(ExprState::Child(
                ExprStep::RangeEnd {
                    buf,
                    start: None,
                    inclusive,
                },
                node,
                Start::Inner,
            ))
        }
        ExprRangeFull => {
            let kind = {
                let p = buf.stream();
                p.expect(K![..])?;
                alloc_with!(cx, p);
                hir::ExprKind::Range(alloc!(hir::ExprRange::RangeFull))
            };

            buf.end()?;
            Ok(ExprState::Kind(kind))
        }
        ExprReturn | ExprYield => {
            let yielded = matches!(buf.stream().kind(), ExprYield);

            let node = {
                let p = buf.stream();
                p.expect(if yielded { K![yield] } else { K![return] })?;
                p.eat(Expr)
            };

            if let MaybeNode::Some(node) = node {
                return Ok(ExprState::Child(
                    ExprStep::OptExpr { buf, yielded },
                    node,
                    Start::Wrapped,
                ));
            }

            let kind = if yielded {
                hir::ExprKind::Yield(None)
            } else {
                hir::ExprKind::Return(None)
            };

            buf.end()?;
            Ok(ExprState::Kind(kind))
        }
        ExprBreak => {
            let (label, node) = {
                let p = buf.stream();
                p.expect(K![break])?;

                let label = p
                    .eat_matching(|k| matches!(k, K!['label]))
                    .ast::<ast::Label>()?;

                (label, p.eat(Expr))
            };

            if let MaybeNode::Some(node) = node {
                return Ok(ExprState::Child(
                    ExprStep::Break { buf, label },
                    node,
                    Start::Wrapped,
                ));
            }

            let kind = expr_break_kind(cx, &mut buf, label, None)?;
            buf.end()?;
            Ok(ExprState::Kind(kind))
        }
        ExprLoop => {
            let label = loop_label(cx, &mut buf)?;
            cx.scopes.push_loop(label)?;

            let node = {
                let p = buf.stream();
                p.expect(K![loop])?;
                p.expect(Block)?
            };

            Ok(ExprState::Child(
                ExprStep::Loop {
                    buf,
                    label,
                    condition: None,
                },
                node,
                Start::Block,
            ))
        }
        ExprWhile => {
            let label = loop_label(cx, &mut buf)?;
            cx.scopes.push_loop(label)?;

            let node = {
                let p = buf.stream();
                p.expect(K![while])?;
                p.pump()?
            };

            Ok(ExprState::Child(
                ExprStep::WhileCond { buf, label },
                node,
                Start::Condition,
            ))
        }
        ExprFor => {
            let (pat, iter, block) = {
                let p = buf.stream();
                p.expect(K![for])?;
                let pat = p.expect(Pat)?;
                p.expect(K![in])?;
                let iter = p.expect(Expr)?;
                let block = p.expect(Block)?;
                (pat, iter, block)
            };

            let label = loop_label(cx, &mut buf)?;

            Ok(ExprState::Child(
                ExprStep::ForIter {
                    buf,
                    label,
                    pat,
                    block,
                },
                iter,
                Start::Wrapped,
            ))
        }
        ExprIf => {
            let (start, node) = {
                let p = buf.stream();
                let start = p.expect(K![if])?.span();
                cx.scopes.push_loop(None)?;
                (start, p.pump()?)
            };

            Ok(ExprState::Child(
                ExprStep::IfCond {
                    buf,
                    start,
                    branches: Vec::new(),
                    else_buf: None,
                },
                node,
                Start::Condition,
            ))
        }
        ExprMatch => {
            let node = {
                let p = buf.stream();
                p.expect(K![match])?;
                p.expect(Expr)?
            };

            Ok(ExprState::Child(
                ExprStep::MatchSubject { buf },
                node,
                Start::Wrapped,
            ))
        }
        ExprBinary => {
            let node = buf.stream().pump()?;

            Ok(ExprState::Child(
                ExprStep::BinaryLhs { buf },
                node,
                Start::Inner,
            ))
        }
        AsyncBlock(item) => {
            let meta = {
                let p = buf.stream();

                if cx.const_eval {
                    return Err(Error::msg(
                        &*p,
                        "async blocks are not supported in constant contexts",
                    ));
                };

                let item = cx.q.item_for("lowering async block", item).with_span(&*p)?;
                let meta = cx.lookup_meta(&*p, item.item, GenericsParameters::default())?;

                let meta::Kind::AsyncBlock { .. } = meta.kind else {
                    return Err(Error::expected_meta(
                        &*p,
                        meta.info(cx.q.pool)?,
                        "async block",
                    ));
                };

                meta
            };

            cx.scopes.push_captures()?;

            let node = {
                let p = buf.stream();
                p.expect(K!['{'])?;
                p.expect(BlockBody)?
            };

            Ok(ExprState::Child(
                ExprStep::AsyncBlock(Box::try_new(StepAsyncBlock { buf, meta })?),
                node,
                Start::Body { label: None },
            ))
        }
        ConstBlock(item) => {
            if cx.const_eval {
                let node = {
                    let p = buf.stream();
                    p.expect(K!['{'])?;
                    p.expect(BlockBody)?
                };

                return Ok(ExprState::Child(
                    ExprStep::ConstBlock { buf },
                    node,
                    Start::Body { label: None },
                ));
            }

            let kind = {
                let p = buf.stream();
                let item = cx.q.item_for("lowering const block", item).with_span(&*p)?;
                let meta = cx.lookup_meta(&*p, item.item, GenericsParameters::default())?;

                let meta::Kind::Const = meta.kind else {
                    return Err(Error::expected_meta(
                        &*p,
                        meta.info(cx.q.pool)?,
                        "constant block",
                    ));
                };

                p.ignore();
                hir::ExprKind::Const(meta.hash)
            };

            buf.end()?;
            Ok(ExprState::Kind(kind))
        }
        Closure(item) => {
            let (meta, args) = {
                let p = buf.stream();
                alloc_with!(cx, p);

                let Some(meta) = cx.q.query_meta(&*p, item, Used::default())? else {
                    return Err(Error::new(
                        &*p,
                        ErrorKind::MissingItem {
                            item: cx.q.pool.item(item).try_to_owned()?,
                        },
                    ));
                };

                let meta::Kind::Closure { .. } = meta.kind else {
                    return Err(Error::expected_meta(
                        &*p,
                        meta.info(cx.q.pool)?,
                        "a closure",
                    ));
                };

                cx.scopes.push_captures()?;

                let args = p.expect(ClosureArguments)?.parse(|p| {
                    if matches!(p.peek(), K![||]) {
                        p.pump()?;
                        return Ok(&[][..]);
                    };

                    p.expect(K![|])?;

                    let mut args = Vec::new();
                    let mut comma = Remaining::default();

                    while let MaybeNode::Some(pat) = p.eat(Pat) {
                        comma.exactly_one(cx)?;
                        let binding = pat.parse(|p| self::pat_binding(cx, p))?;
                        comma = p.remaining(cx, K![,])?;
                        args.try_push(hir::FnArg::Pat(alloc!(binding)))
                            .with_span(&*p)?;
                    }

                    comma.at_most_one(cx)?;
                    p.expect(K![|])?;
                    Ok(iter!(args))
                })?;

                (meta, args)
            };

            let node = buf.stream().expect(Expr)?;

            Ok(ExprState::Child(
                ExprStep::Closure(Box::try_new(StepClosure { buf, meta, args })?),
                node,
                Start::Wrapped,
            ))
        }
        ExprChain => {
            let label = cx.label.take();
            let node = buf.stream().pump()?;
            cx.label = label;

            if matches!(node.kind(), IndexedPath(..)) {
                let mut base = node.into_stream();
                let inner = expr_path(cx, base.stream())?;
                base.end()?;
                return chain_next(cx, buf, inner);
            }

            let span = node.span();

            Ok(ExprState::Child(
                ExprStep::ChainBase { buf },
                node,
                Start::Base { span },
            ))
        }
        ExprObject => {
            let key_node = {
                let p = buf.stream();
                let key = p.pump()?;
                p.expect(K!['{'])?;
                key
            };

            object_next(
                cx,
                buf,
                key_node,
                Vec::new(),
                Remaining::default(),
                HashMap::new(),
            )
        }
        ExprSelect => {
            {
                let p = buf.stream();
                p.expect(K![select])?;
                p.expect(K!['{'])?;
            }

            let state = SelectState {
                exprs: Vec::new(),
                branches: Vec::new(),
                default: None,
            };

            select_next(cx, buf, state, Remaining::default(), false)
        }
        Block => {
            let label = cx.label.take();

            let node = {
                let p = buf.stream();
                p.expect(K!['{'])?;
                p.expect(BlockBody)?
            };

            Ok(ExprState::Child(
                ExprStep::BlockExpr { buf },
                node,
                Start::Body { label },
            ))
        }
        _ => {
            // Not converted yet, so lower it recursively. Its own children
            // still come back through this driver.
            let kind = {
                let p = buf.stream();
                expr_inner(cx, p)?.into_kind(cx)?
            };

            buf.end()?;
            Ok(ExprState::Kind(kind))
        }
    }
}

/// Resume an expression whose child has just been lowered.
fn expr_resume<'hir, 'a>(
    cx: &mut Ctxt<'hir, '_, '_>,
    step: ExprStep<'hir, 'a>,
    value: ExprState<'hir, 'a>,
) -> Result<ExprState<'hir, 'a>> {
    match step {
        ExprStep::Wrapper { mut buf } => {
            let ExprState::Kind(kind) = value else {
                return Err(Error::msg(buf.stream().span(), "Expected an expression"));
            };

            let span = buf.stream().span();

            if let Some(label) = cx.label.take() {
                return Err(Error::msg(label, "labels are not supported for expression"));
            }

            buf.end()?;
            Ok(ExprState::Expr(hir::Expr { span, kind }))
        }
        ExprStep::Only { span } => {
            let ExprState::Kind(kind) = value else {
                return Err(Error::msg(span, "Expected an expression"));
            };

            Ok(ExprState::Expr(hir::Expr { span, kind }))
        }
        ExprStep::BlockExpr { mut buf } => {
            let ExprState::Block(block) = value else {
                return Err(Error::msg(buf.stream().span(), "Expected a block"));
            };

            let kind = {
                let p = buf.stream();
                p.expect(K!['}'])?;
                alloc_with!(cx, p);
                hir::ExprKind::Block(alloc!(block))
            };

            buf.end()?;
            Ok(ExprState::Kind(kind))
        }
        ExprStep::Stmts {
            buf,
            label,
            at,
            must_be_last,
            pending,
            ..
        } => {
            let Some(pending) = pending else {
                return Err(Error::msg(Span::empty(), "Expected a statement"));
            };

            match value {
                ExprState::Expr(expr) => {
                    let span = expr.span;
                    let id = cx.exprs.insert(expr).with_span(span)?;
                    let stmt = hir::Stmt::Expr(span, id);
                    cx.statements.try_push(stmt).with_span(pending.span)?;
                }
                ExprState::Local(local) => {
                    let local = cx.arena.alloc(local).map_err(|e| {
                        Error::new(
                            pending.span,
                            ErrorKind::ArenaAllocError {
                                requested: e.requested,
                            },
                        )
                    })?;

                    let stmt = hir::Stmt::Local(local);
                    cx.statements.try_push(stmt).with_span(pending.span)?;
                }
                _ => {
                    return Err(Error::msg(pending.span, "Expected a statement"));
                }
            }

            stmts_after(cx, buf, label, at, must_be_last, pending)
        }
        ExprStep::Local { mut buf, pat } => {
            let expr = expect_expr(&mut buf, value)?;

            let local = {
                let p = buf.stream();
                alloc_with!(cx, p);
                let expr = expr!(expr);
                let pat = pat.parse(|p| self::pat_binding(cx, p))?;

                hir::Local {
                    span: p.span(),
                    pat,
                    expr,
                }
            };

            buf.end()?;
            Ok(ExprState::Local(local))
        }
        ExprStep::Block { mut buf } => {
            let ExprState::Block(block) = value else {
                return Err(Error::msg(buf.stream().span(), "Expected a block"));
            };

            buf.stream().expect(K!['}'])?;
            buf.end()?;
            Ok(ExprState::Block(block))
        }
        ExprStep::OptExpr { mut buf, yielded } => {
            let expr = expect_expr(&mut buf, value)?;

            let kind = {
                let p = buf.stream();
                alloc_with!(cx, p);
                let expr = Some(expr!(expr));

                if yielded {
                    hir::ExprKind::Yield(expr)
                } else {
                    hir::ExprKind::Return(expr)
                }
            };

            buf.end()?;
            Ok(ExprState::Kind(kind))
        }
        ExprStep::Break { mut buf, label } => {
            let expr = expect_expr(&mut buf, value)?;
            let kind = expr_break_kind(cx, &mut buf, label, Some(expr))?;
            buf.end()?;
            Ok(ExprState::Kind(kind))
        }
        ExprStep::Loop {
            mut buf,
            label,
            condition,
        } => {
            let ExprState::Block(body) = value else {
                return Err(Error::msg(buf.stream().span(), "Expected a block"));
            };

            let kind = {
                let p = buf.stream();
                let layer = cx.scopes.pop().with_span(&*p)?;
                alloc_with!(cx, p);

                hir::ExprKind::Loop(alloc!(hir::ExprLoop {
                    label,
                    condition,
                    body,
                    drop: iter!(layer.into_drop_order()),
                }))
            };

            buf.end()?;
            Ok(ExprState::Kind(kind))
        }
        ExprStep::ForIter {
            mut buf,
            label,
            pat,
            block,
        } => {
            let expr = expect_expr(&mut buf, value)?;

            let iter = {
                let p = buf.stream();
                alloc_with!(cx, p);
                expr!(expr)
            };

            cx.scopes.push_loop(label)?;

            let binding = pat.parse(|p| self::pat_binding(cx, p))?;

            Ok(ExprState::Child(
                ExprStep::For {
                    buf,
                    label,
                    binding,
                    iter,
                },
                block,
                Start::Block,
            ))
        }
        ExprStep::For {
            mut buf,
            label,
            binding,
            iter,
        } => {
            let ExprState::Block(body) = value else {
                return Err(Error::msg(buf.stream().span(), "Expected a block"));
            };

            let kind = {
                let p = buf.stream();
                let layer = cx.scopes.pop().with_span(&*p)?;
                alloc_with!(cx, p);

                hir::ExprKind::For(alloc!(hir::ExprFor {
                    label,
                    binding,
                    iter,
                    body,
                    drop: iter!(layer.into_drop_order()),
                }))
            };

            buf.end()?;
            Ok(ExprState::Kind(kind))
        }
        ExprStep::IfBranch {
            mut buf,
            start,
            mut branches,
            condition,
            else_buf,
        } => {
            let ExprState::Block(block) = value else {
                return Err(Error::msg(buf.stream().span(), "Expected a block"));
            };

            {
                let p = buf.stream();
                let layer = cx.scopes.pop().with_span(&*p)?;
                alloc_with!(cx, p);

                branches
                    .try_push(hir::ConditionalBranch {
                        span: start.join(block.span),
                        block,
                        condition,
                        drop: iter!(layer.into_drop_order()),
                    })
                    .with_span(start)?;
            }

            if let Some(else_buf) = else_buf {
                else_buf.end()?;
            }

            if_next(cx, buf, start, branches)
        }
        ExprStep::IfElse {
            mut buf,
            branches,
            else_buf,
        } => {
            let ExprState::Block(block) = value else {
                return Err(Error::msg(buf.stream().span(), "Expected a block"));
            };

            else_buf.end()?;

            let kind = {
                let p = buf.stream();
                alloc_with!(cx, p);
                let fallback = Some(&*alloc!(block));

                hir::ExprKind::If(alloc!(hir::Conditional {
                    branches: iter!(branches),
                    fallback,
                }))
            };

            buf.end()?;
            Ok(ExprState::Kind(kind))
        }
        ExprStep::MatchSubject { mut buf } => {
            let expr = expect_expr(&mut buf, value)?;

            let subject = {
                let p = buf.stream();
                alloc_with!(cx, p);
                let subject = expr!(expr);
                p.expect(K!['{'])?;
                subject
            };

            match_next(cx, buf, subject, Vec::new(), Remaining::default(), false)
        }
        ExprStep::MatchGuard(step) => {
            let StepMatchGuard {
                buf,
                mut arm,
                subject,
                branches,
                pat,
            } = Box::into_inner(step);

            let expr = expect_expr(&mut arm, value)?;

            let (condition, node, was_block) = {
                let p = arm.stream();
                alloc_with!(cx, p);
                let condition = Some(expr!(expr));
                p.expect(K![=>])?;
                let node = p.expect(Expr)?;
                let was_block = node_is_block(&node);
                (condition, node, was_block)
            };

            Ok(ExprState::Child(
                ExprStep::MatchBody(Box::try_new(StepMatchBody {
                    buf,
                    arm,
                    subject,
                    branches,
                    pat,
                    condition,
                    was_block,
                })?),
                node,
                Start::Wrapped,
            ))
        }
        ExprStep::MatchBody(step) => {
            let StepMatchBody {
                buf,
                mut arm,
                subject,
                mut branches,
                pat,
                condition,
                was_block,
            } = Box::into_inner(step);

            let expr = expect_expr(&mut arm, value)?;

            {
                let p = arm.stream();
                let layer = cx.scopes.pop().with_span(&*p)?;
                alloc_with!(cx, p);

                branches
                    .try_push(hir::ExprMatchBranch {
                        span: p.span(),
                        pat,
                        condition,
                        body: expr!(expr),
                        drop: iter!(layer.into_drop_order()),
                    })
                    .with_span(&*p)?;
            }

            arm.end()?;

            let mut buf = buf;
            let comma = buf.stream().remaining(cx, K![,])?;
            match_next(cx, buf, subject, branches, comma, was_block)
        }
        ExprStep::BinaryLhs { mut buf } => {
            let lhs = expect_expr(&mut buf, value)?;
            binary_next(cx, buf, lhs)
        }
        ExprStep::BinaryRhs {
            mut buf,
            lhs,
            op,
            needs,
        } => {
            let rhs = expect_expr(&mut buf, value)?;
            cx.needs = needs;

            let lhs = {
                let p = buf.stream();
                alloc_with!(cx, p);

                let span = lhs.span.join(rhs.span);

                let kind = hir::ExprKind::Binary(alloc!(hir::ExprBinary {
                    lhs: expr!(lhs),
                    op,
                    rhs: expr!(rhs),
                }));

                hir::Expr { span, kind }
            };

            binary_next(cx, buf, lhs)
        }
        ExprStep::AsyncBlock(step) => {
            let StepAsyncBlock { mut buf, meta } = Box::into_inner(step);

            let ExprState::Block(block) = value else {
                return Err(Error::msg(buf.stream().span(), "Expected a block"));
            };

            let meta::Kind::AsyncBlock { call, do_move, .. } = meta.kind else {
                return Err(Error::msg(buf.stream().span(), "Expected an async block"));
            };

            let kind = {
                let p = buf.stream();
                p.expect(K!['}'])?;

                let layer = cx.scopes.pop().with_span(&*p)?;
                cx.q.set_used(&meta.item_meta)?;

                alloc_with!(cx, p);

                let block = &*alloc!(block);
                let captures = &*iter!(layer.captures().map(|(_, id)| id));

                let Some(queue) = cx.secondary_builds.as_mut() else {
                    return Err(Error::new(&*p, ErrorKind::AsyncBlockInConst));
                };

                queue.try_push(query::SecondaryBuildEntry {
                    item_meta: meta.item_meta,
                    build: query::SecondaryBuild::AsyncBlock(query::AsyncBlock {
                        hir: alloc!(hir::AsyncBlock { block, captures }),
                        call,
                    }),
                })?;

                hir::ExprKind::AsyncBlock(alloc!(hir::ExprAsyncBlock {
                    hash: meta.hash,
                    do_move,
                    captures,
                }))
            };

            buf.end()?;
            Ok(ExprState::Kind(kind))
        }
        ExprStep::ConstBlock { mut buf } => {
            let ExprState::Block(block) = value else {
                return Err(Error::msg(buf.stream().span(), "Expected a block"));
            };

            let kind = {
                let p = buf.stream();
                p.expect(K!['}'])?;
                alloc_with!(cx, p);
                hir::ExprKind::Block(alloc!(block))
            };

            buf.end()?;
            Ok(ExprState::Kind(kind))
        }
        ExprStep::Closure(step) => {
            let StepClosure {
                mut buf,
                meta,
                args,
            } = Box::into_inner(step);

            let body = expect_expr(&mut buf, value)?;

            let meta::Kind::Closure { call, do_move, .. } = meta.kind else {
                return Err(Error::msg(buf.stream().span(), "Expected a closure"));
            };

            let kind = {
                let p = buf.stream();
                alloc_with!(cx, p);

                let body_span = body.span;
                let body = expr!(body);

                let layer = cx.scopes.pop().with_span(&*p)?;
                cx.q.set_used(&meta.item_meta)?;

                let captures = &*iter!(layer.captures().map(|(_, id)| id));

                let Some(queue) = cx.secondary_builds.as_mut() else {
                    return Err(Error::new(&*p, ErrorKind::ClosureInConst));
                };

                queue.try_push(query::SecondaryBuildEntry {
                    item_meta: meta.item_meta,
                    build: query::SecondaryBuild::Closure(query::Closure {
                        hir: alloc!(hir::ExprClosure {
                            span: body_span,
                            args,
                            body,
                            captures,
                        }),
                        call,
                    }),
                })?;

                if captures.is_empty() {
                    hir::ExprKind::Fn(meta.hash)
                } else {
                    hir::ExprKind::CallClosure(alloc!(hir::ExprCallClosure {
                        hash: meta.hash,
                        do_move,
                        captures,
                    }))
                }
            };

            buf.end()?;
            Ok(ExprState::Kind(kind))
        }
        ExprStep::ChainBase { buf } => {
            let ExprState::Inner(inner) = value else {
                return Err(Error::msg(Span::empty(), "Expected a chain base"));
            };

            chain_next(cx, buf, inner)
        }
        ExprStep::BaseInner { span } => {
            let ExprState::Kind(kind) = value else {
                return Err(Error::msg(span, "Expected an expression"));
            };

            Ok(ExprState::Inner(chain_inner(span, kind)))
        }
        ExprStep::ChainCall(step) => {
            let StepChainCall {
                buf,
                call,
                inner,
                mut args,
                comma,
                start,
            } = Box::into_inner(step);

            let mut call = call;
            let expr = expect_expr(&mut call, value)?;
            comma.exactly_one(cx)?;
            args.try_push(expr).with_span(start)?;
            let comma = call.stream().one(K![,]);

            match chain_call_next(cx, buf, call, inner, args, comma, start)? {
                ChainOutcome::State(state) => Ok(state),
                ChainOutcome::Continue(buf, inner) => chain_next(cx, buf, inner),
            }
        }
        ExprStep::ChainIndex(step) => {
            let StepChainIndex {
                buf,
                index,
                inner,
                start,
            } = Box::into_inner(step);

            let mut index = index;
            let expr = expect_expr(&mut index, value)?;

            let kind = {
                let p = index.stream();
                p.expect(K![']'])?;

                let span = inner.span;
                let target_kind = inner.into_kind(cx)?;
                alloc_with!(cx, p);

                hir::ExprKind::Index(alloc!(hir::ExprIndex {
                    target: expr!(hir::Expr {
                        span,
                        kind: target_kind
                    }),
                    index: expr!(expr),
                }))
            };

            index.end()?;
            chain_next(cx, buf, chain_inner(start, kind))
        }
        ExprStep::ObjectValue {
            mut buf,
            key_node,
            mut assignments,
            mut keys_dup,
            key: (key_span, key),
        } => {
            let assign = expect_expr(&mut buf, value)?;

            if let Some(_existing) = keys_dup.try_insert(key, key_span)? {
                return Err(Error::new(
                    key_span,
                    ErrorKind::DuplicateObjectKey {
                        #[cfg(feature = "emit")]
                        existing: _existing.span(),
                        #[cfg(feature = "emit")]
                        object: buf.stream().span(),
                    },
                ));
            }

            let comma = {
                let p = buf.stream();
                alloc_with!(cx, p);

                assignments
                    .try_push(hir::FieldAssign {
                        key: (key_span, key),
                        assign: expr!(assign),
                        position: None,
                    })
                    .with_span(key_span)?;

                p.one(K![,])
            };

            object_next(cx, buf, key_node, assignments, comma, keys_dup)
        }
        ExprStep::SelectDefault(step) => {
            let StepSelectDefault {
                buf,
                mut arm,
                mut state,
                default_span,
                was_block,
            } = Box::into_inner(step);

            let body = expect_expr(&mut arm, value)?;

            {
                let p = arm.stream();
                alloc_with!(cx, p);

                if let Some((existing, _)) = state.default {
                    cx.error(Error::new(
                        default_span,
                        ErrorKind::DuplicateSelectDefault { existing },
                    ))?;
                } else {
                    state.default = Some((body.span, expr!(body)));
                }
            }

            arm.end()?;

            let mut buf = buf;
            let comma = buf.stream().remaining(cx, K![,])?;
            select_next(cx, buf, state, comma, was_block)
        }
        ExprStep::SelectValue(step) => {
            let StepSelectValue {
                buf,
                mut arm,
                mut state,
                pat,
            } = Box::into_inner(step);

            let expr = expect_expr(&mut arm, value)?;
            state.exprs.try_push(expr).with_span(arm.stream().span())?;

            let (node, was_block) = {
                let p = arm.stream();
                p.expect(K![=>])?;
                let node = p.expect(Expr)?;
                let was_block = node_is_block(&node);
                (node, was_block)
            };

            Ok(ExprState::Child(
                ExprStep::SelectBody(Box::try_new(StepSelectBody {
                    buf,
                    arm,
                    state,
                    pat,
                    was_block,
                })?),
                node,
                Start::Wrapped,
            ))
        }
        ExprStep::SelectBody(step) => {
            let StepSelectBody {
                buf,
                mut arm,
                mut state,
                pat,
                was_block,
            } = Box::into_inner(step);

            let body = expect_expr(&mut arm, value)?;

            {
                let p = arm.stream();
                let layer = cx.scopes.pop().with_span(&*p)?;
                alloc_with!(cx, p);

                state
                    .branches
                    .try_push(hir::ExprSelectBranch {
                        pat,
                        body: expr!(body),
                        drop: iter!(layer.into_drop_order()),
                    })
                    .with_span(&*p)?;
            }

            arm.end()?;

            let mut buf = buf;
            let comma = buf.stream().remaining(cx, K![,])?;
            select_next(cx, buf, state, comma, was_block)
        }
        ExprStep::CondLet { mut buf, pat } => {
            let expr = expect_expr(&mut buf, value)?;

            let condition = {
                let p = buf.stream();
                alloc_with!(cx, p);
                let expr = expr!(expr);
                let pat = pat.parse(|p| self::pat_binding(cx, p))?;
                hir::Condition::ExprLet(alloc!(hir::ExprLet { pat, expr }))
            };

            buf.end()?;
            Ok(ExprState::Condition(condition))
        }
        ExprStep::CondExpr => {
            let ExprState::Expr(expr) = value else {
                return Err(Error::msg(Span::empty(), "Expected a condition"));
            };

            let span = expr.span;
            let id = cx.exprs.insert(expr).with_span(span)?;
            Ok(ExprState::Condition(hir::Condition::Expr(span, id)))
        }
        ExprStep::WhileCond { mut buf, label } => {
            let ExprState::Condition(condition) = value else {
                return Err(Error::msg(buf.stream().span(), "Expected a condition"));
            };

            let (condition, node) = {
                let p = buf.stream();
                alloc_with!(cx, p);
                let condition = Some(&*alloc!(condition));
                (condition, p.expect(Block)?)
            };

            Ok(ExprState::Child(
                ExprStep::Loop {
                    buf,
                    label,
                    condition,
                },
                node,
                Start::Block,
            ))
        }
        ExprStep::IfCond {
            mut buf,
            start,
            branches,
            else_buf,
        } => {
            let ExprState::Condition(condition) = value else {
                return Err(Error::msg(buf.stream().span(), "Expected a condition"));
            };

            let condition = {
                let p = buf.stream();
                alloc_with!(cx, p);
                &*alloc!(condition)
            };

            let node = match &else_buf {
                Some(_) => None,
                None => Some(buf.stream().expect(Block)?),
            };

            let (node, else_buf) = match node {
                Some(node) => (node, else_buf),
                None => {
                    let mut else_buf = else_buf.expect("checked above");
                    let node = else_buf.stream().expect(Block)?;
                    (node, Some(else_buf))
                }
            };

            Ok(ExprState::Child(
                ExprStep::IfBranch {
                    buf,
                    start,
                    branches,
                    condition,
                    else_buf,
                },
                node,
                Start::Block,
            ))
        }
        ExprStep::Group { mut buf, empty } => {
            let expr = expect_expr(&mut buf, value)?;

            let kind = {
                let p = buf.stream();
                alloc_with!(cx, p);

                if empty {
                    p.expect(Kind::Close(Delimiter::Empty))?;
                } else {
                    p.expect(K![')'])?;
                }

                hir::ExprKind::Group(expr!(expr))
            };

            buf.end()?;
            Ok(ExprState::Kind(kind))
        }
        ExprStep::Unary { mut buf, op } => {
            let expr = expect_expr(&mut buf, value)?;

            let kind = {
                let p = buf.stream();
                alloc_with!(cx, p);
                let expr = expr!(expr);
                hir::ExprKind::Unary(alloc!(hir::ExprUnary { op, expr }))
            };

            buf.end()?;
            Ok(ExprState::Kind(kind))
        }
        ExprStep::AssignRhs { mut buf } => {
            let expr = expect_expr(&mut buf, value)?;

            let (lhs, node) = {
                let p = buf.stream();
                alloc_with!(cx, p);
                let lhs = expr!(expr);
                p.expect(K![=])?;
                (lhs, p.expect(Expr)?)
            };

            Ok(ExprState::Child(
                ExprStep::Assign { buf, lhs },
                node,
                Start::Wrapped,
            ))
        }
        ExprStep::Assign { mut buf, lhs } => {
            let expr = expect_expr(&mut buf, value)?;

            let kind = {
                let p = buf.stream();
                alloc_with!(cx, p);
                let rhs = expr!(expr);
                hir::ExprKind::Assign(alloc!(hir::ExprAssign { lhs, rhs }))
            };

            buf.end()?;
            Ok(ExprState::Kind(kind))
        }
        ExprStep::Seq {
            mut buf,
            mut items,
            comma,
            array,
        } => {
            let expr = expect_expr(&mut buf, value)?;
            comma.exactly_one(cx)?;
            items.try_push(expr).with_span(buf.stream().span())?;
            let comma = buf.stream().one(K![,]);
            expr_seq_next(cx, buf, items, comma, array)
        }
        ExprStep::Range1 { mut buf, kind } => {
            let expr = expect_expr(&mut buf, value)?;

            let kind = {
                let p = buf.stream();

                match kind {
                    RangeKind::From => {
                        alloc_with!(cx, p);
                        let start = expr!(expr);
                        hir::ExprKind::Range(alloc!(hir::ExprRange::RangeFrom { start }))
                    }
                    RangeKind::To => {
                        alloc_with!(cx, p);
                        let end = expr!(expr);
                        hir::ExprKind::Range(alloc!(hir::ExprRange::RangeTo { end }))
                    }
                    RangeKind::ToInclusive => {
                        alloc_with!(cx, p);
                        let end = expr!(expr);
                        hir::ExprKind::Range(alloc!(hir::ExprRange::RangeToInclusive { end }))
                    }
                }
            };

            buf.end()?;
            Ok(ExprState::Kind(kind))
        }
        ExprStep::RangeEnd {
            mut buf,
            start,
            inclusive,
        } => {
            let expr = expect_expr(&mut buf, value)?;

            let Some(start) = start else {
                let (start, node) = {
                    let p = buf.stream();
                    alloc_with!(cx, p);
                    let start = expr!(expr);

                    if inclusive {
                        p.expect(K![..=])?;
                    } else {
                        p.expect(K![..])?;
                    }

                    (start, p.pump()?)
                };

                return Ok(ExprState::Child(
                    ExprStep::RangeEnd {
                        buf,
                        start: Some(start),
                        inclusive,
                    },
                    node,
                    Start::Inner,
                ));
            };

            let kind = {
                let p = buf.stream();
                alloc_with!(cx, p);
                let end = expr!(expr);

                if inclusive {
                    hir::ExprKind::Range(alloc!(hir::ExprRange::RangeInclusive { start, end }))
                } else {
                    hir::ExprKind::Range(alloc!(hir::ExprRange::Range { start, end }))
                }
            };

            buf.end()?;
            Ok(ExprState::Kind(kind))
        }
    }
}

/// Lower the next item of a sequence expression, or finish it.
fn expr_seq_next<'hir, 'a>(
    cx: &mut Ctxt<'hir, '_, '_>,
    mut buf: StreamBuf<'a>,
    items: Vec<hir::Expr<'hir>>,
    comma: Remaining<'a>,
    array: bool,
) -> Result<ExprState<'hir, 'a>> {
    if let MaybeNode::Some(node) = buf.stream().eat(Expr) {
        return Ok(ExprState::Child(
            ExprStep::Seq {
                buf,
                items,
                comma,
                array,
            },
            node,
            Start::Wrapped,
        ));
    }

    if array {
        comma.at_most_one(cx)?;
    } else if items.len() <= 1 {
        comma.exactly_one(cx)?;
    } else {
        comma.at_most_one(cx)?;
    }

    let kind = {
        let p = buf.stream();
        p.expect(if array { K![']'] } else { K![')'] })?;
        alloc_with!(cx, p);

        let seq = alloc!(hir::ExprSeq {
            items: iter!(items, |e| expr!(e))
        });

        if array {
            hir::ExprKind::Vec(seq)
        } else {
            hir::ExprKind::Tuple(seq)
        }
    };

    buf.end()?;
    Ok(ExprState::Kind(kind))
}

/// The statement whose child is currently being lowered.
#[derive(Debug, Clone, Copy)]
struct Pending {
    needs_semi: bool,
    class: NodeClass,
    span: Span,
}

/// Start lowering the statements of a block body.
fn stmts_start<'hir, 'a>(
    cx: &mut Ctxt<'hir, '_, '_>,
    mut buf: StreamBuf<'a>,
    label: Option<ast::Label>,
) -> Result<ExprState<'hir, 'a>> {
    let label = {
        let p = buf.stream();
        alloc_with!(cx, p);

        match label {
            Some(label) => Some(alloc_str!(label.resolve(resolve_context!(cx.q))?)),
            None => None,
        }
    };

    cx.scopes.push(label)?;

    let at = cx.statements.len();

    stmts_next(cx, buf, label, at, None, true)
}

/// Lower the next statement of a block body, or finish it.
fn stmts_next<'hir, 'a>(
    cx: &mut Ctxt<'hir, '_, '_>,
    mut buf: StreamBuf<'a>,
    label: Option<&'hir str>,
    at: usize,
    must_be_last: Option<Span>,
    mut last_item: bool,
) -> Result<ExprState<'hir, 'a>> {
    while let Some(node) = buf.stream().next() {
        let (needs_semi, class) = classify(&node);
        let span = node.span();

        let pending = Some(Pending {
            needs_semi,
            class,
            span,
        });

        match node.kind() {
            Local => {
                return Ok(ExprState::Child(
                    ExprStep::Stmts {
                        buf,
                        label,
                        at,
                        must_be_last,
                        pending,
                    },
                    node,
                    Start::Local,
                ));
            }
            Expr => {
                return Ok(ExprState::Child(
                    ExprStep::Stmts {
                        buf,
                        label,
                        at,
                        must_be_last,
                        pending,
                    },
                    node,
                    Start::Wrapped,
                ));
            }
            Item => {
                let semi = buf.stream().remaining(cx, K![;])?;

                if needs_semi {
                    semi.exactly_one(cx)?;
                } else {
                    semi.at_most_one(cx)?;
                }

                last_item = true;
                continue;
            }
            _ => {
                cx.error(node.expected("an expression or local"))?;
                continue;
            }
        }
    }

    let block = {
        let p = buf.stream();
        alloc_with!(cx, p);

        let value = 'out: {
            if last_item {
                break 'out None;
            }

            debug_assert!(
                at < cx.statements.len(),
                "starting point for assertions must be prior to buffer size"
            );

            match cx.statements.pop() {
                Some(hir::Stmt::Expr(_, e)) => Some(e),
                Some(stmt) => {
                    cx.statements.try_push(stmt).with_span(&*p)?;
                    None
                }
                None => None,
            }
        };

        let statements = iter!(cx.statements.drain(at..));
        let layer = cx.scopes.pop().with_span(&*p)?;

        hir::Block {
            span: p.span(),
            label,
            statements,
            value,
            drop: iter!(layer.into_drop_order()),
        }
    };

    buf.end()?;
    Ok(ExprState::Block(block))
}

/// Apply the statement separator rules once a statement has been lowered.
fn stmts_after<'hir, 'a>(
    cx: &mut Ctxt<'hir, '_, '_>,
    mut buf: StreamBuf<'a>,
    label: Option<&'hir str>,
    at: usize,
    mut must_be_last: Option<Span>,
    pending: Pending,
) -> Result<ExprState<'hir, 'a>> {
    let Pending {
        needs_semi,
        class,
        span,
    } = pending;

    let semis = buf.stream().remaining(cx, K![;])?;

    let last_item = semis.is_present();

    if let Some(span) = must_be_last {
        cx.error(Error::new(
            span,
            ErrorKind::ExpectedBlockSemiColon {
                #[cfg(feature = "emit")]
                followed_span: span,
            },
        ))?;
    }

    if matches!(class, NodeClass::Expr) && semis.is_absent() {
        must_be_last = Some(span);
    }

    if let Some(span) = semis.trailing() {
        cx.error(Error::msg(span, "unused semi-colons"))?;
    }

    if needs_semi {
        semis.at_least_one(cx)?;
    } else {
        semis.at_most_one(cx)?;
    }

    stmts_next(cx, buf, label, at, must_be_last, last_item)
}

/// Resolve the label which applies to a loop.
fn loop_label<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    buf: &mut StreamBuf<'_>,
) -> Result<Option<&'hir str>> {
    let p = buf.stream();
    alloc_with!(cx, p);

    match cx.label.take() {
        Some(label) => Ok(Some(alloc_str!(label.resolve(resolve_context!(cx.q))?))),
        None => Ok(None),
    }
}

/// Build the kind of a `break`, which needs the drop order of the loop it
/// breaks out of.
fn expr_break_kind<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    buf: &mut StreamBuf<'_>,
    label: Option<ast::Label>,
    expr: Option<hir::Expr<'hir>>,
) -> Result<hir::ExprKind<'hir>> {
    let p = buf.stream();
    alloc_with!(cx, p);

    let label = match label {
        Some(label) => Some(label.resolve(resolve_context!(cx.q))?),
        None => None,
    };

    let Some(drop) = cx.scopes.loop_drop(label)? else {
        if let Some(label) = label {
            return Err(Error::new(
                &*p,
                ErrorKind::MissingLabel {
                    label: label.try_into()?,
                },
            ));
        } else {
            return Err(Error::new(&*p, ErrorKind::BreakUnsupported));
        }
    };

    Ok(hir::ExprKind::Break(alloc!(hir::ExprBreak {
        label: match label {
            Some(label) => Some(alloc_str!(label)),
            None => None,
        },
        expr: match expr {
            Some(expr) => Some(expr!(expr)),
            None => None,
        },
        drop: iter!(drop),
    })))
}

/// Continue the `else` chain of an `if`, or finish it.
fn if_next<'hir, 'a>(
    cx: &mut Ctxt<'hir, '_, '_>,
    mut buf: StreamBuf<'a>,
    start: Span,
    branches: Vec<hir::ConditionalBranch<'hir>>,
) -> Result<ExprState<'hir, 'a>> {
    match buf.stream().peek() {
        ExprElse => {
            let mut else_buf = buf.stream().pump()?.into_stream();

            let node = {
                let p = else_buf.stream();
                p.expect(K![else])?;
                p.expect(Block)?
            };

            Ok(ExprState::Child(
                ExprStep::IfElse {
                    buf,
                    branches,
                    else_buf,
                },
                node,
                Start::Block,
            ))
        }
        ExprElseIf => {
            let mut else_buf = buf.stream().pump()?.into_stream();

            let node = {
                let p = else_buf.stream();
                p.expect(K![else])?;
                p.expect(K![if])?;
                cx.scopes.push_loop(None)?;
                p.pump()?
            };

            Ok(ExprState::Child(
                ExprStep::IfCond {
                    buf,
                    start,
                    branches,
                    else_buf: Some(else_buf),
                },
                node,
                Start::Condition,
            ))
        }
        _ => {
            let kind = {
                let p = buf.stream();
                alloc_with!(cx, p);

                hir::ExprKind::If(alloc!(hir::Conditional {
                    branches: iter!(branches),
                    fallback: None,
                }))
            };

            buf.end()?;
            Ok(ExprState::Kind(kind))
        }
    }
}

/// Test if the expression in the given node is a block, which decides whether
/// a match arm needs a trailing comma.
fn node_is_block(node: &Node<'_>) -> bool {
    let Some(node) = node.children().find(|n| !n.is_whitespace()) else {
        return false;
    };

    matches!(node.kind(), Block)
}

/// Lower the next arm of a `match`, or finish it.
fn match_next<'hir, 'a>(
    cx: &mut Ctxt<'hir, '_, '_>,
    mut buf: StreamBuf<'a>,
    subject: hir::ExprId,
    branches: Vec<hir::ExprMatchBranch<'hir>>,
    comma: Remaining<'a>,
    was_block: bool,
) -> Result<ExprState<'hir, 'a>> {
    if let MaybeNode::Some(node) = buf.stream().eat(ExprMatchArm) {
        if was_block {
            comma.at_most_one(cx)?;
        } else {
            comma.exactly_one(cx)?;
        }

        let mut arm = node.into_stream();

        cx.scopes.push(None)?;

        let pat = arm.stream().expect(Pat)?.parse(|p| pat_binding(cx, p))?;

        if arm.stream().eat(K![if]).is_some() {
            let node = arm.stream().expect(Expr)?;

            return Ok(ExprState::Child(
                ExprStep::MatchGuard(Box::try_new(StepMatchGuard {
                    buf,
                    arm,
                    subject,
                    branches,
                    pat,
                })?),
                node,
                Start::Wrapped,
            ));
        }

        let (node, was_block) = {
            let p = arm.stream();
            p.expect(K![=>])?;
            let node = p.expect(Expr)?;
            let was_block = node_is_block(&node);
            (node, was_block)
        };

        return Ok(ExprState::Child(
            ExprStep::MatchBody(Box::try_new(StepMatchBody {
                buf,
                arm,
                subject,
                branches,
                pat,
                condition: None,
                was_block,
            })?),
            node,
            Start::Wrapped,
        ));
    }

    comma.at_most_one(cx)?;

    let kind = {
        let p = buf.stream();
        p.expect(K!['}'])?;
        alloc_with!(cx, p);

        hir::ExprKind::Match(alloc!(hir::ExprMatch {
            expr: subject,
            branches: iter!(branches),
        }))
    };

    buf.end()?;
    Ok(ExprState::Kind(kind))
}

/// Lower the next operand of a binary chain, or finish it.
///
/// The chain itself is flat in the source, so this loops rather than nesting -
/// only the operands are children.
fn binary_next<'hir, 'a>(
    cx: &mut Ctxt<'hir, '_, '_>,
    mut buf: StreamBuf<'a>,
    lhs: hir::Expr<'hir>,
) -> Result<ExprState<'hir, 'a>> {
    if buf.stream().is_eof() {
        buf.end()?;
        return Ok(ExprState::Kind(lhs.kind));
    }

    let (op, node) = {
        let p = buf.stream();
        let node = p.expect(ExprOperator)?;

        let Some(op) = node
            .tokens::<2>()
            .as_deref()
            .and_then(ast::BinOp::from_slice)
        else {
            return Err(node.expected("valid operator"));
        };

        (op, p.pump()?)
    };

    let rhs_needs = match op {
        ast::BinOp::As(..) | ast::BinOp::Is(..) | ast::BinOp::IsNot(..) => Needs::Type,
        _ => Needs::Value,
    };

    let needs = replace(&mut cx.needs, rhs_needs);

    Ok(ExprState::Child(
        ExprStep::BinaryRhs {
            buf,
            lhs,
            op,
            needs,
        },
        node,
        Start::Inner,
    ))
}

/// What lowering a chain element produced.
enum ChainOutcome<'hir, 'a> {
    /// The chain is suspended or complete.
    State(ExprState<'hir, 'a>),
    /// Continue the chain with the given base.
    Continue(StreamBuf<'a>, ExprInner<'hir, 'a>),
}

/// Lower the elements of a chain, or finish it.
///
/// A chain is flat in the source, so this loops over its elements rather than
/// nesting - only the arguments of a call and the index of an index are
/// children.
fn chain_next<'hir, 'a>(
    cx: &mut Ctxt<'hir, '_, '_>,
    mut buf: StreamBuf<'a>,
    mut inner: ExprInner<'hir, 'a>,
) -> Result<ExprState<'hir, 'a>> {
    loop {
        let start = inner.span;

        let Some(node) = buf.stream().next() else {
            let kind = inner.into_kind(cx)?;
            buf.end()?;
            return Ok(ExprState::Kind(kind));
        };

        let span = start.join(node.span());

        match node.kind() {
            ExprCall => {
                let mut call = node.into_stream();
                call.stream().expect(K!['('])?;

                match chain_call_next(cx, buf, call, inner, Vec::new(), Remaining::default(), span)?
                {
                    ChainOutcome::State(state) => return Ok(state),
                    ChainOutcome::Continue(next_buf, next_inner) => {
                        buf = next_buf;
                        inner = next_inner;
                    }
                }
            }
            ExprIndex => {
                let mut index = node.into_stream();

                let node = {
                    let p = index.stream();
                    p.expect(K!['['])?;
                    p.expect(Expr)?
                };

                return Ok(ExprState::Child(
                    ExprStep::ChainIndex(Box::try_new(StepChainIndex {
                        buf,
                        index,
                        inner,
                        start: span,
                    })?),
                    node,
                    Start::Wrapped,
                ));
            }
            ExprField => {
                let kind = node.parse(|p| expr_field(cx, p, inner))?;
                inner = chain_inner(span, kind);
            }
            ExprAwait => {
                let kind = node.parse(|p| expr_await(cx, p, inner))?;
                inner = chain_inner(span, kind);
            }
            ExprTry => {
                let kind = node.parse(|p| expr_try(cx, p, inner))?;
                inner = chain_inner(span, kind);
            }
            _ => return Err(node.expected(ExprChain)),
        }
    }
}

/// Lower the next argument of a call in a chain, or finish the call.
fn chain_call_next<'hir, 'a>(
    cx: &mut Ctxt<'hir, '_, '_>,
    buf: StreamBuf<'a>,
    mut call: StreamBuf<'a>,
    inner: ExprInner<'hir, 'a>,
    args: Vec<hir::Expr<'hir>>,
    comma: Remaining<'a>,
    start: Span,
) -> Result<ChainOutcome<'hir, 'a>> {
    if let MaybeNode::Some(node) = call.stream().eat(Expr) {
        return Ok(ChainOutcome::State(ExprState::Child(
            ExprStep::ChainCall(Box::try_new(StepChainCall {
                buf,
                call,
                inner,
                args,
                comma,
                start,
            })?),
            node,
            Start::Wrapped,
        )));
    }

    comma.at_most_one(cx)?;

    let kind = {
        let p = call.stream();
        p.expect(K![')'])?;
        let call = inner.into_call(cx, args.len())?;
        alloc_with!(cx, p);

        hir::ExprKind::Call(alloc!(hir::ExprCall {
            call,
            args: iter!(args, |e| expr!(e)),
        }))
    };

    call.end()?;
    Ok(ChainOutcome::Continue(buf, chain_inner(start, kind)))
}

/// Wrap a chain element's kind so that it becomes the base of the next one.
fn chain_inner<'hir, 'a>(span: Span, kind: hir::ExprKind<'hir>) -> ExprInner<'hir, 'a> {
    ExprInner {
        span,
        kind: ExprInnerKind::Kind(kind),
    }
}

/// Lower the next field of an object, or finish it.
fn object_next<'hir, 'a>(
    cx: &mut Ctxt<'hir, '_, '_>,
    mut buf: StreamBuf<'a>,
    key_node: Node<'a>,
    mut assignments: Vec<hir::FieldAssign<'hir>>,
    mut comma: Remaining<'a>,
    mut keys_dup: HashMap<&'hir str, Span>,
) -> Result<ExprState<'hir, 'a>> {
    while matches!(buf.stream().peek(), object_key!()) {
        comma.exactly_one(cx)?;

        let (key_span, key) = {
            let p = buf.stream();
            alloc_with!(cx, p);

            match p.peek() {
                K![str] => {
                    let lit = p.ast::<ast::LitStr>()?;
                    let string = lit.resolve(resolve_context!(cx.q))?;
                    (lit.span(), alloc_str!(string.as_ref()))
                }
                K![ident] => {
                    let ident = p.ast::<ast::Ident>()?;
                    let string = ident.resolve(resolve_context!(cx.q))?;
                    (ident.span(), alloc_str!(string))
                }
                _ => {
                    return Err(p.expected("object key"));
                }
            }
        };

        if buf.stream().eat(K![:]).is_some() {
            let node = buf.stream().expect(Expr)?;

            return Ok(ExprState::Child(
                ExprStep::ObjectValue {
                    buf,
                    key_node,
                    assignments,
                    keys_dup,
                    key: (key_span, key),
                },
                node,
                Start::Wrapped,
            ));
        }

        let assign = {
            let Some((name, _)) = cx.scopes.get(hir::Name::Str(key))? else {
                return Err(Error::new(
                    key_span,
                    ErrorKind::MissingLocal {
                        name: key.try_to_string()?.try_into()?,
                    },
                ));
            };

            hir::Expr {
                span: key_span,
                kind: hir::ExprKind::Variable(name),
            }
        };

        if let Some(_existing) = keys_dup.try_insert(key, key_span)? {
            return Err(Error::new(
                key_span,
                ErrorKind::DuplicateObjectKey {
                    #[cfg(feature = "emit")]
                    existing: _existing.span(),
                    #[cfg(feature = "emit")]
                    object: buf.stream().span(),
                },
            ));
        }

        {
            let p = buf.stream();
            alloc_with!(cx, p);

            assignments
                .try_push(hir::FieldAssign {
                    key: (key_span, key),
                    assign: expr!(assign),
                    position: None,
                })
                .with_span(key_span)?;
        }

        comma = buf.stream().one(K![,]);
    }

    comma.at_most_one(cx)?;
    buf.stream().expect(K!['}'])?;

    let kind = object_kind(cx, &mut buf, &key_node, &mut assignments)?;

    let kind = {
        let p = buf.stream();
        alloc_with!(cx, p);

        hir::ExprKind::Object(alloc!(hir::ExprObject {
            kind,
            assignments: iter!(assignments),
        }))
    };

    buf.end()?;
    Ok(ExprState::Kind(kind))
}

/// Resolve the kind of an object literal from the key which precedes it.
fn object_kind<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    buf: &mut StreamBuf<'_>,
    key_node: &Node<'_>,
    assignments: &mut [hir::FieldAssign<'hir>],
) -> Result<hir::ExprObjectKind> {
    fn check_object_fields(
        span: Span,
        assignments: &mut [hir::FieldAssign<'_>],
        fields: &[meta::FieldMeta],
        item: &crate::Item,
    ) -> Result<()> {
        let mut named = HashMap::new();

        for f in fields {
            named.try_insert(f.name.as_ref(), f)?;
        }

        for assign in assignments.iter_mut() {
            let Some(meta) = named.remove(assign.key.1) else {
                return Err(Error::new(
                    assign.key.0,
                    ErrorKind::LitObjectNotField {
                        field: assign.key.1.try_into()?,
                        item: item.try_to_owned()?,
                    },
                ));
            };

            assign.position = Some(meta.position);
        }

        if let Some(field) = named.into_keys().next() {
            return Err(Error::new(
                span,
                ErrorKind::LitObjectMissingField {
                    field: field.try_into()?,
                    item: item.try_to_owned()?,
                },
            ));
        }

        Ok(())
    }

    match key_node.kind() {
        AnonymousObjectKey => Ok(hir::ExprObjectKind::Anonymous),
        IndexedPath(..) => {
            let (named, span) = key_node
                .clone()
                .parse(|p| Ok((cx.q.convert_path2(p)?, p.span())))?;

            let parameters = generics_parameters(cx, &named)?;
            let meta = cx.lookup_meta(&span, named.item, parameters)?;
            let item = cx.q.pool.item(meta.item_meta.item);

            match &meta.kind {
                meta::Kind::Struct {
                    fields: meta::Fields::Empty,
                    constructor,
                    ..
                } => {
                    check_object_fields(span, assignments, &[], item)?;

                    Ok(match constructor {
                        Some(_) => hir::ExprObjectKind::ExternalType {
                            hash: meta.hash,
                            args: 0,
                        },
                        None => hir::ExprObjectKind::Struct { hash: meta.hash },
                    })
                }
                meta::Kind::Struct {
                    fields: meta::Fields::Named(st),
                    constructor,
                    ..
                } => {
                    check_object_fields(span, assignments, &st.fields, item)?;

                    Ok(match constructor {
                        Some(_) => hir::ExprObjectKind::ExternalType {
                            hash: meta.hash,
                            args: st.fields.len(),
                        },
                        None => hir::ExprObjectKind::Struct { hash: meta.hash },
                    })
                }
                _ => Err(Error::new(
                    span,
                    ErrorKind::UnsupportedLitObject {
                        meta: meta.info(cx.q.pool)?,
                    },
                )),
            }
        }
        _ => Err(buf.stream().expected("object key")),
    }
}

/// Lower the next arm of a `select`, or finish it.
fn select_next<'hir, 'a>(
    cx: &mut Ctxt<'hir, '_, '_>,
    mut buf: StreamBuf<'a>,
    state: SelectState<'hir>,
    comma: Remaining<'a>,
    was_block: bool,
) -> Result<ExprState<'hir, 'a>> {
    if let MaybeNode::Some(node) = buf.stream().eat(ExprSelectArm) {
        if was_block {
            comma.at_most_one(cx)?;
        } else {
            comma.exactly_one(cx)?;
        }

        let mut arm = node.into_stream();

        cx.scopes.push(None)?;

        match arm.stream().peek() {
            K![default] => {
                let (default_span, node, was_block) = {
                    let p = arm.stream();
                    let default_token = p.expect(K![default])?;
                    p.expect(K![=>])?;
                    let node = p.expect(Expr)?;
                    let was_block = node_is_block(&node);
                    (default_token.span(), node, was_block)
                };

                return Ok(ExprState::Child(
                    ExprStep::SelectDefault(Box::try_new(StepSelectDefault {
                        buf,
                        arm,
                        state,
                        default_span,
                        was_block,
                    })?),
                    node,
                    Start::Wrapped,
                ));
            }
            Pat => {
                let (pat, node) = {
                    let p = arm.stream();
                    let pat = p.expect(Pat)?.parse(|p| pat_binding(cx, p))?;
                    p.expect(K![=])?;
                    (pat, p.expect(Expr)?)
                };

                return Ok(ExprState::Child(
                    ExprStep::SelectValue(Box::try_new(StepSelectValue {
                        buf,
                        arm,
                        state,
                        pat,
                    })?),
                    node,
                    Start::Wrapped,
                ));
            }
            _ => {
                return Err(arm.stream().expected(ExprSelectArm));
            }
        }
    }

    comma.at_most_one(cx)?;

    let kind = {
        let p = buf.stream();
        p.expect(K!['}'])?;
        alloc_with!(cx, p);

        let SelectState {
            exprs,
            branches,
            default,
        } = state;

        hir::ExprKind::Select(alloc!(hir::ExprSelect {
            exprs: iter!(exprs, |e| expr!(e)),
            branches: iter!(branches),
            default: default.map(|(_, expr)| expr),
        }))
    };

    buf.end()?;
    Ok(ExprState::Kind(kind))
}

/// Require that a resumed child produced a complete expression.
fn expect_expr<'hir>(
    buf: &mut StreamBuf<'_>,
    value: ExprState<'hir, '_>,
) -> Result<hir::Expr<'hir>> {
    match value {
        ExprState::Expr(expr) => Ok(expr),
        _ => Err(Error::msg(buf.stream().span(), "Expected an expression")),
    }
}

struct ExprInner<'hir, 'a> {
    span: Span,
    kind: ExprInnerKind<'hir, 'a>,
}

enum ExprInnerKind<'hir, 'a> {
    Kind(hir::ExprKind<'hir>),
    Path(StreamBuf<'a>),
}

impl<'hir> ExprInner<'hir, '_> {
    fn into_call(self, cx: &mut Ctxt<'hir, '_, '_>, args: usize) -> Result<hir::Call> {
        match self.kind {
            ExprInnerKind::Path(p) => {
                let named = p.parse(|p| cx.q.convert_path2(p))?;
                let parameters = generics_parameters(cx, &named)?;
                let meta = cx.lookup_meta(&self.span, named.item, parameters)?;

                debug_assert_eq!(meta.item_meta.item, named.item);

                match &meta.kind {
                    meta::Kind::Struct {
                        fields: meta::Fields::Empty,
                        ..
                    } => {
                        if args > 0 {
                            return Err(Error::new(
                                self.span,
                                ErrorKind::BadArgumentCount {
                                    expected: 0,
                                    actual: args,
                                },
                            ));
                        }
                    }
                    meta::Kind::Struct {
                        fields: meta::Fields::Unnamed(expected),
                        ..
                    } => {
                        if *expected != args {
                            return Err(Error::new(
                                self.span,
                                ErrorKind::BadArgumentCount {
                                    expected: *expected,
                                    actual: args,
                                },
                            ));
                        }

                        if *expected == 0 {
                            cx.q.diagnostics.remove_tuple_call_parens(
                                cx.source_id,
                                &self.span,
                                &self.span,
                                None,
                            )?;
                        }
                    }
                    meta::Kind::Function { .. } => {
                        if let Some(message) = cx.q.lookup_deprecation(meta.hash) {
                            cx.q.diagnostics.used_deprecated(
                                cx.source_id,
                                &self.span,
                                None,
                                message.try_into()?,
                            )?;
                        };
                    }
                    meta::Kind::ConstFn => {
                        return Ok(hir::Call::ConstFn {
                            id: meta.item_meta.item,
                        });
                    }
                    _ => {
                        return Err(Error::expected_meta(
                            self.span,
                            meta.info(cx.q.pool)?,
                            "something that can be called as a function",
                        ));
                    }
                };

                Ok(hir::Call::Meta { hash: meta.hash })
            }
            ExprInnerKind::Kind(kind) => {
                alloc_with!(cx, &self.span);

                match kind {
                    hir::ExprKind::Variable(name) => Ok(hir::Call::Var { name }),
                    hir::ExprKind::FieldAccess(&hir::ExprFieldAccess {
                        expr_field,
                        expr: target,
                    }) => {
                        let hash = match expr_field {
                            hir::ExprField::Index(index) => Hash::index(index),
                            hir::ExprField::Ident(ident) => {
                                cx.q.unit.insert_debug_ident(ident)?;
                                Hash::ident(ident)
                            }
                            hir::ExprField::IdentGenerics(ident, hash) => {
                                cx.q.unit.insert_debug_ident(ident)?;
                                Hash::ident(ident).with_function_parameters(hash)
                            }
                        };

                        Ok(hir::Call::Associated { target, hash })
                    }
                    kind => Ok(hir::Call::Expr {
                        expr: expr!(hir::Expr {
                            span: self.span,
                            kind
                        }),
                    }),
                }
            }
        }
    }

    fn into_kind(self, cx: &mut Ctxt<'hir, '_, '_>) -> Result<hir::ExprKind<'hir>> {
        match self.kind {
            ExprInnerKind::Kind(kind) => Ok(kind),
            ExprInnerKind::Path(p) => {
                let named = p.parse(|p| cx.q.convert_path2(p))?;
                let parameters = generics_parameters(cx, &named)?;

                if let Some(meta) = cx.try_lookup_meta(&self.span, named.item, &parameters)? {
                    return expr_path_meta(cx, &meta, &self.span);
                }

                if let (Needs::Value, Named2Kind::Ident(local)) = (cx.needs, named.kind) {
                    let local = local.resolve(resolve_context!(cx.q))?;

                    // light heuristics, treat it as a type error in case the first
                    // character is uppercase.
                    if !local.starts_with(char::is_uppercase) {
                        return Err(Error::new(
                            self.span,
                            ErrorKind::MissingLocal {
                                name: Box::<str>::try_from(local)?,
                            },
                        ));
                    }
                }

                let kind = if !parameters.is_empty() {
                    ErrorKind::MissingItemParameters {
                        item: cx.q.pool.item(named.item).try_to_owned()?,
                        parameters: parameters.parameters,
                    }
                } else {
                    ErrorKind::MissingItem {
                        item: cx.q.pool.item(named.item).try_to_owned()?,
                    }
                };

                Err(Error::new(self.span, kind))
            }
        }
    }
}

#[instrument_ast(span = p)]
fn expr_inner<'hir, 'a>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'a>,
) -> Result<ExprInner<'hir, 'a>> {
    let kind = match p.kind() {
        IndexedPath(..) => return expr_path(cx, p),
        Path => return Err(p.expected("an expanded path")),
        // An operand which starts with a modifier - `1 + const { 2 }` - is
        // parsed wrapped, since reading modifiers is what the wrapping half of
        // the grammar is for. Everything else arrives here unwrapped.
        Expr => expr(cx, p)?.kind,
        Block => expr_block(cx, p)?,
        Lit => expr_lit(cx, p)?,
        ConstBlock(item) => expr_const_block(cx, p, item)?,
        AsyncBlock(item) => expr_async_block(cx, p, item)?,
        Closure(item) => expr_closure(cx, p, item)?,
        ExpandedMacro(id) => expr_expanded_macro(cx, p, id)?,
        ExprReturn => expr_return(cx, p)?,
        ExprYield => expr_yield(cx, p)?,
        ExprBreak => expr_break(cx, p)?,
        ExprContinue => expr_continue(cx, p)?,
        ExprArray => expr_array(cx, p)?,
        ExprTuple => expr_tuple(cx, p)?,
        ExprGroup => expr_group(cx, p)?,
        ExprEmptyGroup => expr_empty_group(cx, p)?,
        ExprObject => expr_object(cx, p)?,
        ExprChain => expr_chain(cx, p)?,
        ExprUnary => expr_unary(cx, p)?,
        ExprBinary => expr_binary(cx, p)?,
        ExprAssign => expr_assign(cx, p)?,
        ExprIf => expr_if(cx, p)?,
        ExprMatch => expr_match(cx, p)?,
        ExprSelect => expr_select(cx, p)?,
        ExprWhile => expr_while(cx, p)?,
        ExprLoop => expr_loop(cx, p)?,
        ExprFor => expr_for(cx, p)?,
        ExprRange => expr_range(cx, p)?,
        ExprRangeInclusive => expr_range_inclusive(cx, p)?,
        ExprRangeFrom => expr_range_from(cx, p)?,
        ExprRangeFull => expr_range_full(cx, p)?,
        ExprRangeTo => expr_range_to(cx, p)?,
        ExprRangeToInclusive => expr_range_to_inclusive(cx, p)?,
        _ => return Err(p.expected(Expr)),
    };

    Ok(ExprInner {
        span: p.span(),
        kind: ExprInnerKind::Kind(kind),
    })
}

/// Lower the given block expression.
#[instrument_ast(span = p)]
fn expr_block<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);
    let label = cx.label.take();
    Ok(hir::ExprKind::Block(alloc!(block(cx, label, p)?)))
}

/// Lower the given async block expression.
#[instrument_ast(span = p)]
fn expr_const_block<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
    item: ItemId,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    if cx.const_eval {
        return Ok(hir::ExprKind::Block(alloc!(block(cx, None, p)?)));
    }

    let item = cx.q.item_for("lowering const block", item).with_span(&*p)?;
    let meta = cx.lookup_meta(&*p, item.item, GenericsParameters::default())?;

    let meta::Kind::Const = meta.kind else {
        return Err(Error::expected_meta(
            &*p,
            meta.info(cx.q.pool)?,
            "constant block",
        ));
    };

    p.ignore();
    Ok(hir::ExprKind::Const(meta.hash))
}

/// Lower the given async block expression.
#[instrument_ast(span = p)]
fn expr_async_block<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
    item: ItemId,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    if cx.const_eval {
        return Err(Error::msg(
            &*p,
            "async blocks are not supported in constant contexts",
        ));
    };

    let item = cx.q.item_for("lowering async block", item).with_span(&*p)?;
    let meta = cx.lookup_meta(&*p, item.item, GenericsParameters::default())?;

    let meta::Kind::AsyncBlock { call, do_move, .. } = meta.kind else {
        return Err(Error::expected_meta(
            &*p,
            meta.info(cx.q.pool)?,
            "async block",
        ));
    };

    cx.scopes.push_captures()?;
    let block = alloc!(block(cx, None, p)?);
    let layer = cx.scopes.pop().with_span(&*p)?;

    cx.q.set_used(&meta.item_meta)?;

    let captures = &*iter!(layer.captures().map(|(_, id)| id));

    let Some(queue) = cx.secondary_builds.as_mut() else {
        return Err(Error::new(&*p, ErrorKind::AsyncBlockInConst));
    };

    queue.try_push(query::SecondaryBuildEntry {
        item_meta: meta.item_meta,
        build: query::SecondaryBuild::AsyncBlock(query::AsyncBlock {
            hir: alloc!(hir::AsyncBlock { block, captures }),
            call,
        }),
    })?;

    Ok(hir::ExprKind::AsyncBlock(alloc!(hir::ExprAsyncBlock {
        hash: meta.hash,
        do_move,
        captures,
    })))
}

/// Lower the given path.
#[instrument_ast(span = p)]
fn expr_path<'hir, 'a>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'a>,
) -> Result<ExprInner<'hir, 'a>> {
    alloc_with!(cx, p);

    fn is_self(p: &Stream<'_>) -> bool {
        matches!(p.kinds(), Some([K![self]]))
    }

    fn try_as_ident(p: &Stream<'_>) -> Option<ast::Ident> {
        let [node] = p.nodes()?;
        node.ast().ok()
    }

    let kind = 'out: {
        if is_self(p) {
            let Some((id, _)) = cx.scopes.get(hir::Name::SelfValue)? else {
                return Err(Error::new(&*p, ErrorKind::MissingSelf));
            };

            p.ignore();
            break 'out ExprInnerKind::Kind(hir::ExprKind::Variable(id));
        }

        if let Needs::Value = cx.needs {
            if let Some(name) = try_as_ident(p) {
                let name = alloc_str!(name.resolve(resolve_context!(cx.q))?);

                if let Some((name, _)) = cx.scopes.get(hir::Name::Str(name))? {
                    p.ignore();
                    break 'out ExprInnerKind::Kind(hir::ExprKind::Variable(name));
                }
            }
        }

        ExprInnerKind::Path(p.take_remaining())
    };

    Ok(ExprInner {
        span: p.span(),
        kind,
    })
}

/// Lower the given path.
///
/// What an expansion produced is lowered from a tree of its own, so this
/// recurses rather than descending through the driver, and how deeply
/// expansions may nest is bounded here.
#[instrument_ast(span = p)]
fn expr_expanded_macro<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
    id: NonZeroId,
) -> Result<hir::ExprKind<'hir>> {
    cx.enter_expansion(&*p)?;
    let result = expr_expanded_macro_inner(cx, p, id);
    cx.leave_expansion();
    result
}

fn expr_expanded_macro_inner<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
    id: NonZeroId,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    p.ignore();

    let Some(expanded) = cx.q.take_expanded_macro(id) else {
        return Err(Error::msg(
            &*p,
            try_format!("missing expanded macro for id {id}"),
        ));
    };

    match expanded {
        query::ExpandedMacro::Builtin(e) => match e {
            query::BuiltInMacro2::File(lit) => {
                let lit = lit.resolve_string(resolve_context!(cx.q))?;
                let lit = alloc_str!(lit.as_ref());
                Ok(hir::ExprKind::Lit(hir::Lit::Str(lit)))
            }
            query::BuiltInMacro2::Line(line) => {
                let Ok(n) = u64::try_from(line) else {
                    return Err(Error::new(
                        &*p,
                        ErrorKind::BadUnsignedOutOfBounds {
                            size: NumberSize::S64,
                        },
                    ));
                };

                Ok(hir::ExprKind::Lit(hir::Lit::Unsigned(n)))
            }
            query::BuiltInMacro2::Format(tree) => expr_format_macro(cx, p, tree),
            query::BuiltInMacro2::Template(tree, literal) => {
                expr_template_macro(cx, p, tree, literal)
            }
        },
        query::ExpandedMacro::Tree(tree) => {
            #[cfg(feature = "std")]
            if cx.q.options.print_tree {
                tree.print(&*p, format_args!("Expanded macro tree #{id}"))?;
            }

            let Some([root]) = tree.nodes() else {
                return Err(Error::msg(&*p, "expected single root in expanded macro"));
            };

            if !matches!(root.kind(), Root) {
                return Err(Error::expected(root, Root));
            }

            let Some([expr]) = root.nodes() else {
                return Err(Error::msg(
                    &*p,
                    "expected single expression in expanded macro",
                ));
            };

            if !matches!(expr.kind(), Expr) {
                return Err(Error::expected(expr, Expr));
            }

            expr.parse(|p| Ok(self::expr(cx, p)?.kind))
        }
    }
}

fn expr_format_macro<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
    tree: Rc<Tree>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    let Some([root]) = tree.nodes() else {
        return Err(Error::msg(
            &*p,
            "expected single root in expanded format!()",
        ));
    };

    if !matches!(root.kind(), Root) {
        return Err(Error::expected(root, Root));
    }

    let mut spec = hir::BuiltInFormatSpec::default();

    root.parse(|p| {
        let expr = p.expect(Expr)?.parse(|p| self::expr(cx, p))?;

        while p.eat(K![,]).is_some() {
            let ident = p.ast::<ast::Ident>()?;
            let key = ident.resolve(resolve_context!(cx.q))?;
            p.expect(K![=])?;

            match key {
                "fill" => {
                    if spec.fill.is_some() {
                        return Err(Error::msg(ident, "multiple `format!(.., fill = ..)`"));
                    }

                    let arg = p.ast::<ast::LitChar>()?;
                    let f = arg.resolve(resolve_context!(cx.q))?;
                    spec.fill = Some(f);
                }
                "align" => {
                    if spec.align.is_some() {
                        return Err(Error::msg(ident, "multiple `format!(.., align = ..)`"));
                    }

                    let arg = p.ast::<ast::Ident>()?;
                    let value = arg.resolve(resolve_context!(cx.q))?;

                    let Ok(a) = str::parse::<format::Alignment>(value) else {
                        return Err(Error::unsupported(arg, "`format!(.., align = ..)`"));
                    };

                    spec.align = Some(a);
                }
                "flags" => {
                    if spec.flags.is_some() {
                        return Err(Error::unsupported(
                            ident,
                            "multiple `format!(.., flags = ..)`",
                        ));
                    }

                    let arg = p.ast::<ast::LitNumber>()?;

                    let Some(f) = arg.resolve(resolve_context!(cx.q))?.as_u32() else {
                        return Err(Error::unsupported(arg, "argument out-of-bounds"));
                    };

                    let f = format::Flags::from(f);
                    spec.flags = Some(f);
                }
                "width" => {
                    if spec.width.is_some() {
                        return Err(Error::unsupported(
                            ident,
                            "multiple `format!(.., width = ..)`",
                        ));
                    }

                    let arg = p.ast::<ast::LitNumber>()?;

                    let Some(f) = arg.resolve(resolve_context!(cx.q))?.as_usize() else {
                        return Err(Error::unsupported(arg, "argument out-of-bounds"));
                    };

                    spec.width = NonZero::new(f);
                }
                "precision" => {
                    if spec.precision.is_some() {
                        return Err(Error::unsupported(
                            ident,
                            "multiple `format!(.., precision = ..)`",
                        ));
                    }

                    let arg = p.ast::<ast::LitNumber>()?;

                    let Some(f) = arg.resolve(resolve_context!(cx.q))?.as_usize() else {
                        return Err(Error::unsupported(arg, "argument out-of-bounds"));
                    };

                    spec.precision = Some(f);
                }
                "type" => {
                    if spec.format_type.is_some() {
                        return Err(Error::unsupported(
                            ident,
                            "multiple `format!(.., type = ..)`",
                        ));
                    }

                    let arg = p.ast::<ast::Ident>()?;
                    let value = arg.resolve(resolve_context!(cx.q))?;

                    let Ok(format_type) = str::parse::<format::Type>(value) else {
                        return Err(Error::unsupported(arg, "`format!(.., type = ..)`"));
                    };

                    spec.format_type = Some(format_type);
                }
                _ => {
                    return Err(Error::unsupported(ident, "`format!(.., <key>)`"));
                }
            }
        }

        let format = alloc!(hir::BuiltInFormat {
            span: expr.span,
            spec,
            value: expr!(expr),
        });

        Ok(hir::ExprKind::Format(format))
    })
}

fn expr_template_macro<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
    tree: Rc<Tree>,
    literal: query::BuiltInLiteral,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    let Some([root]) = tree.nodes() else {
        return Err(Error::msg(
            &*p,
            "expected single root in expanded template!()",
        ));
    };

    if !matches!(root.kind(), Root) {
        return Err(Error::expected(root, Root));
    }

    let mut exprs = Vec::new();

    root.parse(|p| {
        let mut comma = Remaining::default();

        let in_template = replace(&mut cx.in_template, true);

        while let MaybeNode::Some(expr) = p.eat(Expr) {
            comma.exactly_one(cx)?;
            exprs.try_push(expr.parse(|p| self::expr(cx, p))?)?;
            comma = p.one(K![,]);
        }

        cx.in_template = in_template;

        comma.at_most_one(cx)?;

        let template = alloc!(hir::BuiltInTemplate {
            span: p.span(),
            from_literal: literal.is_yes(),
            exprs: iter!(exprs, |e| expr!(e)),
        });

        Ok(hir::ExprKind::Template(template))
    })
}

#[instrument_ast(span = p)]
fn expr_return<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);
    p.expect(K![return])?;
    let expr = p.eat(Expr).parse(|p| expr(cx, p))?;
    let expr = match expr {
        Some(expr) => Some(expr!(expr)),
        None => None,
    };

    Ok(hir::ExprKind::Return(expr))
}

#[instrument_ast(span = p)]
fn expr_yield<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);
    p.expect(K![yield])?;
    let expr = p.eat(Expr).parse(|p| expr(cx, p))?;
    let expr = match expr {
        Some(expr) => Some(expr!(expr)),
        None => None,
    };

    Ok(hir::ExprKind::Yield(expr))
}

#[instrument_ast(span = p)]
fn expr_break<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    p.expect(K![break])?;

    let label = p
        .eat_matching(|k| matches!(k, K!['label]))
        .ast::<ast::Label>()?;

    let expr = p.eat(Expr).parse(|p| expr(cx, p))?;

    let label = match label {
        Some(label) => Some(label.resolve(resolve_context!(cx.q))?),
        None => None,
    };

    let Some(drop) = cx.scopes.loop_drop(label)? else {
        if let Some(label) = label {
            return Err(Error::new(
                &*p,
                ErrorKind::MissingLabel {
                    label: label.try_into()?,
                },
            ));
        } else {
            return Err(Error::new(&*p, ErrorKind::BreakUnsupported));
        }
    };

    Ok(hir::ExprKind::Break(alloc!(hir::ExprBreak {
        label: match label {
            Some(label) => Some(alloc_str!(label)),
            None => None,
        },
        expr: match expr {
            Some(expr) => Some(expr!(expr)),
            None => None,
        },
        drop: iter!(drop),
    })))
}

#[instrument_ast(span = p)]
fn expr_continue<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    p.expect(K![continue])?;

    let label = p
        .eat_matching(|k| matches!(k, K!['label]))
        .ast::<ast::Label>()?;

    let label = match label {
        Some(label) => Some(label.resolve(resolve_context!(cx.q))?),
        None => None,
    };

    let Some(drop) = cx.scopes.loop_drop(label)? else {
        if let Some(label) = label {
            return Err(Error::new(
                &*p,
                ErrorKind::MissingLabel {
                    label: label.try_into()?,
                },
            ));
        } else {
            return Err(Error::new(&*p, ErrorKind::ContinueUnsupported));
        }
    };

    let kind = hir::ExprContinue {
        label: match label {
            Some(label) => Some(alloc_str!(label)),
            None => None,
        },
        drop: iter!(drop),
    };

    Ok(hir::ExprKind::Continue(alloc!(kind)))
}

#[instrument_ast(span = p)]
fn expr_array<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    p.expect(K!['['])?;

    let mut items = Vec::new();
    let mut comma = Remaining::default();

    while let MaybeNode::Some(node) = p.eat(Expr) {
        comma.exactly_one(cx)?;
        items.try_push(node.parse(|p| expr(cx, p))?)?;
        comma = p.one(K![,]);
    }

    comma.at_most_one(cx)?;
    p.expect(K![']'])?;

    let seq = alloc!(hir::ExprSeq {
        items: iter!(items, |e| expr!(e))
    });

    Ok(hir::ExprKind::Vec(seq))
}

/// Lower the given tuple.
#[instrument_ast(span = p)]
fn expr_tuple<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    p.expect(K!['('])?;

    let mut items = Vec::new();
    let mut comma = Remaining::default();

    while let MaybeNode::Some(node) = p.eat(Expr) {
        comma.exactly_one(cx)?;
        items.try_push(node.parse(|p| expr(cx, p))?)?;
        comma = p.one(K![,]);
    }

    if items.len() <= 1 {
        comma.exactly_one(cx)?;
    } else {
        comma.at_most_one(cx)?;
    }

    p.expect(K![')'])?;

    let seq = alloc!(hir::ExprSeq {
        items: iter!(items, |e| expr!(e))
    });

    Ok(hir::ExprKind::Tuple(seq))
}

#[instrument_ast(span = p)]
fn expr_group<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    p.expect(K!['('])?;

    let expr = match p.eat(Expr).parse(|p| expr(cx, p))? {
        Some(expr) => expr,
        None => hir::Expr {
            span: p.span(),
            kind: hir::ExprKind::Tuple(&hir::ExprSeq { items: &[] }),
        },
    };

    p.expect(K![')'])?;
    Ok(hir::ExprKind::Group(expr!(expr)))
}

#[instrument_ast(span = p)]
fn expr_empty_group<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    p.expect(Kind::Open(Delimiter::Empty))?;
    let expr = p.expect(Expr)?.parse(|p| expr(cx, p))?;
    p.expect(Kind::Close(Delimiter::Empty))?;

    Ok(hir::ExprKind::Group(expr!(expr)))
}

/// Lower the given tuple.
#[instrument_ast(span = p)]
fn expr_object<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    let key = p.pump()?;

    let mut assignments = Vec::new();
    let mut comma = Remaining::default();
    let mut keys_dup = HashMap::new();

    p.expect(K!['{'])?;

    while matches!(p.peek(), object_key!()) {
        comma.exactly_one(cx)?;

        let (key_span, key) = match p.peek() {
            K![str] => {
                let lit = p.ast::<ast::LitStr>()?;
                let string = lit.resolve(resolve_context!(cx.q))?;
                (lit.span(), alloc_str!(string.as_ref()))
            }
            K![ident] => {
                let ident = p.ast::<ast::Ident>()?;
                let string = ident.resolve(resolve_context!(cx.q))?;
                (ident.span(), alloc_str!(string))
            }
            _ => {
                return Err(p.expected("object key"));
            }
        };

        let assign = if p.eat(K![:]).is_some() {
            p.expect(Expr)?.parse(|p| expr(cx, p))?
        } else {
            let Some((name, _)) = cx.scopes.get(hir::Name::Str(key))? else {
                return Err(Error::new(
                    key_span,
                    ErrorKind::MissingLocal {
                        name: key.try_to_string()?.try_into()?,
                    },
                ));
            };

            hir::Expr {
                span: key_span,
                kind: hir::ExprKind::Variable(name),
            }
        };

        if let Some(_existing) = keys_dup.try_insert(key, key_span)? {
            return Err(Error::new(
                key_span,
                ErrorKind::DuplicateObjectKey {
                    #[cfg(feature = "emit")]
                    existing: _existing.span(),
                    #[cfg(feature = "emit")]
                    object: p.span(),
                },
            ));
        }

        assignments.try_push(hir::FieldAssign {
            key: (key_span, key),
            assign: expr!(assign),
            position: None,
        })?;

        comma = p.one(K![,]);
    }

    comma.at_most_one(cx)?;
    p.expect(K!['}'])?;

    let mut check_object_fields = |fields: &[meta::FieldMeta], item: &crate::Item| {
        let mut named = HashMap::new();

        for f in fields {
            named.try_insert(f.name.as_ref(), f)?;
        }

        for assign in assignments.iter_mut() {
            let Some(meta) = named.remove(assign.key.1) else {
                return Err(Error::new(
                    assign.key.0,
                    ErrorKind::LitObjectNotField {
                        field: assign.key.1.try_into()?,
                        item: item.try_to_owned()?,
                    },
                ));
            };

            assign.position = Some(meta.position);
        }

        if let Some(field) = named.into_keys().next() {
            return Err(Error::new(
                p.span(),
                ErrorKind::LitObjectMissingField {
                    field: field.try_into()?,
                    item: item.try_to_owned()?,
                },
            ));
        }

        Ok(())
    };

    let kind = match key.kind() {
        AnonymousObjectKey => hir::ExprObjectKind::Anonymous,
        IndexedPath(..) => {
            let (named, span) = key.parse(|p| Ok((cx.q.convert_path2(p)?, p.span())))?;
            let parameters = generics_parameters(cx, &named)?;
            let meta = cx.lookup_meta(&span, named.item, parameters)?;
            let item = cx.q.pool.item(meta.item_meta.item);

            match &meta.kind {
                meta::Kind::Struct {
                    fields: meta::Fields::Empty,
                    constructor,
                    ..
                } => {
                    check_object_fields(&[], item)?;

                    match constructor {
                        Some(_) => hir::ExprObjectKind::ExternalType {
                            hash: meta.hash,
                            args: 0,
                        },
                        None => hir::ExprObjectKind::Struct { hash: meta.hash },
                    }
                }
                meta::Kind::Struct {
                    fields: meta::Fields::Named(st),
                    constructor,
                    ..
                } => {
                    check_object_fields(&st.fields, item)?;

                    match constructor {
                        Some(_) => hir::ExprObjectKind::ExternalType {
                            hash: meta.hash,
                            args: st.fields.len(),
                        },
                        None => hir::ExprObjectKind::Struct { hash: meta.hash },
                    }
                }
                _ => {
                    return Err(Error::new(
                        span,
                        ErrorKind::UnsupportedLitObject {
                            meta: meta.info(cx.q.pool)?,
                        },
                    ));
                }
            }
        }
        _ => {
            return Err(p.expected("object key"));
        }
    };

    let object = alloc!(hir::ExprObject {
        kind,
        assignments: iter!(assignments),
    });

    Ok(hir::ExprKind::Object(object))
}

#[instrument_ast(span = p)]
fn expr_chain<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    let node = p.pump()?;

    let label = cx.label.take();

    let mut inner = node.parse(|p| expr_inner(cx, p))?;

    let start = inner.span;

    cx.label = label;

    for node in p.by_ref() {
        let span = start.join(node.span());

        let kind = match node.kind() {
            ExprCall => node.parse(|p| expr_call(cx, p, inner))?,
            ExprField => node.parse(|p| expr_field(cx, p, inner))?,
            ExprIndex => node.parse(|p| expr_index(cx, p, inner))?,
            ExprAwait => node.parse(|p| expr_await(cx, p, inner))?,
            ExprTry => node.parse(|p| expr_try(cx, p, inner))?,
            _ => {
                return Err(node.expected(ExprChain));
            }
        };

        inner = ExprInner {
            span,
            kind: ExprInnerKind::Kind(kind),
        };
    }

    inner.into_kind(cx)
}

#[instrument_ast(span = p)]
fn expr_unary<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    let op = p.ast::<ast::UnOp>()?;

    if let ast::UnOp::BorrowRef { .. } = op {
        return Err(Error::new(op, ErrorKind::UnsupportedRef));
    }

    let inner = p.pump()?.parse(|p| expr_only(cx, p))?;
    let inner = expr!(inner);

    Ok(hir::ExprKind::Unary(alloc!(hir::ExprUnary {
        op,
        expr: inner
    })))
}

#[instrument_ast(span = p)]
fn expr_binary<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    let (mut lhs, mut lhs_span) = p
        .pump()?
        .parse(|p| Ok((expr_inner(cx, p)?.into_kind(cx)?, p.span())))?;

    while !p.is_eof() {
        let node = p.expect(ExprOperator)?;

        let Some(op) = node
            .tokens::<2>()
            .as_deref()
            .and_then(ast::BinOp::from_slice)
        else {
            return Err(node.expected("valid operator"));
        };

        let rhs_needs = match op {
            ast::BinOp::As(..) | ast::BinOp::Is(..) | ast::BinOp::IsNot(..) => Needs::Type,
            _ => Needs::Value,
        };

        let needs = replace(&mut cx.needs, rhs_needs);
        let (rhs, rhs_span) = p
            .pump()?
            .parse(|p| Ok((expr_inner(cx, p)?.into_kind(cx)?, p.span())))?;
        cx.needs = needs;

        let span = lhs_span.join(rhs_span);
        let lhs_span = replace(&mut lhs_span, span);

        lhs = hir::ExprKind::Binary(alloc!(hir::ExprBinary {
            lhs: expr!(hir::Expr {
                span: lhs_span,
                kind: lhs
            }),
            op,
            rhs: expr!(hir::Expr {
                span: rhs_span,
                kind: rhs
            }),
        }));
    }

    Ok(lhs)
}

#[instrument_ast(span = p)]
fn expr_lit<'hir>(cx: &mut Ctxt<'hir, '_, '_>, p: &mut Stream<'_>) -> Result<hir::ExprKind<'hir>> {
    let lit = lit(cx, p)?;
    Ok(hir::ExprKind::Lit(lit))
}

#[instrument_ast(span = p)]
fn expr_assign<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    let lhs = p.expect(Expr)?.parse(|p| expr(cx, p))?;
    p.expect(K![=])?;
    let rhs = p.expect(Expr)?.parse(|p| expr(cx, p))?;

    let lhs = expr!(lhs);
    let rhs = expr!(rhs);

    Ok(hir::ExprKind::Assign(alloc!(hir::ExprAssign { lhs, rhs })))
}

#[instrument_ast(span = p)]
fn expr_if<'hir>(cx: &mut Ctxt<'hir, '_, '_>, p: &mut Stream<'_>) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    let mut branches = Vec::new();

    let start = p.expect(K![if])?;

    cx.scopes.push_loop(None)?;
    let condition = p.pump()?.parse(|p| self::condition(cx, p))?;
    let block = p.expect(Block)?.parse(|p| self::block(cx, None, p))?;
    let layer = cx.scopes.pop().with_span(&*p)?;

    branches.try_push(hir::ConditionalBranch {
        span: start.span().join(block.span),
        block,
        condition: alloc!(condition),
        drop: iter!(layer.into_drop_order()),
    })?;

    let mut fallback = None;

    while fallback.is_none() {
        match p.peek() {
            ExprElse => {
                p.pump()?.parse(|p| {
                    p.expect(K![else])?;
                    let block = p.expect(Block)?.parse(|p| self::block(cx, None, p))?;
                    fallback = Some(alloc!(block));
                    Ok(())
                })?;
            }
            ExprElseIf => {
                p.pump()?.parse(|p| {
                    p.expect(K![else])?;
                    p.expect(K![if])?;

                    cx.scopes.push_loop(None)?;
                    let condition = p.pump()?.parse(|p| self::condition(cx, p))?;
                    let block = p.expect(Block)?.parse(|p| self::block(cx, None, p))?;
                    let layer = cx.scopes.pop().with_span(&*p)?;

                    branches.try_push(hir::ConditionalBranch {
                        span: start.span().join(block.span),
                        block,
                        condition: alloc!(condition),
                        drop: iter!(layer.into_drop_order()),
                    })?;

                    Ok(())
                })?;
            }
            _ => {
                break;
            }
        }
    }

    Ok(hir::ExprKind::If(alloc!(hir::Conditional {
        branches: iter!(branches),
        fallback: option!(fallback),
    })))
}

#[instrument_ast(span = p)]
fn expr_match<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    let mut branches = Vec::new();

    p.expect(K![match])?;

    let expr = p.expect(Expr)?.parse(|p| expr(cx, p))?;

    p.expect(K!['{'])?;

    let mut comma = Remaining::default();
    let mut was_block = false;

    while let MaybeNode::Some(node) = p.eat(ExprMatchArm) {
        if was_block {
            comma.at_most_one(cx)?;
        } else {
            comma.exactly_one(cx)?;
        }

        was_block = node.parse(|p| {
            cx.scopes.push(None)?;

            let pat = p.expect(Pat)?.parse(|p| self::pat_binding(cx, p))?;

            let condition = if p.eat(K![if]).is_some() {
                let expr = p.expect(Expr)?.parse(|p| self::expr(cx, p))?;
                Some(expr!(expr))
            } else {
                None
            };

            p.expect(K![=>])?;

            let (body, is_block) = p.expect(Expr)?.parse(|p| {
                let is_block = matches!(p.peek(), Block);
                let expr = self::expr(cx, p)?;
                Ok((expr, is_block))
            })?;

            let layer = cx.scopes.pop().with_span(&*p)?;

            branches.try_push(hir::ExprMatchBranch {
                span: p.span(),
                pat,
                condition,
                body: expr!(body),
                drop: iter!(layer.into_drop_order()),
            })?;

            Ok(is_block)
        })?;

        comma = p.remaining(cx, K![,])?;
    }

    comma.at_most_one(cx)?;
    p.expect(K!['}'])?;

    Ok(hir::ExprKind::Match(alloc!(hir::ExprMatch {
        expr: expr!(expr),
        branches: iter!(branches),
    })))
}

#[instrument_ast(span = p)]
fn expr_select<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    let mut exprs = Vec::new();
    let mut branches = Vec::new();
    let mut default = None::<(Span, hir::ExprId)>;

    p.expect(K![select])?;

    p.expect(K!['{'])?;

    let mut comma = Remaining::default();
    let mut was_block = false;

    while let MaybeNode::Some(node) = p.eat(ExprSelectArm) {
        if was_block {
            comma.at_most_one(cx)?;
        } else {
            comma.exactly_one(cx)?;
        }

        was_block = node.parse(|p| {
            cx.scopes.push(None)?;

            match p.peek() {
                K![default] => {
                    let default_token = p.expect(K![default])?;
                    p.expect(K![=>])?;

                    let (body, is_block) = p.expect(Expr)?.parse(|p| {
                        let is_block = matches!(p.peek(), Block);
                        let expr = self::expr(cx, p)?;
                        Ok((expr, is_block))
                    })?;

                    if let Some((existing, _)) = default {
                        cx.error(Error::new(
                            &default_token,
                            ErrorKind::DuplicateSelectDefault { existing },
                        ))?;
                    } else {
                        default = Some((body.span, expr!(body)));
                    }

                    Ok(is_block)
                }
                Pat => {
                    let pat = p.expect(Pat)?.parse(|p| self::pat_binding(cx, p))?;
                    p.expect(K![=])?;
                    let expr = p.expect(Expr)?.parse(|p| self::expr(cx, p))?;
                    exprs.try_push(expr)?;
                    p.expect(K![=>])?;

                    let (body, is_block) = p.expect(Expr)?.parse(|p| {
                        let is_block = matches!(p.peek(), Block);
                        let expr = self::expr(cx, p)?;
                        Ok((expr, is_block))
                    })?;

                    let layer = cx.scopes.pop().with_span(&*p)?;

                    branches.try_push(hir::ExprSelectBranch {
                        pat,
                        body: expr!(body),
                        drop: iter!(layer.into_drop_order()),
                    })?;

                    Ok(is_block)
                }
                _ => Err(p.expected(ExprSelectArm)),
            }
        })?;

        comma = p.remaining(cx, K![,])?;
    }

    comma.at_most_one(cx)?;
    p.expect(K!['}'])?;

    Ok(hir::ExprKind::Select(alloc!(hir::ExprSelect {
        exprs: iter!(exprs, |e| expr!(e)),
        branches: iter!(branches),
        default: default.map(|(_, expr)| expr),
    })))
}

#[instrument_ast(span = p)]
fn expr_while<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    let label = match cx.label.take() {
        Some(label) => Some(alloc_str!(label.resolve(resolve_context!(cx.q))?)),
        None => None,
    };

    cx.scopes.push_loop(label)?;

    p.expect(K![while])?;

    let condition = p.pump()?.parse(|p| condition(cx, p))?;
    let body = p.expect(Block)?.parse(|p| block(cx, None, p))?;
    let layer = cx.scopes.pop().with_span(&*p)?;

    Ok(hir::ExprKind::Loop(alloc!(hir::ExprLoop {
        label,
        condition: Some(alloc!(condition)),
        body,
        drop: iter!(layer.into_drop_order()),
    })))
}

#[instrument_ast(span = p)]
fn expr_loop<'hir>(cx: &mut Ctxt<'hir, '_, '_>, p: &mut Stream<'_>) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    let label = match cx.label.take() {
        Some(label) => Some(alloc_str!(label.resolve(resolve_context!(cx.q))?)),
        None => None,
    };

    cx.scopes.push_loop(label)?;

    p.expect(K![loop])?;
    let body = p.expect(Block)?.parse(|p| block(cx, None, p))?;
    let layer = cx.scopes.pop().with_span(&*p)?;

    Ok(hir::ExprKind::Loop(alloc!(hir::ExprLoop {
        label,
        condition: None,
        body,
        drop: iter!(layer.into_drop_order()),
    })))
}

#[instrument_ast(span = p)]
fn expr_for<'hir>(cx: &mut Ctxt<'hir, '_, '_>, p: &mut Stream<'_>) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    p.expect(K![for])?;
    let pat = p.expect(Pat)?;
    p.expect(K![in])?;
    let iter = p.expect(Expr)?;
    let block = p.expect(Block)?;

    let label = match cx.label.take() {
        Some(label) => Some(alloc_str!(label.resolve(resolve_context!(cx.q))?)),
        None => None,
    };

    let iter = iter.parse(|p| expr(cx, p))?;

    cx.scopes.push_loop(label)?;

    let binding = pat.parse(|p| self::pat_binding(cx, p))?;
    let body = block.parse(|p| self::block(cx, None, p))?;

    let layer = cx.scopes.pop().with_span(&*p)?;

    Ok(hir::ExprKind::For(alloc!(hir::ExprFor {
        label,
        binding,
        iter: expr!(iter),
        body,
        drop: iter!(layer.into_drop_order()),
    })))
}

#[instrument_ast(span = p)]
fn expr_range<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    let start = p.pump()?.parse(|p| expr_only(cx, p))?;
    p.expect(K![..])?;
    let end = p.pump()?.parse(|p| expr_only(cx, p))?;

    let start = expr!(start);
    let end = expr!(end);

    Ok(hir::ExprKind::Range(alloc!(hir::ExprRange::Range {
        start,
        end,
    })))
}

#[instrument_ast(span = p)]
fn expr_range_inclusive<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    let start = p.pump()?.parse(|p| expr_only(cx, p))?;
    p.expect(K![..=])?;
    let end = p.pump()?.parse(|p| expr_only(cx, p))?;

    let start = expr!(start);
    let end = expr!(end);

    Ok(hir::ExprKind::Range(alloc!(
        hir::ExprRange::RangeInclusive { start, end }
    )))
}

#[instrument_ast(span = p)]
fn expr_range_from<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    let start = p.pump()?.parse(|p| expr_only(cx, p))?;
    p.expect(K![..])?;

    let start = expr!(start);

    Ok(hir::ExprKind::Range(alloc!(hir::ExprRange::RangeFrom {
        start,
    })))
}

#[instrument_ast(span = p)]
fn expr_range_full<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    p.expect(K![..])?;

    Ok(hir::ExprKind::Range(alloc!(hir::ExprRange::RangeFull)))
}

#[instrument_ast(span = p)]
fn expr_range_to<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    p.expect(K![..])?;
    let end = p.pump()?.parse(|p| expr_only(cx, p))?;

    let end = expr!(end);

    Ok(hir::ExprKind::Range(alloc!(hir::ExprRange::RangeTo {
        end,
    })))
}

#[instrument_ast(span = p)]
fn expr_range_to_inclusive<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    p.expect(K![..=])?;
    let end = p.pump()?.parse(|p| expr_only(cx, p))?;

    let end = expr!(end);

    Ok(hir::ExprKind::Range(alloc!(
        hir::ExprRange::RangeToInclusive { end }
    )))
}

#[instrument_ast(span = p)]
fn condition<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::Condition<'hir>> {
    alloc_with!(cx, p);

    match p.kind() {
        Condition => Ok(hir::Condition::ExprLet(alloc!(expr_let(cx, p)?))),
        Expr => {
            let expr = expr(cx, p)?;
            Ok(hir::Condition::Expr(expr.span, expr!(expr)))
        }
        _ => Err(p.expected(Condition)),
    }
}

#[instrument_ast(span = p)]
fn expr_let<'hir>(cx: &mut Ctxt<'hir, '_, '_>, p: &mut Stream<'_>) -> Result<hir::ExprLet<'hir>> {
    alloc_with!(cx, p);

    p.expect(K![let])?;
    let pat = p.expect(Pat)?;
    p.expect(K![=])?;
    let expr = p.expect(Expr)?;

    let expr = expr.parse(|p| self::expr(cx, p))?;
    let pat = pat.parse(|p| self::pat_binding(cx, p))?;

    Ok(hir::ExprLet {
        pat,
        expr: expr!(expr),
    })
}

/// Assemble a closure expression.
#[instrument_ast(span = p)]
fn expr_closure<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
    item: ItemId,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    let Some(meta) = cx.q.query_meta(&*p, item, Used::default())? else {
        return Err(Error::new(
            &*p,
            ErrorKind::MissingItem {
                item: cx.q.pool.item(item).try_to_owned()?,
            },
        ));
    };

    let meta::Kind::Closure { call, do_move, .. } = meta.kind else {
        return Err(Error::expected_meta(
            &*p,
            meta.info(cx.q.pool)?,
            "a closure",
        ));
    };

    tracing::trace!("queuing closure build entry");

    cx.scopes.push_captures()?;

    let args = p.expect(ClosureArguments)?.parse(|p| {
        if matches!(p.peek(), K![||]) {
            p.pump()?;
            return Ok(&[][..]);
        };

        p.expect(K![|])?;

        let mut args = Vec::new();
        let mut comma = Remaining::default();

        while let MaybeNode::Some(pat) = p.eat(Pat) {
            comma.exactly_one(cx)?;
            let binding = pat.parse(|p| self::pat_binding(cx, p))?;
            comma = p.remaining(cx, K![,])?;
            args.try_push(hir::FnArg::Pat(alloc!(binding)))
                .with_span(&*p)?;
        }

        comma.at_most_one(cx)?;
        p.expect(K![|])?;
        Ok(iter!(args))
    })?;

    let body = p.expect(Expr)?.parse(|p| expr(cx, p))?;
    let body_span = body.span;
    let body = expr!(body);

    let layer = cx.scopes.pop().with_span(&*p)?;

    cx.q.set_used(&meta.item_meta)?;

    let captures = &*iter!(layer.captures().map(|(_, id)| id));

    let Some(queue) = cx.secondary_builds.as_mut() else {
        return Err(Error::new(&*p, ErrorKind::ClosureInConst));
    };

    queue.try_push(query::SecondaryBuildEntry {
        item_meta: meta.item_meta,
        build: query::SecondaryBuild::Closure(query::Closure {
            hir: alloc!(hir::ExprClosure {
                span: body_span,
                args,
                body,
                captures,
            }),
            call,
        }),
    })?;

    if captures.is_empty() {
        return Ok(hir::ExprKind::Fn(meta.hash));
    }

    Ok(hir::ExprKind::CallClosure(alloc!(hir::ExprCallClosure {
        hash: meta.hash,
        do_move,
        captures,
    })))
}

#[instrument_ast(span = p)]
fn expr_call<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
    inner: ExprInner<'hir, '_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    p.expect(K!['('])?;

    let mut comma = Remaining::default();
    let mut args = Vec::new();

    while let MaybeNode::Some(node) = p.eat(Expr) {
        comma.exactly_one(cx)?;
        let expr = node.parse(|p| expr(cx, p))?;
        args.try_push(expr)?;
        comma = p.one(K![,]);
    }

    comma.at_most_one(cx)?;
    p.expect(K![')'])?;

    let call = inner.into_call(cx, args.len())?;

    let kind = hir::ExprKind::Call(alloc!(hir::ExprCall {
        call,
        args: iter!(args, |e| expr!(e)),
    }));

    Ok(kind)
}

#[instrument_ast(span = p)]
fn expr_field<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
    inner: ExprInner<'hir, '_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    p.expect(K![.])?;

    let expr_field = match p.peek() {
        K![number] => {
            let number = p.ast::<ast::LitNumber>()?;
            let index = number.resolve(resolve_context!(cx.q))?;

            let Some(index) = index.as_tuple_index() else {
                // `a.0.1` lexes its two indices as the one number `0.1`, since
                // nothing about a number says what it is written next to. Two
                // indices separated by a point is what it can be here and
                // nothing else, so it is taken apart again.
                let Some((first, second)) = tuple_index_pair(cx, &number) else {
                    return Err(Error::new(
                        number,
                        ErrorKind::UnsupportedTupleIndex { number: index },
                    ));
                };

                let span = inner.span;
                let kind = inner.into_kind(cx)?;

                let kind = hir::ExprKind::FieldAccess(alloc!(hir::ExprFieldAccess {
                    expr: expr!(hir::Expr { span, kind }),
                    expr_field: hir::ExprField::Index(first),
                }));

                let kind = hir::ExprKind::FieldAccess(alloc!(hir::ExprFieldAccess {
                    expr: expr!(hir::Expr { span, kind }),
                    expr_field: hir::ExprField::Index(second),
                }));

                return Ok(kind);
            };

            hir::ExprField::Index(index)
        }
        IndexedPath(..) => p.pump()?.parse(|p| match p.kinds() {
            Some([K![ident]]) => {
                let base = p.ast::<ast::Ident>()?;
                let base = base.resolve(resolve_context!(cx.q))?;
                let base = alloc_str!(base);
                Ok(hir::ExprField::Ident(base))
            }
            None => {
                let base = p.ast::<ast::Ident>()?;
                let base = base.resolve(resolve_context!(cx.q))?;
                let base = alloc_str!(base);

                if p.eat(K![::]).is_some() {
                    let hash = p
                        .expect(PathGenerics)?
                        .parse(|p| generic_arguments(cx, p))?;
                    Ok(hir::ExprField::IdentGenerics(base, hash))
                } else {
                    Ok(hir::ExprField::Ident(base))
                }
            }
            _ => Err(p.expected_peek(Path)),
        })?,
        _ => {
            return Err(p.expected(ExprField));
        }
    };

    let span = inner.span;
    let kind = inner.into_kind(cx)?;

    let kind = hir::ExprKind::FieldAccess(alloc!(hir::ExprFieldAccess {
        expr: expr!(hir::Expr { span, kind }),
        expr_field,
    }));

    Ok(kind)
}

/// Take a number written where a field is expected apart into the two tuple
/// indices it is.
///
/// Only a plain decimal number with a point in it and nothing else can be two
/// indices: a base prefix, a suffix, an exponent or a digit separator all mean
/// it was written as a number and is simply not one which can index a tuple.
fn tuple_index_pair(cx: &Ctxt<'_, '_, '_>, number: &ast::LitNumber) -> Option<(usize, usize)> {
    let ast::NumberSource::Text(text) = number.source else {
        return None;
    };

    if !text.is_fractional
        || !matches!(text.base, ast::NumberBase::Decimal)
        || !text.suffix.range().is_empty()
    {
        return None;
    }

    let source = cx.q.sources.source(text.source_id, text.number)?;
    let (first, second) = source.split_once('.')?;

    if first.is_empty() || second.is_empty() {
        return None;
    }

    if !first
        .bytes()
        .chain(second.bytes())
        .all(|b| b.is_ascii_digit())
    {
        return None;
    }

    Some((first.parse().ok()?, second.parse().ok()?))
}

#[instrument_ast(span = p)]
fn expr_index<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
    inner: ExprInner<'hir, '_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    p.expect(K!['['])?;
    let index = p.expect(Expr)?.parse(|p| self::expr(cx, p))?;
    p.expect(K![']'])?;

    let span = inner.span;
    let kind = inner.into_kind(cx)?;

    let kind = hir::ExprKind::Index(alloc!(hir::ExprIndex {
        target: expr!(hir::Expr { span, kind }),
        index: expr!(index),
    }));

    Ok(kind)
}

#[instrument_ast(span = p)]
fn expr_await<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
    inner: ExprInner<'hir, '_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    p.expect(K![.])?;
    p.expect(K![await])?;

    let span = inner.span;
    let kind = inner.into_kind(cx)?;

    Ok(hir::ExprKind::Await(expr!(hir::Expr { span, kind })))
}

#[instrument_ast(span = p)]
fn expr_try<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
    inner: ExprInner<'hir, '_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);
    p.expect(K![?])?;
    let span = inner.span.join(p.span());
    let kind = inner.into_kind(cx)?;
    Ok(hir::ExprKind::Try(expr!(hir::Expr { span, kind })))
}

/// Compile an item.
#[instrument_ast(span = span)]
fn expr_path_meta<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    meta: &meta::Meta,
    span: &dyn Spanned,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, span);

    if let Needs::Value = cx.needs {
        match &meta.kind {
            meta::Kind::Struct {
                fields: meta::Fields::Empty | meta::Fields::Unnamed(0),
                ..
            } => Ok(hir::ExprKind::Call(alloc!(hir::ExprCall {
                call: hir::Call::Meta { hash: meta.hash },
                args: &[],
            }))),
            meta::Kind::Struct {
                fields: meta::Fields::Unnamed(..),
                ..
            } => Ok(hir::ExprKind::Fn(meta.hash)),
            meta::Kind::Function { .. } => Ok(hir::ExprKind::Fn(meta.hash)),
            meta::Kind::Const => Ok(hir::ExprKind::Const(meta.hash)),
            meta::Kind::Static => Ok(hir::ExprKind::Static(meta.hash)),
            meta::Kind::Struct { .. } | meta::Kind::Type { .. } | meta::Kind::Enum { .. } => {
                Ok(hir::ExprKind::Type(Type::new(meta.hash)))
            }
            _ => Err(Error::expected_meta(
                span,
                meta.info(cx.q.pool)?,
                "something that can be used as a value",
            )),
        }
    } else {
        let Some(type_hash) = meta.type_hash_of() else {
            return Err(Error::expected_meta(
                span,
                meta.info(cx.q.pool)?,
                "something that has a type",
            ));
        };

        Ok(hir::ExprKind::Type(Type::new(type_hash)))
    }
}

fn pat_binding<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::PatBinding<'hir>> {
    pat_binding_with(cx, p, false)
}

fn pat_binding_with<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
    self_value: bool,
) -> Result<hir::PatBinding<'hir>> {
    alloc_with!(cx, p);
    let pat = pat_node(cx, p.pump()?, self_value)?;
    let names = iter!(cx.pattern_bindings.drain(..));
    Ok(hir::PatBinding { pat, names })
}

#[instrument_ast(span = p)]
fn pat_path<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
    self_value: bool,
) -> Result<hir::Pat<'hir>> {
    alloc_with!(cx, p);

    let named = cx.q.convert_path2(p)?;
    let parameters = generics_parameters(cx, &named)?;

    let path = 'path: {
        if let Some(meta) = cx.try_lookup_meta(&*p, named.item, &parameters)? {
            match meta.kind {
                meta::Kind::Const => {
                    let Some(const_value) = cx.q.get_const_value(meta.hash) else {
                        return Err(Error::msg(
                            &*p,
                            try_format!("Missing constant for hash {}", meta.hash),
                        ));
                    };

                    let const_value = const_value.try_clone().with_span(&*p)?;
                    return pat_const_value(cx, &const_value, &*p);
                }
                meta::Kind::Static => {
                    return Err(Error::new(
                        &*p,
                        ErrorKind::StaticInPattern {
                            item: cx.q.pool.item(meta.item_meta.item).try_to_owned()?,
                        },
                    ));
                }
                _ => {
                    if let Some((0, kind)) = tuple_match_for(&meta) {
                        break 'path hir::PatPathKind::Kind(alloc!(kind));
                    }
                }
            }
        };

        match named.kind {
            Named2Kind::SelfValue(ast) if self_value => {
                let name = cx.scopes.define(hir::Name::SelfValue, &ast)?;
                cx.pattern_bindings.try_push(name)?;
                break 'path hir::PatPathKind::Ident(name);
            }
            Named2Kind::Ident(ident) => {
                let name = alloc_str!(ident.resolve(resolve_context!(cx.q))?);
                let name = cx.scopes.define(hir::Name::Str(name), &*p)?;
                cx.pattern_bindings.try_push(name)?;
                break 'path hir::PatPathKind::Ident(name);
            }
            _ => {
                return Err(Error::new(&*p, ErrorKind::UnsupportedBinding));
            }
        }
    };

    let kind = hir::PatKind::Path(alloc!(path));

    Ok(hir::Pat {
        span: p.span(),
        kind,
    })
}

#[instrument_ast(span = p)]
fn pat_lit<'hir>(cx: &mut Ctxt<'hir, '_, '_>, p: &mut Stream<'_>) -> Result<hir::Pat<'hir>> {
    alloc_with!(cx, p);

    let lit = lit(cx, p)?;

    let expr = expr!(hir::Expr {
        span: p.span(),
        kind: hir::ExprKind::Lit(lit),
    });

    Ok(hir::Pat {
        span: p.span(),
        kind: hir::PatKind::Lit(expr),
    })
}

/// A pattern which is partially lowered, waiting on one of its children.
enum PatStep<'hir, 'a> {
    /// A tuple or an array pattern.
    Seq {
        buf: StreamBuf<'a>,
        path: Option<Node<'a>>,
        items: Vec<hir::Pat<'hir>>,
        comma: Remaining<'a>,
        array: bool,
    },
    /// An object pattern, waiting on the value of `key`.
    Object {
        buf: StreamBuf<'a>,
        path: Option<Node<'a>>,
        bindings: Vec<hir::Binding<'hir>>,
        keys_dup: HashMap<&'hir str, Span>,
        key: (Span, &'hir str),
    },
}

/// What lowering a pattern produced.
enum PatState<'hir, 'a> {
    /// The pattern is complete, along with the stream it was lowered from.
    Done(StreamBuf<'a>, hir::Pat<'hir>),
    /// Park the step and lower the given child pattern next.
    Child(PatStep<'hir, 'a>, Node<'a>),
}

/// Lower the pattern in the given node.
///
/// Patterns nest through tuple, object and array patterns, which is walked over
/// an explicit stack rather than recursively.
fn pat_node<'hir, 'a>(
    cx: &mut Ctxt<'hir, '_, '_>,
    node: Node<'a>,
    self_value: bool,
) -> Result<hir::Pat<'hir>> {
    let span = node.span();
    let mut stack = Vec::new();
    let mut state = pat_start(cx, node.into_stream(), self_value)?;

    loop {
        match state {
            PatState::Done(buf, pat) => {
                buf.end()?;

                let Some(step) = stack.pop() else {
                    return Ok(pat);
                };

                state = pat_resume(cx, step, pat)?;
            }
            PatState::Child(step, node) => {
                stack.try_push(step).with_span(span)?;
                state = pat_start(cx, node.into_stream(), false)?;
            }
        }
    }
}

/// Start lowering a pattern, parking a step if it has children.
fn pat_start<'hir, 'a>(
    cx: &mut Ctxt<'hir, '_, '_>,
    mut buf: StreamBuf<'a>,
    self_value: bool,
) -> Result<PatState<'hir, 'a>> {
    match buf.stream().kind() {
        Lit => {
            let p = buf.stream();
            let pat = pat_lit(cx, p)?;
            Ok(PatState::Done(buf, pat))
        }
        PatIgnore => {
            let p = buf.stream();

            let pat = hir::Pat {
                span: p.expect(K![_])?.span(),
                kind: hir::PatKind::Ignore,
            };

            Ok(PatState::Done(buf, pat))
        }
        IndexedPath(..) => {
            let p = buf.stream();
            let pat = pat_path(cx, p, self_value)?;
            Ok(PatState::Done(buf, pat))
        }
        PatTuple => {
            let path = {
                let p = buf.stream();
                let path = match p.eat_matching(|kind| matches!(kind, IndexedPath(..))) {
                    MaybeNode::Some(node) => Some(node),
                    MaybeNode::None => None,
                };

                p.expect(K!['('])?;
                path
            };

            pat_seq_next(cx, buf, path, Vec::new(), Remaining::default(), false)
        }
        PatArray => {
            {
                let p = buf.stream();
                p.expect(K!['['])?;
            }

            pat_seq_next(cx, buf, None, Vec::new(), Remaining::default(), true)
        }
        PatObject => {
            let path = {
                let p = buf.stream();
                let key = p.pump()?;

                let path = match key.kind() {
                    AnonymousObjectKey => None,
                    IndexedPath(..) => Some(key),
                    _ => {
                        return Err(p.expected_peek("object kind"));
                    }
                };

                p.expect(K!['{'])?;
                path
            };

            pat_object_next(
                cx,
                buf,
                path,
                Vec::new(),
                Remaining::default(),
                HashMap::new(),
            )
        }
        _ => Err(buf.stream().expected(Pat)),
    }
}

/// Resume a pattern whose child has just been lowered.
fn pat_resume<'hir, 'a>(
    cx: &mut Ctxt<'hir, '_, '_>,
    step: PatStep<'hir, 'a>,
    pat: hir::Pat<'hir>,
) -> Result<PatState<'hir, 'a>> {
    match step {
        PatStep::Seq {
            mut buf,
            path,
            mut items,
            comma,
            array,
        } => {
            comma.exactly_one(cx)?;
            items.try_push(pat).with_span(buf.stream().span())?;
            let comma = buf.stream().one(K![,]);
            pat_seq_next(cx, buf, path, items, comma, array)
        }
        PatStep::Object {
            mut buf,
            path,
            mut bindings,
            keys_dup,
            key: (_, key),
        } => {
            let binding = {
                let p = buf.stream();
                alloc_with!(cx, p);
                hir::Binding::Binding(p.span(), key, alloc!(pat))
            };

            bindings.try_push(binding).with_span(buf.stream().span())?;
            let comma = buf.stream().one(K![,]);
            pat_object_next(cx, buf, path, bindings, comma, keys_dup)
        }
    }
}

/// Lower the next item of a tuple or array pattern, or finish it.
fn pat_seq_next<'hir, 'a>(
    cx: &mut Ctxt<'hir, '_, '_>,
    mut buf: StreamBuf<'a>,
    path: Option<Node<'a>>,
    items: Vec<hir::Pat<'hir>>,
    comma: Remaining<'a>,
    array: bool,
) -> Result<PatState<'hir, 'a>> {
    if let MaybeNode::Some(node) = buf.stream().eat(Pat) {
        let node = node.parse(|p| p.pump())?;

        return Ok(PatState::Child(
            PatStep::Seq {
                buf,
                path,
                items,
                comma,
                array,
            },
            node,
        ));
    }

    let (is_open, span) = {
        let p = buf.stream();

        let is_open = if p.eat(K![..]).is_some() {
            comma.exactly_one(cx)?;
            true
        } else {
            comma.at_most_one(cx)?;
            false
        };

        p.expect(if array { K![']'] } else { K![')'] })?;
        (is_open, p.span())
    };

    let items = {
        let p = buf.stream();
        alloc_with!(cx, p);
        iter!(items)
    };

    let kind = if array {
        hir::PatSequenceKind::Sequence {
            hash: runtime::Vec::HASH,
            count: items.len(),
            is_open,
        }
    } else if let Some(path) = path {
        let (named, path_span) = path.parse(|p| Ok((cx.q.convert_path2(p)?, p.span())))?;
        let parameters = generics_parameters(cx, &named)?;
        let meta = cx.lookup_meta(&path_span, named.item, parameters)?;

        // Treat the current meta as a tuple and get the number of arguments it
        // should receive and the type check that applies to it.
        let Some((args, kind)) = tuple_match_for(&meta) else {
            return Err(Error::expected_meta(
                path_span,
                meta.info(cx.q.pool)?,
                "type that can be used in a tuple pattern",
            ));
        };

        if !(args == items.len() || items.len() < args && is_open) {
            cx.error(Error::new(
                path_span,
                ErrorKind::BadArgumentCount {
                    expected: args,
                    actual: items.len(),
                },
            ))?;
        }

        kind
    } else {
        hir::PatSequenceKind::Sequence {
            hash: runtime::Tuple::HASH,
            count: items.len(),
            is_open,
        }
    };

    let pat = {
        let p = buf.stream();
        alloc_with!(cx, p);

        hir::Pat {
            span,
            kind: hir::PatKind::Sequence(alloc!(hir::PatSequence { kind, items })),
        }
    };

    Ok(PatState::Done(buf, pat))
}

/// Lower the next field of an object pattern, or finish it.
fn pat_object_next<'hir, 'a>(
    cx: &mut Ctxt<'hir, '_, '_>,
    mut buf: StreamBuf<'a>,
    path: Option<Node<'a>>,
    mut bindings: Vec<hir::Binding<'hir>>,
    mut comma: Remaining<'a>,
    mut keys_dup: HashMap<&'hir str, Span>,
) -> Result<PatState<'hir, 'a>> {
    while matches!(buf.stream().peek(), object_key!()) {
        comma.exactly_one(cx)?;

        let (span, key) = {
            let p = buf.stream();
            alloc_with!(cx, p);

            match p.peek() {
                K![str] => {
                    let lit = p.ast::<ast::LitStr>()?;
                    let string = lit.resolve(resolve_context!(cx.q))?;
                    (lit.span(), alloc_str!(string.as_ref()))
                }
                K![ident] => {
                    let ident = p.ast::<ast::Ident>()?;
                    let string = ident.resolve(resolve_context!(cx.q))?;
                    (ident.span(), alloc_str!(string))
                }
                _ => {
                    return Err(p.expected_peek("object key"));
                }
            }
        };

        if let Some(_existing) = keys_dup.try_insert(key, span)? {
            return Err(Error::new(
                span,
                ErrorKind::DuplicateObjectKey {
                    #[cfg(feature = "emit")]
                    existing: _existing.span(),
                    #[cfg(feature = "emit")]
                    object: buf.stream().span(),
                },
            ));
        }

        if buf.stream().eat(K![:]).is_some() {
            let node = buf.stream().expect(Pat)?.parse(|p| p.pump())?;

            return Ok(PatState::Child(
                PatStep::Object {
                    buf,
                    path,
                    bindings,
                    keys_dup,
                    key: (span, key),
                },
                node,
            ));
        }

        let binding = {
            let p = buf.stream();
            let id = cx.scopes.define(hir::Name::Str(key), &*p)?;
            cx.pattern_bindings.try_push(id)?;
            hir::Binding::Ident(p.span(), key, id)
        };

        bindings.try_push(binding).with_span(span)?;
        comma = buf.stream().one(K![,]);
    }

    let (is_open, span) = {
        let p = buf.stream();

        let is_open = if p.eat(K![..]).is_some() {
            comma.exactly_one(cx)?;
            true
        } else {
            comma.at_most_one(cx)?;
            false
        };

        p.expect(K!['}'])?;
        (is_open, p.span())
    };

    let kind = match path {
        Some(path) => {
            let (named, path_span) = path.parse(|p| Ok((cx.q.convert_path2(p)?, p.span())))?;
            let parameters = generics_parameters(cx, &named)?;
            let meta = cx.lookup_meta(&path_span, named.item, parameters)?;

            let Some((mut fields, kind)) = struct_match_for(&meta, is_open && bindings.is_empty())?
            else {
                return Err(Error::expected_meta(
                    path_span,
                    meta.info(cx.q.pool)?,
                    "type that can be used in a struct pattern",
                ));
            };

            for binding in bindings.iter() {
                if !fields.remove(binding.key()) {
                    return Err(Error::new(
                        path_span,
                        ErrorKind::LitObjectNotField {
                            field: binding.key().try_into()?,
                            item: cx.q.pool.item(meta.item_meta.item).try_to_owned()?,
                        },
                    ));
                }
            }

            if !is_open && !fields.is_empty() {
                let mut fields = fields.into_iter().try_collect::<Box<[_]>>()?;
                fields.sort();

                return Err(Error::new(
                    span,
                    ErrorKind::PatternMissingFields {
                        item: cx.q.pool.item(meta.item_meta.item).try_to_owned()?,
                        #[cfg(feature = "emit")]
                        fields,
                    },
                ));
            }

            kind
        }
        None => hir::PatSequenceKind::Sequence {
            hash: runtime::Object::HASH,
            count: bindings.len(),
            is_open,
        },
    };

    let pat = {
        let p = buf.stream();
        alloc_with!(cx, p);
        let bindings = iter!(bindings);

        hir::Pat {
            span,
            kind: hir::PatKind::Object(alloc!(hir::PatObject { kind, bindings })),
        }
    };

    Ok(PatState::Done(buf, pat))
}

fn generics_parameters(
    cx: &mut Ctxt<'_, '_, '_>,
    named: &Named2<'_>,
) -> Result<GenericsParameters> {
    let mut parameters = GenericsParameters {
        trailing: named.trailing,
        parameters: [None, None],
    };

    for (value, o) in named
        .parameters
        .iter()
        .zip(parameters.parameters.iter_mut())
    {
        if let Some(node) = value {
            let hash = node.clone().parse(|p| generic_arguments(cx, p))?;
            *o = Some(hash);
        }
    }

    Ok(parameters)
}

fn generic_arguments(cx: &mut Ctxt<'_, '_, '_>, p: &mut Stream<'_>) -> Result<Hash> {
    cx.enter_generics(&*p)?;
    let result = generic_arguments_inner(cx, p);
    cx.leave_generics();
    result
}

fn generic_arguments_inner(cx: &mut Ctxt<'_, '_, '_>, p: &mut Stream<'_>) -> Result<Hash> {
    p.expect(K![<])?;

    let mut comma = Remaining::default();
    let mut builder = ParametersBuilder::new();

    let needs = replace(&mut cx.needs, Needs::Type);

    while matches!(p.peek(), IndexedPath(..)) {
        comma.exactly_one(cx)?;

        let hir::ExprKind::Type(ty) = p.pump()?.parse(|p| expr_path(cx, p)?.into_kind(cx))? else {
            return Err(Error::new(&*p, ErrorKind::UnsupportedGenerics));
        };

        builder = builder.add(ty.into_hash()).with_span(&*p)?;
        comma = p.one(K![,]);
    }

    cx.needs = needs;

    comma.at_most_one(cx)?;
    p.expect(K![>])?;
    Ok(builder.finish())
}

/// Construct a pattern from a constant value.
#[instrument_ast(span = span)]
fn pat_const_value<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    const_value: &ConstValue,
    span: &dyn Spanned,
) -> Result<hir::Pat<'hir>> {
    alloc_with!(cx, span);

    let kind = 'kind: {
        let lit = match const_value.as_kind() {
            ConstValueKind::Inline(value) => match *value {
                Inline::Unit => {
                    break 'kind hir::PatKind::Sequence(alloc!(hir::PatSequence {
                        kind: hir::PatSequenceKind::Sequence {
                            hash: runtime::Tuple::HASH,
                            count: 0,
                            is_open: false,
                        },
                        items: &[],
                    }));
                }
                Inline::Bool(b) => hir::Lit::Bool(b),
                Inline::Char(ch) => hir::Lit::Char(ch),
                Inline::Unsigned(value) => hir::Lit::Unsigned(value),
                Inline::Signed(value) => hir::Lit::Signed(value),
                _ => return Err(Error::msg(span, "Unsupported constant value in pattern")),
            },
            ConstValueKind::String(string) => hir::Lit::Str(alloc_str!(string.as_ref())),
            ConstValueKind::Bytes(bytes) => hir::Lit::ByteStr(alloc_bytes!(bytes.as_ref())),
            ConstValueKind::Instance(instance) => match &**instance {
                ConstInstance {
                    hash: runtime::Object::HASH,
                    variant_hash: Hash::EMPTY,
                    fields,
                } => {
                    let bindings = iter!(fields.iter(), fields.len(), |value| {
                        let (key, value) = value.as_pair().with_span(span)?;
                        let key = key.as_string().with_span(span)?;
                        let pat = alloc!(pat_const_value(cx, value, span)?);
                        hir::Binding::Binding(span.span(), alloc_str!(key.as_ref()), pat)
                    });

                    break 'kind hir::PatKind::Object(alloc!(hir::PatObject {
                        kind: hir::PatSequenceKind::Sequence {
                            hash: runtime::Object::HASH,
                            count: bindings.len(),
                            is_open: false,
                        },
                        bindings,
                    }));
                }
                ConstInstance {
                    hash,
                    variant_hash: Hash::EMPTY,
                    fields,
                } => {
                    let items = iter!(fields.iter(), fields.len(), |value| pat_const_value(
                        cx, value, span
                    )?);

                    break 'kind hir::PatKind::Sequence(alloc!(hir::PatSequence {
                        kind: hir::PatSequenceKind::Sequence {
                            hash: *hash,
                            count: items.len(),
                            is_open: false,
                        },
                        items,
                    }));
                }
                _ => return Err(Error::msg(span, "Unsupported constant value in pattern")),
            },
        };

        hir::PatKind::Lit(expr!(hir::Expr {
            span: span.span(),
            kind: hir::ExprKind::Lit(lit),
        }))
    };

    Ok(hir::Pat {
        span: span.span(),
        kind,
    })
}

/// Generate a legal struct match for the given meta which indicates the type of
/// sequence and the fields that it expects.
///
/// For `open` matches (i.e. `{ .. }`), `Unnamed` and `Empty` structs are also
/// supported and they report empty fields.
fn struct_match_for(
    meta: &meta::Meta,
    open: bool,
) -> alloc::Result<Option<(HashSet<Box<str>>, hir::PatSequenceKind)>> {
    let (fields, kind) = match meta.kind {
        meta::Kind::Struct {
            ref fields,
            enum_hash,
            ..
        } => {
            let kind = 'kind: {
                if enum_hash != Hash::EMPTY {
                    break 'kind hir::PatSequenceKind::Type {
                        hash: enum_hash,
                        variant_hash: meta.hash,
                    };
                }

                hir::PatSequenceKind::Type {
                    hash: meta.hash,
                    variant_hash: Hash::EMPTY,
                }
            };

            (fields, kind)
        }
        meta::Kind::Type { .. } if open => {
            return Ok(Some((
                HashSet::new(),
                hir::PatSequenceKind::Type {
                    hash: meta.hash,
                    variant_hash: Hash::EMPTY,
                },
            )));
        }
        _ => {
            return Ok(None);
        }
    };

    let fields = match fields {
        meta::Fields::Named(st) => st
            .fields
            .iter()
            .map(|f| f.name.try_clone())
            .try_collect::<alloc::Result<_>>()??,
        _ if open => HashSet::new(),
        _ => return Ok(None),
    };

    Ok(Some((fields, kind)))
}

fn tuple_match_for(meta: &meta::Meta) -> Option<(usize, hir::PatSequenceKind)> {
    match meta.kind {
        meta::Kind::Struct {
            ref fields,
            enum_hash,
            ..
        } => {
            let args = fields.as_tuple()?;

            let kind = 'kind: {
                if enum_hash != Hash::EMPTY {
                    break 'kind hir::PatSequenceKind::Type {
                        hash: enum_hash,
                        variant_hash: meta.hash,
                    };
                }

                hir::PatSequenceKind::Type {
                    hash: meta.hash,
                    variant_hash: Hash::EMPTY,
                }
            };

            Some((args, kind))
        }
        _ => None,
    }
}

#[instrument_ast(span = p)]
fn lit<'hir>(cx: &mut Ctxt<'hir, '_, '_>, p: &mut Stream<'_>) -> Result<hir::Lit<'hir>> {
    alloc_with!(cx, p);

    match p.peek() {
        K![true] => {
            p.ignore();
            Ok(hir::Lit::Bool(true))
        }
        K![false] => {
            p.ignore();
            Ok(hir::Lit::Bool(false))
        }
        K![-] | K![number] => {
            let neg = p.eat(K![-]).is_some();

            let lit = p.ast::<ast::LitNumber>()?;
            let n = lit.resolve(resolve_context!(cx.q))?;

            match (n.value, n.suffix) {
                (ast::NumberValue::Float(n), _) => {
                    let n = if neg { -n } else { n };
                    Ok(hir::Lit::Float(n))
                }
                (ast::NumberValue::Integer(int), Some(ast::NumberSuffix::Unsigned(_, size))) => {
                    let int = if neg { -int } else { int };

                    let Ok(n) = u64::try_from(int) else {
                        return Err(Error::new(lit, ErrorKind::BadUnsignedOutOfBounds { size }));
                    };

                    if !size.unsigned_in(n) {
                        return Err(Error::new(lit, ErrorKind::BadUnsignedOutOfBounds { size }));
                    }

                    Ok(hir::Lit::Unsigned(n))
                }
                (ast::NumberValue::Integer(int), Some(ast::NumberSuffix::Signed(_, size))) => {
                    let int = if neg { -int } else { int };

                    let Ok(n) = i64::try_from(int) else {
                        return Err(Error::new(lit, ErrorKind::BadSignedOutOfBounds { size }));
                    };

                    if !size.signed_in(n) {
                        return Err(Error::new(lit, ErrorKind::BadSignedOutOfBounds { size }));
                    }

                    Ok(hir::Lit::Signed(n))
                }
                (ast::NumberValue::Integer(int), _) => {
                    let int = if neg { -int } else { int };

                    let Ok(n) = i64::try_from(int) else {
                        return Err(Error::new(
                            lit,
                            ErrorKind::BadSignedOutOfBounds {
                                size: NumberSize::S64,
                            },
                        ));
                    };

                    Ok(hir::Lit::Signed(n))
                }
            }
        }
        K![byte] => {
            let lit = p.ast::<ast::LitByte>()?;
            let b = lit.resolve(resolve_context!(cx.q))?;
            Ok(hir::Lit::Unsigned(b as u64))
        }
        K![char] => {
            let lit = p.ast::<ast::LitChar>()?;
            let ch = lit.resolve(resolve_context!(cx.q))?;
            Ok(hir::Lit::Char(ch))
        }
        K![str] => {
            let lit = p.ast::<ast::LitStr>()?;

            let string = if cx.in_template {
                lit.resolve_template_string(resolve_context!(cx.q))?
            } else {
                lit.resolve_string(resolve_context!(cx.q))?
            };

            Ok(hir::Lit::Str(alloc_str!(string.as_ref())))
        }
        K![bytestr] => {
            let lit = p.ast::<ast::LitByteStr>()?;
            let bytes = lit.resolve(resolve_context!(cx.q))?;
            Ok(hir::Lit::ByteStr(alloc_bytes!(bytes.as_ref())))
        }
        _ => Err(p.expected(Lit)),
    }
}
