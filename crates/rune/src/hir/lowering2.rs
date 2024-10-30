use core::mem::replace;
use core::num::NonZero;
use core::ops::Neg;

use rust_alloc::rc::Rc;

use num::ToPrimitive;
use tracing::instrument_ast;

use crate::alloc::prelude::*;
use crate::alloc::{self, HashMap, HashSet};
use crate::ast::{self, Delimiter, Kind, Span, Spanned};
use crate::compile::{meta, Error, ErrorKind, ItemId, Result, WithSpan};
use crate::grammar::{
    classify, object_key, Ignore, MaybeNode, NodeClass, Remaining, Stream, StreamBuf, Tree,
};
use crate::hash::ParametersBuilder;
use crate::hir;
use crate::parse::{NonZeroId, Resolve};
use crate::query::{self, GenericsParameters, Named2, Named2Kind, Used};
use crate::runtime::{format, ConstValue, ConstValueKind, Inline, Type, TypeCheck};
use crate::Hash;

use super::{Ctxt, Needs};

use Kind::*;

/// Lower a bare function.
#[instrument_ast(span = p)]
pub(crate) fn bare<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ItemFn<'hir>> {
    let body = statements(cx, None, p)?;

    Ok(hir::ItemFn {
        span: p.span(),
        args: &[],
        body,
    })
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
                let stmt = hir::Stmt::Expr(&*alloc!(expr));
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
            Some(hir::Stmt::Expr(e)) => Some(e),
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

/// Lower a local.
#[instrument_ast(span = p)]
pub(crate) fn local<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::Local<'hir>> {
    // Note: expression needs to be assembled before pattern, otherwise the
    // expression will see declarations in the pattern.

    p.expect(K![let])?;
    let pat = p.expect(Pat)?;
    p.expect(K![=])?;
    let expr = p.expect(Expr)?;

    let expr = expr.parse(|p| self::expr(cx, p))?;
    let pat = pat.parse(|p| self::pat_binding(cx, p))?;

    Ok(hir::Local {
        span: p.span(),
        pat,
        expr,
    })
}

/// Lower an expression.
#[instrument_ast(span = p)]
pub(crate) fn expr<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::Expr<'hir>> {
    alloc_with!(cx, ast);

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

    let kind = p.pump()?.parse(|p| expr_inner(cx, p))?.into_kind(cx)?;

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

struct ExprInner<'hir, 'a> {
    span: Span,
    kind: ExprInnerKind<'hir, 'a>,
}

enum ExprInnerKind<'hir, 'a> {
    Kind(hir::ExprKind<'hir>),
    Path(StreamBuf<'a>),
}

impl<'hir, 'a> ExprInner<'hir, 'a> {
    fn into_call(self, cx: &mut Ctxt<'hir, '_, '_>, args: usize) -> Result<hir::Call<'hir>> {
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
                    }
                    | meta::Kind::Variant {
                        fields: meta::Fields::Empty,
                        ..
                    } => {
                        if args > 0 {
                            return Err(Error::new(
                                self.span,
                                ErrorKind::UnsupportedArgumentCount {
                                    expected: 0,
                                    actual: args,
                                },
                            ));
                        }
                    }
                    meta::Kind::Struct {
                        fields: meta::Fields::Unnamed(expected),
                        ..
                    }
                    | meta::Kind::Variant {
                        fields: meta::Fields::Unnamed(expected),
                        ..
                    } => {
                        if *expected != args {
                            return Err(Error::new(
                                self.span,
                                ErrorKind::UnsupportedArgumentCount {
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
                        let from =
                            cx.q.item_for("lowering constant function", named.item)
                                .with_span(self.span)?;

                        return Ok(hir::Call::ConstFn {
                            from_module: from.module,
                            from_item: from.item,
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

                        Ok(hir::Call::Associated {
                            target: alloc!(target),
                            hash,
                        })
                    }
                    kind => Ok(hir::Call::Expr {
                        expr: alloc!(hir::Expr {
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

                let kind = if !parameters.parameters.is_empty() {
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
#[instrument_ast(span = p)]
fn expr_expanded_macro<'hir>(
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
                let Some(n) = line.to_u64() else {
                    return Err(Error::new(&*p, ErrorKind::BadUnsignedOutOfBounds));
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

                    let Some(f) = arg.resolve(resolve_context!(cx.q))?.as_u32(false) else {
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

                    let Some(f) = arg.resolve(resolve_context!(cx.q))?.as_usize(false) else {
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

                    let Some(f) = arg.resolve(resolve_context!(cx.q))?.as_usize(false) else {
                        return Err(Error::unsupported(arg, "argument out-of-bounds"));
                    };

                    spec.precision = NonZero::new(f);
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
            spec,
            value: alloc!(expr),
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
            exprs: iter!(exprs),
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
    Ok(hir::ExprKind::Return(option!(expr)))
}

#[instrument_ast(span = p)]
fn expr_yield<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);
    p.expect(K![yield])?;
    let expr = p.eat(Expr).parse(|p| expr(cx, p))?;
    Ok(hir::ExprKind::Yield(option!(expr)))
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
        expr: option!(expr),
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
        items: iter!(items)
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
        items: iter!(items)
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
    Ok(hir::ExprKind::Group(alloc!(expr)))
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

    Ok(hir::ExprKind::Group(alloc!(expr)))
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
            assign,
            position: None,
        })?;

        comma = p.one(K![,]);
    }

    comma.at_most_one(cx)?;
    p.expect(K!['}'])?;

    let mut check_object_fields = |fields: &HashMap<_, meta::FieldMeta>, item: &crate::Item| {
        let mut fields = fields.try_clone()?;

        for assign in assignments.iter_mut() {
            let Some(meta) = fields.remove(assign.key.1) else {
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

        if let Some(field) = fields.into_keys().next() {
            return Err(Error::new(
                p.span(),
                ErrorKind::LitObjectMissingField {
                    field,
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
                    ..
                } => {
                    check_object_fields(&HashMap::new(), item)?;
                    hir::ExprObjectKind::EmptyStruct { hash: meta.hash }
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
                meta::Kind::Variant {
                    fields: meta::Fields::Named(st),
                    ..
                } => {
                    check_object_fields(&st.fields, item)?;
                    hir::ExprObjectKind::StructVariant { hash: meta.hash }
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

    let expr = p.pump()?.parse(|p| expr_only(cx, p))?;

    Ok(hir::ExprKind::Unary(alloc!(hir::ExprUnary { op, expr })))
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
            lhs: hir::Expr {
                span: lhs_span,
                kind: lhs
            },
            op,
            rhs: hir::Expr {
                span: rhs_span,
                kind: rhs
            },
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
                Some(&*alloc!(expr))
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
                body,
                drop: iter!(layer.into_drop_order()),
            })?;

            Ok(is_block)
        })?;

        comma = p.remaining(cx, K![,])?;
    }

    comma.at_most_one(cx)?;
    p.expect(K!['}'])?;

    Ok(hir::ExprKind::Match(alloc!(hir::ExprMatch {
        expr: alloc!(expr),
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
    let mut default = None::<hir::Expr>;

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

                    if let Some(existing) = &default {
                        cx.error(Error::new(
                            &default_token,
                            ErrorKind::DuplicateSelectDefault {
                                existing: existing.span(),
                            },
                        ))?;
                    } else {
                        default = Some(body);
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
                        body,
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
        exprs: iter!(exprs),
        branches: iter!(branches),
        default: option!(default),
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
        iter,
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
        Expr => Ok(hir::Condition::Expr(alloc!(expr(cx, p)?))),
        _ => Err(p.expected(Condition)),
    }
}

#[instrument_ast(span = p)]
fn expr_let<'hir>(cx: &mut Ctxt<'hir, '_, '_>, p: &mut Stream<'_>) -> Result<hir::ExprLet<'hir>> {
    p.expect(K![let])?;
    let pat = p.expect(Pat)?;
    p.expect(K![=])?;
    let expr = p.expect(Expr)?;

    let expr = expr.parse(|p| self::expr(cx, p))?;
    let pat = pat.parse(|p| self::pat_binding(cx, p))?;

    Ok(hir::ExprLet { pat, expr })
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
    let body = alloc!(body);

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
        args: iter!(args),
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
                return Err(Error::new(
                    number,
                    ErrorKind::UnsupportedTupleIndex { number: index },
                ));
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
        expr: hir::Expr { span, kind },
        expr_field,
    }));

    Ok(kind)
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
        target: hir::Expr { span, kind },
        index,
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

    Ok(hir::ExprKind::Await(alloc!(hir::Expr { span, kind })))
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
    Ok(hir::ExprKind::Try(alloc!(hir::Expr { span, kind })))
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
                fields: meta::Fields::Empty,
                ..
            }
            | meta::Kind::Variant {
                fields: meta::Fields::Empty,
                ..
            } => Ok(hir::ExprKind::Call(alloc!(hir::ExprCall {
                call: hir::Call::Meta { hash: meta.hash },
                args: &[],
            }))),
            meta::Kind::Variant {
                fields: meta::Fields::Unnamed(0),
                ..
            }
            | meta::Kind::Struct {
                fields: meta::Fields::Unnamed(0),
                ..
            } => Ok(hir::ExprKind::Call(alloc!(hir::ExprCall {
                call: hir::Call::Meta { hash: meta.hash },
                args: &[],
            }))),
            meta::Kind::Struct {
                fields: meta::Fields::Unnamed(..),
                ..
            } => Ok(hir::ExprKind::Fn(meta.hash)),
            meta::Kind::Variant {
                fields: meta::Fields::Unnamed(..),
                ..
            } => Ok(hir::ExprKind::Fn(meta.hash)),
            meta::Kind::Function { .. } => Ok(hir::ExprKind::Fn(meta.hash)),
            meta::Kind::Const { .. } => Ok(hir::ExprKind::Const(meta.hash)),
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
    let pat = p.pump()?.parse(|p| pat_inner(cx, p, self_value))?;
    let names = iter!(cx.pattern_bindings.drain(..));
    Ok(hir::PatBinding { pat, names })
}

/// Parses a pattern inside of a binding.
fn pat<'hir>(cx: &mut Ctxt<'hir, '_, '_>, p: &mut Stream<'_>) -> Result<hir::Pat<'hir>> {
    p.pump()?.parse(|p| pat_inner(cx, p, false))
}

fn pat_inner<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
    self_value: bool,
) -> Result<hir::Pat<'hir>> {
    alloc_with!(cx, p);

    match p.kind() {
        Lit => pat_lit(cx, p),
        PatIgnore => Ok(hir::Pat {
            span: p.expect(K![_])?.span(),
            kind: hir::PatKind::Ignore,
        }),
        IndexedPath(..) => pat_path(cx, p, self_value),
        PatTuple => pat_tuple(cx, p),
        PatObject => pat_object(cx, p),
        PatArray => pat_array(cx, p),
        _ => Err(p.expected(Pat)),
    }
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
                _ => {
                    if let Some((0, kind)) = tuple_match_for(cx, &meta) {
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

    let expr = alloc!(hir::Expr {
        span: p.span(),
        kind: hir::ExprKind::Lit(lit),
    });

    Ok(hir::Pat {
        span: p.span(),
        kind: hir::PatKind::Lit(expr),
    })
}

#[instrument_ast(span = p)]
fn pat_tuple<'hir>(cx: &mut Ctxt<'hir, '_, '_>, p: &mut Stream<'_>) -> Result<hir::Pat<'hir>> {
    alloc_with!(cx, p);

    let path = p.eat_matching(|kind| matches!(kind, IndexedPath(..)));

    p.expect(K!['('])?;

    let mut items = Vec::new();
    let mut comma = Remaining::default();

    while let Some(pat) = p.eat(Pat).parse(|p| self::pat(cx, p))? {
        comma.exactly_one(cx)?;
        items.try_push(pat)?;
        comma = p.one(K![,]);
    }

    let is_open = if p.eat(K![..]).is_some() {
        comma.exactly_one(cx)?;
        true
    } else {
        comma.at_most_one(cx)?;
        false
    };

    p.expect(K![')'])?;

    let items = iter!(items);

    let kind = if let MaybeNode::Some(path) = path {
        let (named, span) = path.parse(|p| Ok((cx.q.convert_path2(p)?, p.span())))?;
        let parameters = generics_parameters(cx, &named)?;
        let meta = cx.lookup_meta(&span, named.item, parameters)?;

        // Treat the current meta as a tuple and get the number of arguments it
        // should receive and the type check that applies to it.
        let Some((args, kind)) = tuple_match_for(cx, &meta) else {
            return Err(Error::expected_meta(
                span,
                meta.info(cx.q.pool)?,
                "type that can be used in a tuple pattern",
            ));
        };

        if !(args == items.len() || items.len() < args && is_open) {
            cx.error(Error::new(
                span,
                ErrorKind::UnsupportedArgumentCount {
                    expected: args,
                    actual: items.len(),
                },
            ))?;
        }

        kind
    } else {
        hir::PatSequenceKind::Anonymous {
            type_check: TypeCheck::Tuple,
            count: items.len(),
            is_open,
        }
    };

    Ok(hir::Pat {
        span: p.span(),
        kind: hir::PatKind::Sequence(alloc!(hir::PatSequence { kind, items })),
    })
}

#[instrument_ast(span = p)]
fn pat_object<'hir>(cx: &mut Ctxt<'hir, '_, '_>, p: &mut Stream<'_>) -> Result<hir::Pat<'hir>> {
    alloc_with!(cx, p);

    let key = p.pump()?;

    let path = match key.kind() {
        AnonymousObjectKey => None,
        IndexedPath(..) => Some(key),
        _ => {
            return Err(p.expected_peek("object kind"));
        }
    };

    p.expect(K!['{'])?;

    let mut bindings = Vec::new();
    let mut comma = Remaining::default();
    let mut keys_dup = HashMap::new();

    while matches!(p.peek(), object_key!()) {
        comma.exactly_one(cx)?;

        let (span, key) = match p.peek() {
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
        };

        if let Some(_existing) = keys_dup.try_insert(key, span)? {
            return Err(Error::new(
                span,
                ErrorKind::DuplicateObjectKey {
                    #[cfg(feature = "emit")]
                    existing: _existing.span(),
                    #[cfg(feature = "emit")]
                    object: p.span(),
                },
            ));
        }

        if p.eat(K![:]).is_some() {
            let pat = p.expect(Pat)?.parse(|p| pat(cx, p))?;
            bindings.try_push(hir::Binding::Binding(p.span(), key, alloc!(pat)))?;
        } else {
            let id = cx.scopes.define(hir::Name::Str(key), &*p)?;
            cx.pattern_bindings.try_push(id)?;
            bindings.try_push(hir::Binding::Ident(p.span(), key, id))?;
        }

        comma = p.one(K![,]);
    }

    let is_open = if p.eat(K![..]).is_some() {
        comma.exactly_one(cx)?;
        true
    } else {
        comma.at_most_one(cx)?;
        false
    };

    p.expect(K!['}'])?;

    let kind = match path {
        Some(path) => {
            let (named, span) = path.parse(|p| Ok((cx.q.convert_path2(p)?, p.span())))?;
            let parameters = generics_parameters(cx, &named)?;
            let meta = cx.lookup_meta(&span, named.item, parameters)?;

            let Some((mut fields, kind)) =
                struct_match_for(cx, &meta, is_open && bindings.is_empty())?
            else {
                return Err(Error::expected_meta(
                    span,
                    meta.info(cx.q.pool)?,
                    "type that can be used in a struct pattern",
                ));
            };

            for binding in bindings.iter() {
                if !fields.remove(binding.key()) {
                    return Err(Error::new(
                        span,
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
                    p.span(),
                    ErrorKind::PatternMissingFields {
                        item: cx.q.pool.item(meta.item_meta.item).try_to_owned()?,
                        #[cfg(feature = "emit")]
                        fields,
                    },
                ));
            }

            kind
        }
        None => hir::PatSequenceKind::Anonymous {
            type_check: TypeCheck::Object,
            count: bindings.len(),
            is_open,
        },
    };

    let bindings = iter!(bindings);

    Ok(hir::Pat {
        span: p.span(),
        kind: hir::PatKind::Object(alloc!(hir::PatObject { kind, bindings })),
    })
}

#[instrument_ast(span = p)]
fn pat_array<'hir>(cx: &mut Ctxt<'hir, '_, '_>, p: &mut Stream<'_>) -> Result<hir::Pat<'hir>> {
    alloc_with!(cx, p);

    p.expect(K!['['])?;

    let mut items = Vec::new();
    let mut comma = Remaining::default();

    while let Some(pat) = p.eat(Pat).parse(|p| self::pat(cx, p))? {
        comma.exactly_one(cx)?;
        items.try_push(pat)?;
        comma = p.one(K![,]);
    }

    let is_open = if p.eat(K![..]).is_some() {
        comma.exactly_one(cx)?;
        true
    } else {
        comma.at_most_one(cx)?;
        false
    };

    p.expect(K![']'])?;

    let items = iter!(items);

    let kind = hir::PatSequenceKind::Anonymous {
        type_check: TypeCheck::Vec,
        count: items.len(),
        is_open,
    };

    Ok(hir::Pat {
        span: p.span(),
        kind: hir::PatKind::Sequence(alloc!(hir::PatSequence { kind, items })),
    })
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
        let lit = match *const_value.as_kind() {
            ConstValueKind::Inline(value) => match value {
                Inline::Unit => {
                    break 'kind hir::PatKind::Sequence(alloc!(hir::PatSequence {
                        kind: hir::PatSequenceKind::Anonymous {
                            type_check: TypeCheck::Unit,
                            count: 0,
                            is_open: false,
                        },
                        items: &[],
                    }));
                }
                Inline::Bool(b) => hir::Lit::Bool(b),
                Inline::Byte(b) => hir::Lit::Byte(b),
                Inline::Char(ch) => hir::Lit::Char(ch),
                Inline::Unsigned(value) => hir::Lit::Unsigned(value),
                Inline::Signed(value) => hir::Lit::Signed(value),
                _ => return Err(Error::msg(span, "Unsupported constant value in pattern")),
            },
            ConstValueKind::String(ref string) => hir::Lit::Str(alloc_str!(string.as_ref())),
            ConstValueKind::Bytes(ref bytes) => hir::Lit::ByteStr(alloc_bytes!(bytes.as_ref())),
            ConstValueKind::Vec(ref items) => {
                let items = iter!(items.iter(), items.len(), |value| pat_const_value(
                    cx, value, span
                )?);

                break 'kind hir::PatKind::Sequence(alloc!(hir::PatSequence {
                    kind: hir::PatSequenceKind::Anonymous {
                        type_check: TypeCheck::Vec,
                        count: items.len(),
                        is_open: false,
                    },
                    items,
                }));
            }
            ConstValueKind::Tuple(ref items) => {
                let items = iter!(items.iter(), items.len(), |value| pat_const_value(
                    cx, value, span
                )?);

                break 'kind hir::PatKind::Sequence(alloc!(hir::PatSequence {
                    kind: hir::PatSequenceKind::Anonymous {
                        type_check: TypeCheck::Vec,
                        count: items.len(),
                        is_open: false,
                    },
                    items,
                }));
            }
            ConstValueKind::Object(ref fields) => {
                let bindings = iter!(fields.iter(), fields.len(), |(key, value)| {
                    let pat = alloc!(pat_const_value(cx, value, span)?);

                    hir::Binding::Binding(span.span(), alloc_str!(key.as_ref()), pat)
                });

                break 'kind hir::PatKind::Object(alloc!(hir::PatObject {
                    kind: hir::PatSequenceKind::Anonymous {
                        type_check: TypeCheck::Object,
                        count: bindings.len(),
                        is_open: false,
                    },
                    bindings,
                }));
            }
            _ => {
                return Err(Error::msg(span, "Unsupported constant value in pattern"));
            }
        };

        hir::PatKind::Lit(alloc!(hir::Expr {
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
    cx: &Ctxt<'_, '_, '_>,
    meta: &meta::Meta,
    open: bool,
) -> alloc::Result<Option<(HashSet<Box<str>>, hir::PatSequenceKind)>> {
    let (fields, kind) = match &meta.kind {
        meta::Kind::Struct { fields, .. } => {
            (fields, hir::PatSequenceKind::Type { hash: meta.hash })
        }
        meta::Kind::Variant {
            enum_hash,
            index,
            fields,
            ..
        } => {
            let kind = if let Some(type_check) = cx.q.context.type_check_for(meta.hash) {
                hir::PatSequenceKind::BuiltInVariant { type_check }
            } else {
                hir::PatSequenceKind::Variant {
                    variant_hash: meta.hash,
                    enum_hash: *enum_hash,
                    index: *index,
                }
            };

            (fields, kind)
        }
        _ => {
            return Ok(None);
        }
    };

    let fields = match fields {
        meta::Fields::Unnamed(0) if open => HashSet::new(),
        meta::Fields::Empty if open => HashSet::new(),
        meta::Fields::Named(st) => st
            .fields
            .keys()
            .try_cloned()
            .try_collect::<alloc::Result<_>>()??,
        _ => return Ok(None),
    };

    Ok(Some((fields, kind)))
}

fn tuple_match_for(
    cx: &Ctxt<'_, '_, '_>,
    meta: &meta::Meta,
) -> Option<(usize, hir::PatSequenceKind)> {
    Some(match &meta.kind {
        meta::Kind::Struct {
            fields: meta::Fields::Empty,
            ..
        } => (0, hir::PatSequenceKind::Type { hash: meta.hash }),
        meta::Kind::Struct {
            fields: meta::Fields::Unnamed(args),
            ..
        } => (*args, hir::PatSequenceKind::Type { hash: meta.hash }),
        meta::Kind::Variant {
            enum_hash,
            index,
            fields,
            ..
        } => {
            let args = match fields {
                meta::Fields::Unnamed(args) => *args,
                meta::Fields::Empty => 0,
                _ => return None,
            };

            let kind = if let Some(type_check) = cx.q.context.type_check_for(meta.hash) {
                hir::PatSequenceKind::BuiltInVariant { type_check }
            } else {
                hir::PatSequenceKind::Variant {
                    variant_hash: meta.hash,
                    enum_hash: *enum_hash,
                    index: *index,
                }
            };

            (args, kind)
        }
        _ => return None,
    })
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
                (ast::NumberValue::Integer(int), Some(ast::NumberSuffix::Byte(..))) => {
                    if neg {
                        return Err(Error::new(lit, ErrorKind::BadByteNeg));
                    }

                    let Some(n) = int.to_u8() else {
                        return Err(Error::new(lit, ErrorKind::BadByteOutOfBounds));
                    };

                    Ok(hir::Lit::Byte(n))
                }
                (ast::NumberValue::Integer(int), Some(ast::NumberSuffix::Unsigned(..))) => {
                    let int = if neg { int.neg() } else { int };

                    let Some(n) = int.to_u64() else {
                        return Err(Error::new(lit, ErrorKind::BadUnsignedOutOfBounds));
                    };

                    Ok(hir::Lit::Unsigned(n))
                }
                (ast::NumberValue::Integer(int), _) => {
                    let int = if neg { int.neg() } else { int };

                    let Some(n) = int.to_i64() else {
                        return Err(Error::new(lit, ErrorKind::BadSignedOutOfBounds));
                    };

                    Ok(hir::Lit::Signed(n))
                }
            }
        }
        K![byte] => {
            let lit = p.ast::<ast::LitByte>()?;
            let b = lit.resolve(resolve_context!(cx.q))?;
            Ok(hir::Lit::Byte(b))
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
