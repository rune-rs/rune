use core::mem::{replace, take};

use num::ToPrimitive;
use tracing::instrument_ast;

use crate::alloc::prelude::*;
use crate::ast::{self, Kind, Span, Spanned};
use crate::compile::{meta, Error, ErrorKind, Result, WithSpan};
use crate::grammar::{Node, Remaining, Stream};
use crate::hash::ParametersBuilder;
use crate::hir;
use crate::parse::Resolve;
use crate::query::{GenericsParameters, Named2, Named2Kind};
use crate::runtime::{ConstValue, Type, TypeCheck};
use crate::Hash;

use super::{Ctxt, Needs};

use Kind::*;

/// Lower a function item.
#[instrument_ast(span = p)]
pub(crate) fn item_fn<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
    is_instance: bool,
) -> Result<hir::ItemFn<'hir>> {
    alloc_with!(cx, p);

    p.remaining(cx, Attribute)?.ignore(cx)?;
    p.try_pump(Modifiers)?;
    p.expect(K![fn])?;
    p.ast::<ast::Ident>()?;

    let mut args = Vec::new();

    p.expect(FnArgs)?.parse(|p| {
        p.expect(K!['('])?;

        let mut comma = Remaining::default();

        while let Some(pat) = p.try_pump(Pat)? {
            comma.exactly_one(cx)?;
            let pat = pat.parse(|p| pat_binding(cx, p, is_instance))?;
            args.try_push(hir::FnArg::Pat(alloc!(pat)))?;
            comma = p.one(K![,])?;
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

#[instrument_ast(span = p)]
fn block<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    label: Option<&ast::Label>,
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
    label: Option<&ast::Label>,
    p: &mut Stream<'_>,
) -> Result<hir::Block<'hir>> {
    alloc_with!(cx, p);

    let label = match label {
        Some(label) => Some(alloc_str!(label.resolve(resolve_context!(cx.q))?)),
        None => None,
    };

    cx.scopes.push(label)?;

    let at = cx.statements.len();

    let mut value = None;

    while let Some(node) = p.next() {
        let last = match node.kind() {
            Local => {
                let stmt = hir::Stmt::Local(alloc!(node.parse(|p| local(cx, p))?));
                cx.statement_buffer.try_push(stmt)?;
                value.take()
            }
            Expr => {
                let expr = node.parse(|p| expr(cx, p))?;

                if let Some(stmt) = value.replace(&*alloc!(expr)).map(hir::Stmt::Expr) {
                    cx.statement_buffer.try_push(stmt)?;
                }

                None
            }
            Item => {
                continue;
            }
            _ => {
                return Err(node.expected("an expression or local"));
            }
        };

        if let Some(last) = last {
            cx.statements
                .try_push(hir::Stmt::Expr(last))
                .with_span(&*p)?;
        }

        for stmt in cx.statement_buffer.drain(..) {
            cx.statements.try_push(stmt).with_span(&*p)?;
        }

        p.remaining(cx, K![;])?.ignore(cx)?;
    }

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
    let pat = pat.parse(|p| pat_binding(cx, p, false))?;

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

    let kind = p.pump()?.parse(|p| expr_inner(cx, p))?;

    Ok(hir::Expr {
        span: p.span(),
        kind,
    })
}

#[instrument_ast(span = p)]
fn expr_inner<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    let in_path = take(&mut cx.in_path);

    match p.kind() {
        ExprPath => {
            let node = p.pump()?;
            expr_path(cx, node, in_path)
        }
        ExprChain => expr_chain(cx, p),
        ExprBinary => expr_binary(cx, p),
        ExprLit => {
            let lit = lit(cx, p)?;
            Ok(hir::ExprKind::Lit(lit))
        }
        _ => Err(p.expected(Expr)),
    }
}

/// Lower the given path.
#[instrument_ast(span = node)]
fn expr_path<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    node: Node<'_>,
    in_path: bool,
) -> Result<hir::ExprKind<'hir>> {
    fn is_self(node: &Node<'_>) -> bool {
        node.as_array([PathSelf]).is_some()
    }

    fn try_as_ident(node: &Node<'_>) -> Option<ast::Ident> {
        let [path_ident] = node.as_array([PathIdent])?;
        let [ident] = path_ident.array()?;
        ident.ast().ok()
    }

    alloc_with!(cx, &node);

    if is_self(&node) {
        let Some((id, _)) = cx.scopes.get(hir::Name::SelfValue)? else {
            return Err(Error::new(&node, ErrorKind::MissingSelf));
        };

        return Ok(hir::ExprKind::Variable(id));
    }

    if let Needs::Value = cx.needs {
        if let Some(name) = try_as_ident(&node) {
            let name = alloc_str!(name.resolve(resolve_context!(cx.q))?);

            if let Some((name, _)) = cx.scopes.get(hir::Name::Str(name))? {
                return Ok(hir::ExprKind::Variable(name));
            }
        }
    }

    // Caller has indicated that if they can't have a variable, they do indeed
    // want to treat it as a path.
    if in_path {
        return Ok(hir::ExprKind::Path);
    }

    let named = node.clone().parse(|p| cx.q.convert_path2(p))?;
    let parameters = generics_parameters(cx, &named)?;

    if let Some(meta) = cx.try_lookup_meta(&node, named.item, &parameters)? {
        return expr_path_meta(cx, &meta, &node);
    }

    if let (Needs::Value, Named2Kind::Ident(local)) = (cx.needs, named.kind) {
        let local = local.resolve(resolve_context!(cx.q))?;

        // light heuristics, treat it as a type error in case the first
        // character is uppercase.
        if !local.starts_with(char::is_uppercase) {
            return Err(Error::new(
                &node,
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

    Err(Error::new(&node, kind))
}

#[instrument_ast(span = p)]
fn expr_chain<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    let node = p.pump()?;

    let in_path = replace(&mut cx.in_path, true);
    let (mut kind, mut span) = node.clone().parse(|p| Ok((expr_inner(cx, p)?, p.span())))?;
    let mut outer = Some(node);
    cx.in_path = in_path;

    for node in p.by_ref() {
        let outer = outer.take();

        match node.kind() {
            ExprCall => {
                let span = replace(&mut span, node.span());
                kind = node.parse(|p| expr_call(cx, p, span, kind, outer))?;
            }
            ExprField => {
                let span = replace(&mut span, node.span());
                kind = node.parse(|p| expr_field(cx, p, span, kind))?;
            }
            _ => {
                return Err(node.expected(ExprChain));
            }
        }
    }

    Ok(kind)
}

#[instrument_ast(span = p)]
fn expr_binary<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    let (mut lhs, mut lhs_span) = p.pump()?.parse(|p| Ok((expr_inner(cx, p)?, p.span())))?;

    while !p.is_eof() {
        let node = p.expect(ExprOperator)?;

        let Some(ops) = node.fixed_vec::<2, _>(Node::into_token) else {
            return Err(node.expected(ExprOperator));
        };

        let Some(op) = ast::BinOp::from_slice(&ops) else {
            return Err(node.expected(ExprOperator));
        };

        let rhs_needs = match op {
            ast::BinOp::As(..) | ast::BinOp::Is(..) | ast::BinOp::IsNot(..) => Needs::Type,
            _ => Needs::Value,
        };

        let needs = replace(&mut cx.needs, rhs_needs);
        let (rhs, rhs_span) = p.pump()?.parse(|p| Ok((expr_inner(cx, p)?, p.span())))?;
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
fn expr_call<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
    span: Span,
    kind: hir::ExprKind<'hir>,
    outer: Option<Node<'_>>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    p.expect(K!['('])?;

    let mut comma = Remaining::default();
    let mut args = Vec::new();

    while let Some(node) = p.try_pump(Expr)? {
        comma.exactly_one(cx)?;
        let expr = node.parse(|p| expr(cx, p))?;
        args.try_push(expr)?;
        comma = p.one(K![,])?;
    }

    comma.at_most_one(cx)?;
    p.expect(K![')'])?;

    let call = 'ok: {
        match kind {
            hir::ExprKind::Variable(name) => hir::Call::Var { name },
            hir::ExprKind::Path => {
                let Some(outer) = outer else {
                    return Err(Error::msg(span, "Expected path"));
                };

                let named = outer.clone().parse(|p| cx.q.convert_path2(p))?;
                let parameters = generics_parameters(cx, &named)?;

                let meta = cx.lookup_meta(&span, named.item, parameters)?;
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
                        if !args.is_empty() {
                            return Err(Error::new(
                                p,
                                ErrorKind::UnsupportedArgumentCount {
                                    expected: 0,
                                    actual: args.len(),
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
                        if *expected != args.len() {
                            return Err(Error::new(
                                p,
                                ErrorKind::UnsupportedArgumentCount {
                                    expected: *expected,
                                    actual: args.len(),
                                },
                            ));
                        }

                        if *expected == 0 {
                            cx.q.diagnostics.remove_tuple_call_parens(
                                cx.source_id,
                                p,
                                &outer,
                                None,
                            )?;
                        }
                    }
                    meta::Kind::Function { .. } => {
                        if let Some(message) = cx.q.lookup_deprecation(meta.hash) {
                            cx.q.diagnostics.used_deprecated(
                                cx.source_id,
                                &outer,
                                None,
                                message.try_into()?,
                            )?;
                        };
                    }
                    meta::Kind::ConstFn => {
                        let from = cx.q.item_for(named.item).with_span(&outer)?;

                        break 'ok hir::Call::ConstFn {
                            from_module: from.module,
                            from_item: from.item,
                            id: meta.item_meta.item,
                        };
                    }
                    _ => {
                        return Err(Error::expected_meta(
                            &outer,
                            meta.info(cx.q.pool)?,
                            "something that can be called as a function",
                        ));
                    }
                };

                hir::Call::Meta { hash: meta.hash }
            }
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

                hir::Call::Associated {
                    target: alloc!(target),
                    hash,
                }
            }
            _ => hir::Call::Expr {
                expr: alloc!(hir::Expr { span, kind }),
            },
        }
    };

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
    span: Span,
    kind: hir::ExprKind<'hir>,
) -> Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, p);

    p.expect(K![.])?;

    let expr_field = match p.peek() {
        K![number] => {
            let number = p.ast::<ast::LitNumber>()?;
            let index = number.resolve(resolve_context!(cx.q))?;

            let Some(index) = index.as_tuple_index() else {
                return Err(Error::new(
                    &number,
                    ErrorKind::UnsupportedTupleIndex { number: index },
                ));
            };

            hir::ExprField::Index(index)
        }
        IndexedPath(..) => p.pump()?.parse(|p| match p.peek() {
            PathIdent => {
                let base = p.pump()?.parse(|p| p.ast::<ast::Ident>())?;
                let base = base.resolve(resolve_context!(cx.q))?;
                let base = alloc_str!(base);
                Ok(hir::ExprField::Ident(base))
            }
            PathFull => p.pump()?.parse(|p| {
                let base = p.ast::<ast::Ident>()?;
                let base = base.resolve(resolve_context!(cx.q))?;
                let base = alloc_str!(base);

                if p.try_pump(K![::])?.is_some() {
                    let hash = p
                        .expect(PathGenerics)?
                        .parse(|p| generic_arguments(cx, p))?;
                    Ok(hir::ExprField::IdentGenerics(base, hash))
                } else {
                    Ok(hir::ExprField::Ident(base))
                }
            }),
            _ => {
                return Err(p.expected(Path));
            }
        })?,
        _ => {
            return Err(p.expected(ExprField));
        }
    };

    let kind = hir::ExprKind::FieldAccess(alloc!(hir::ExprFieldAccess {
        expr: hir::Expr { span, kind },
        expr_field,
    }));

    Ok(kind)
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
    self_value: bool,
) -> Result<hir::PatBinding<'hir>> {
    alloc_with!(cx, p);

    let pat = pat(cx, p, self_value)?;
    let names = iter!(cx.pattern_bindings.drain(..));

    Ok(hir::PatBinding { pat, names })
}

fn pat<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    p: &mut Stream<'_>,
    self_value: bool,
) -> Result<hir::Pat<'hir>> {
    alloc_with!(cx, p);

    let n = p.pump()?;

    let pat = match n.kind() {
        K![_] => hir::Pat {
            span: p.span(),
            kind: hir::PatKind::Ignore,
        },
        IndexedPath(..) => pat_path(cx, n, self_value)?,
        _ => {
            return Err(p.expected(Pat));
        }
    };

    Ok(pat)
}

fn pat_path<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    node: Node<'_>,
    self_value: bool,
) -> Result<hir::Pat<'hir>> {
    alloc_with!(cx, &node);

    let named = node.clone().parse(|p| cx.q.convert_path2(p))?;
    let parameters = generics_parameters(cx, &named)?;

    let path = 'path: {
        if let Some(meta) = cx.try_lookup_meta(&node, named.item, &parameters)? {
            match meta.kind {
                meta::Kind::Const => {
                    let Some(const_value) = cx.q.get_const_value(meta.hash) else {
                        return Err(Error::msg(
                            &node,
                            try_format!("Missing constant for hash {}", meta.hash),
                        ));
                    };

                    let const_value = const_value.try_clone().with_span(&node)?;
                    return pat_const_value(cx, &const_value, &node);
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
                let name = cx.scopes.define(hir::Name::Str(name), &node)?;
                cx.pattern_bindings.try_push(name)?;
                break 'path hir::PatPathKind::Ident(name);
            }
            _ => {
                return Err(Error::new(&node, ErrorKind::UnsupportedBinding));
            }
        }
    };

    let kind = hir::PatKind::Path(alloc!(path));

    Ok(hir::Pat {
        span: node.span(),
        kind,
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

        let node = p.pump()?;
        let span = node.span();

        let hir::ExprKind::Type(ty) = expr_path(cx, node, false)? else {
            return Err(Error::new(span, ErrorKind::UnsupportedGenerics));
        };

        builder.add(ty.into_hash());
        comma = p.one(K![,])?;
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
        let lit = match *const_value {
            ConstValue::Bool(b) => hir::Lit::Bool(b),
            ConstValue::Byte(b) => hir::Lit::Byte(b),
            ConstValue::Char(ch) => hir::Lit::Char(ch),
            ConstValue::String(ref string) => hir::Lit::Str(alloc_str!(string.as_ref())),
            ConstValue::Bytes(ref bytes) => hir::Lit::ByteStr(alloc_bytes!(bytes.as_ref())),
            ConstValue::Integer(integer) => hir::Lit::Integer(integer),
            ConstValue::Vec(ref items) => {
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
            ConstValue::Unit => {
                break 'kind hir::PatKind::Sequence(alloc!(hir::PatSequence {
                    kind: hir::PatSequenceKind::Anonymous {
                        type_check: TypeCheck::Unit,
                        count: 0,
                        is_open: false,
                    },
                    items: &[],
                }));
            }
            ConstValue::Tuple(ref items) => {
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
            ConstValue::Object(ref fields) => {
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

    let node = p.pump()?;

    match node.kind() {
        K![true] => Ok(hir::Lit::Bool(true)),
        K![false] => Ok(hir::Lit::Bool(false)),
        K![number] => {
            let lit = node.ast::<ast::LitNumber>()?;
            let n = lit.resolve(resolve_context!(cx.q))?;

            match (n.value, n.suffix) {
                (ast::NumberValue::Float(n), _) => Ok(hir::Lit::Float(n)),
                (ast::NumberValue::Integer(int), Some(ast::NumberSuffix::Byte(..))) => {
                    let Some(n) = int.to_u8() else {
                        return Err(Error::new(lit, ErrorKind::BadNumberOutOfBounds));
                    };

                    Ok(hir::Lit::Byte(n))
                }
                (ast::NumberValue::Integer(int), _) => {
                    let Some(n) = int.to_i64() else {
                        return Err(Error::new(lit, ErrorKind::BadNumberOutOfBounds));
                    };

                    Ok(hir::Lit::Integer(n))
                }
            }
        }
        K![byte] => {
            let lit = node.ast::<ast::LitByte>()?;
            let b = lit.resolve(resolve_context!(cx.q))?;
            Ok(hir::Lit::Byte(b))
        }
        K![char] => {
            let lit = node.ast::<ast::LitChar>()?;
            let ch = lit.resolve(resolve_context!(cx.q))?;
            Ok(hir::Lit::Char(ch))
        }
        K![str] => {
            let lit = node.ast::<ast::LitStr>()?;

            let string = if cx.in_template {
                lit.resolve_template_string(resolve_context!(cx.q))?
            } else {
                lit.resolve_string(resolve_context!(cx.q))?
            };

            Ok(hir::Lit::Str(alloc_str!(string.as_ref())))
        }
        K![bytestr] => {
            let lit = node.ast::<ast::LitByteStr>()?;
            let bytes = lit.resolve(resolve_context!(cx.q))?;
            Ok(hir::Lit::ByteStr(alloc_bytes!(bytes.as_ref())))
        }
        _ => Err(node.expected(ExprLit)),
    }
}

/*
/// Lower an empty function.
#[instrument_ast(span = span)]
pub(crate) fn empty_fn<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::EmptyBlock,
    span: &dyn Spanned,
) -> compile::Result<hir::ItemFn<'hir>> {
    Ok(hir::ItemFn {
        span: span.span(),
        args: &[],
        body: statements(cx, None, &ast.statements, span)?,
    })
}

/// Assemble a closure expression.
#[instrument_ast(span = ast)]
fn expr_call_closure<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::ExprClosure,
) -> compile::Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, ast);

    let item = cx.q.item_for(ast.id).with_span(ast)?;

    let Some(meta) = cx.q.query_meta(ast, item.item, Default::default())? else {
        return Err(Error::new(
            ast,
            ErrorKind::MissingItem {
                item: cx.q.pool.item(item.item).try_to_owned()?,
            },
        ));
    };

    let meta::Kind::Closure { call, do_move, .. } = meta.kind else {
        return Err(Error::expected_meta(
            ast,
            meta.info(cx.q.pool)?,
            "a closure",
        ));
    };

    tracing::trace!("queuing closure build entry");

    cx.scopes.push_captures()?;

    let args = iter!(ast.args.as_slice(), |(arg, _)| fn_arg(cx, arg)?);
    let body = alloc!(expr(cx, &ast.body)?);

    let layer = cx.scopes.pop().with_span(&ast.body)?;

    cx.q.set_used(&meta.item_meta)?;

    let captures = &*iter!(layer.captures().map(|(_, id)| id));

    let Some(queue) = cx.secondary_builds.as_mut() else {
        return Err(Error::new(ast, ErrorKind::ClosureInConst));
    };

    queue.try_push(SecondaryBuildEntry {
        item_meta: meta.item_meta,
        build: SecondaryBuild::Closure(Closure {
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

#[inline]
pub(crate) fn block<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    label: Option<&(ast::Label, T![:])>,
    ast: &ast::Block,
) -> compile::Result<hir::Block<'hir>> {
    statements(cx, label, &ast.statements, ast)
}

#[instrument_ast(span = span)]
fn statements<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    label: Option<&(ast::Label, T![:])>,
    statements: &[ast::Stmt],
    span: &dyn Spanned,
) -> compile::Result<hir::Block<'hir>> {
    alloc_with!(cx, span);

    let label = match label {
        Some((label, _)) => Some(alloc_str!(label.resolve(resolve_context!(cx.q))?)),
        None => None,
    };

    cx.scopes.push(label)?;

    let at = cx.statements.len();

    let mut value = None;

    for ast in statements {
        let last = match ast {
            ast::Stmt::Local(ast) => {
                let depacked = if ast.attributes.is_empty() && cx.q.options.lowering > 0 {
                    unpack_locals(cx, &ast.pat, &ast.expr)?
                } else {
                    false
                };

                if !depacked {
                    let stmt = hir::Stmt::Local(alloc!(local(cx, ast)?));
                    cx.statement_buffer.try_push(stmt)?;
                }

                value.take()
            }
            ast::Stmt::Expr(ast) => {
                if let Some(stmt) = value.replace(&*alloc!(expr(cx, ast)?)).map(hir::Stmt::Expr) {
                    cx.statement_buffer.try_push(stmt)?;
                }

                None
            }
            ast::Stmt::Semi(ast) => {
                let stmt = hir::Stmt::Expr(alloc!(expr(cx, &ast.expr)?));
                cx.statement_buffer.try_push(stmt)?;
                value.take()
            }
            ast::Stmt::Item(..) => continue,
        };

        if let Some(last) = last {
            cx.statements
                .try_push(hir::Stmt::Expr(last))
                .with_span(span)?;
        }

        for stmt in cx.statement_buffer.drain(..) {
            cx.statements.try_push(stmt).with_span(span)?;
        }
    }

    let statements = iter!(cx.statements.drain(at..));

    let layer = cx.scopes.pop().with_span(span)?;

    Ok(hir::Block {
        span: span.span(),
        label,
        statements,
        value,
        drop: iter!(layer.into_drop_order()),
    })
}

#[instrument_ast(span = ast)]
fn expr_range<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::ExprRange,
) -> compile::Result<hir::ExprRange<'hir>> {
    match (ast.start.as_deref(), ast.end.as_deref(), &ast.limits) {
        (Some(start), None, ast::ExprRangeLimits::HalfOpen(..)) => Ok(hir::ExprRange::RangeFrom {
            start: expr(cx, start)?,
        }),
        (None, None, ast::ExprRangeLimits::HalfOpen(..)) => Ok(hir::ExprRange::RangeFull),
        (Some(start), Some(end), ast::ExprRangeLimits::Closed(..)) => {
            Ok(hir::ExprRange::RangeInclusive {
                start: expr(cx, start)?,
                end: expr(cx, end)?,
            })
        }
        (None, Some(end), ast::ExprRangeLimits::Closed(..)) => {
            Ok(hir::ExprRange::RangeToInclusive {
                end: expr(cx, end)?,
            })
        }
        (None, Some(end), ast::ExprRangeLimits::HalfOpen(..)) => Ok(hir::ExprRange::RangeTo {
            end: expr(cx, end)?,
        }),
        (Some(start), Some(end), ast::ExprRangeLimits::HalfOpen(..)) => Ok(hir::ExprRange::Range {
            start: expr(cx, start)?,
            end: expr(cx, end)?,
        }),
        (Some(..) | None, None, ast::ExprRangeLimits::Closed(..)) => Err(Error::msg(
            ast,
            "Unsupported range, you probably want `..` instead of `..=`",
        )),
    }
}

#[instrument_ast(span = ast)]
fn expr_object<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::ExprObject,
) -> compile::Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, ast);

    let span = ast;
    let mut keys_dup = HashMap::new();

    let assignments = &mut *iter!(&ast.assignments, |(ast, _)| {
        let key = object_key(cx, &ast.key)?;

        if let Some(_existing) = keys_dup.try_insert(key.1, key.0)? {
            return Err(Error::new(
                key.0,
                ErrorKind::DuplicateObjectKey {
                    #[cfg(feature = "emit")]
                    existing: _existing.span(),
                    #[cfg(feature = "emit")]
                    object: key.0.span(),
                },
            ));
        }

        let assign = match &ast.assign {
            Some((_, ast)) => expr(cx, ast)?,
            None => {
                let Some((name, _)) = cx.scopes.get(hir::Name::Str(key.1))? else {
                    return Err(Error::new(
                        key.0,
                        ErrorKind::MissingLocal {
                            name: key.1.try_to_string()?.try_into()?,
                        },
                    ));
                };

                hir::Expr {
                    span: ast.span(),
                    kind: hir::ExprKind::Variable(name),
                }
            }
        };

        hir::FieldAssign {
            key: (key.0.span(), key.1),
            assign,
            position: None,
        }
    });

    let mut check_object_fields = |fields: &HashMap<_, meta::FieldMeta>, item: &Item| {
        let mut fields = fields.try_clone()?;

        for assign in assignments.iter_mut() {
            match fields.remove(assign.key.1) {
                Some(field_meta) => {
                    assign.position = Some(field_meta.position);
                }
                None => {
                    return Err(Error::new(
                        assign.key.0,
                        ErrorKind::LitObjectNotField {
                            field: assign.key.1.try_into()?,
                            item: item.try_to_owned()?,
                        },
                    ));
                }
            };
        }

        if let Some(field) = fields.into_keys().next() {
            return Err(Error::new(
                span,
                ErrorKind::LitObjectMissingField {
                    field,
                    item: item.try_to_owned()?,
                },
            ));
        }

        Ok(())
    };

    let kind = match &ast.ident {
        ast::ObjectIdent::Named(path) => {
            let named = cx.q.convert_path(path)?;
            let parameters = generics_parameters(cx, &named)?;
            let meta = cx.lookup_meta(path, named.item, parameters)?;
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
        ast::ObjectIdent::Anonymous(..) => hir::ExprObjectKind::Anonymous,
    };

    Ok(hir::ExprKind::Object(alloc!(hir::ExprObject {
        kind,
        assignments,
    })))
}

/// Lower an expression.
#[instrument_ast(span = ast)]
pub(crate) fn expr<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::Expr,
) -> compile::Result<hir::Expr<'hir>> {
    alloc_with!(cx, ast);

    let in_path = take(&mut cx.in_path);

    let kind = match ast {
        ast::Expr::Path(ast) => expr_path(cx, ast, in_path)?,
        ast::Expr::Assign(ast) => hir::ExprKind::Assign(alloc!(hir::ExprAssign {
            lhs: expr(cx, &ast.lhs)?,
            rhs: expr(cx, &ast.rhs)?,
        })),
        // TODO: lower all of these loop constructs to the same loop-like
        // representation. We only do different ones here right now since it's
        // easier when refactoring.
        ast::Expr::While(ast) => {
            let label = match &ast.label {
                Some((label, _)) => Some(alloc_str!(label.resolve(resolve_context!(cx.q))?)),
                None => None,
            };

            cx.scopes.push_loop(label)?;
            let condition = condition(cx, &ast.condition)?;
            let body = block(cx, None, &ast.body)?;
            let layer = cx.scopes.pop().with_span(ast)?;

            hir::ExprKind::Loop(alloc!(hir::ExprLoop {
                label,
                condition: Some(alloc!(condition)),
                body,
                drop: iter!(layer.into_drop_order()),
            }))
        }
        ast::Expr::Loop(ast) => {
            let label = match &ast.label {
                Some((label, _)) => Some(alloc_str!(label.resolve(resolve_context!(cx.q))?)),
                None => None,
            };

            cx.scopes.push_loop(label)?;
            let body = block(cx, None, &ast.body)?;
            let layer = cx.scopes.pop().with_span(ast)?;

            let kind = hir::ExprKind::Loop(alloc!(hir::ExprLoop {
                label,
                condition: None,
                body,
                drop: iter!(layer.into_drop_order()),
            }));

            kind
        }
        ast::Expr::For(ast) => {
            let iter = expr(cx, &ast.iter)?;

            let label = match &ast.label {
                Some((label, _)) => Some(alloc_str!(label.resolve(resolve_context!(cx.q))?)),
                None => None,
            };

            cx.scopes.push_loop(label)?;
            let binding = pat_binding(cx, &ast.binding)?;
            let body = block(cx, None, &ast.body)?;

            let layer = cx.scopes.pop().with_span(ast)?;

            hir::ExprKind::For(alloc!(hir::ExprFor {
                label,
                binding,
                iter,
                body,
                drop: iter!(layer.into_drop_order()),
            }))
        }
        ast::Expr::Let(ast) => hir::ExprKind::Let(alloc!(hir::ExprLet {
            pat: pat_binding(cx, &ast.pat)?,
            expr: expr(cx, &ast.expr)?,
        })),
        ast::Expr::If(ast) => hir::ExprKind::If(alloc!(expr_if(cx, ast)?)),
        ast::Expr::Match(ast) => hir::ExprKind::Match(alloc!(hir::ExprMatch {
            expr: expr(cx, &ast.expr)?,
            branches: iter!(&ast.branches, |(ast, _)| {
                cx.scopes.push(None)?;

                let pat = pat_binding(cx, &ast.pat)?;
                let condition = option!(&ast.condition, |(_, ast)| expr(cx, ast)?);
                let body = expr(cx, &ast.body)?;

                let layer = cx.scopes.pop().with_span(ast)?;

                hir::ExprMatchBranch {
                    span: ast.span(),
                    pat,
                    condition,
                    body,
                    drop: iter!(layer.into_drop_order()),
                }
            }),
        })),
        ast::Expr::Call(ast) => hir::ExprKind::Call(alloc!(expr_call(cx, ast)?)),
        ast::Expr::FieldAccess(ast) => {
            hir::ExprKind::FieldAccess(alloc!(expr_field_access(cx, ast)?))
        }
        ast::Expr::Empty(ast) => {
            // NB: restore in_path setting.
            cx.in_path = in_path;
            hir::ExprKind::Group(alloc!(expr(cx, &ast.expr)?))
        }
        ast::Expr::Binary(ast) => {
            let rhs_needs = match &ast.op {
                ast::BinOp::As(..) | ast::BinOp::Is(..) | ast::BinOp::IsNot(..) => Needs::Type,
                _ => Needs::Value,
            };

            let lhs = expr(cx, &ast.lhs)?;

            let needs = replace(&mut cx.needs, rhs_needs);
            let rhs = expr(cx, &ast.rhs)?;
            cx.needs = needs;

            hir::ExprKind::Binary(alloc!(hir::ExprBinary {
                lhs,
                op: ast.op,
                rhs,
            }))
        }
        ast::Expr::Unary(ast) => expr_unary(cx, ast)?,
        ast::Expr::Index(ast) => hir::ExprKind::Index(alloc!(hir::ExprIndex {
            target: expr(cx, &ast.target)?,
            index: expr(cx, &ast.index)?,
        })),
        ast::Expr::Block(ast) => expr_block(cx, ast)?,
        ast::Expr::Break(ast) => hir::ExprKind::Break(alloc!(expr_break(cx, ast)?)),
        ast::Expr::Continue(ast) => hir::ExprKind::Continue(alloc!(expr_continue(cx, ast)?)),
        ast::Expr::Yield(ast) => hir::ExprKind::Yield(option!(&ast.expr, |ast| expr(cx, ast)?)),
        ast::Expr::Return(ast) => hir::ExprKind::Return(option!(&ast.expr, |ast| expr(cx, ast)?)),
        ast::Expr::Await(ast) => hir::ExprKind::Await(alloc!(expr(cx, &ast.expr)?)),
        ast::Expr::Try(ast) => hir::ExprKind::Try(alloc!(expr(cx, &ast.expr)?)),
        ast::Expr::Select(ast) => {
            let mut default = None;
            let mut branches = Vec::new();
            let mut exprs = Vec::new();

            for (ast, _) in &ast.branches {
                match ast {
                    ast::ExprSelectBranch::Pat(ast) => {
                        cx.scopes.push(None)?;

                        let pat = pat_binding(cx, &ast.pat)?;
                        let body = expr(cx, &ast.body)?;

                        let layer = cx.scopes.pop().with_span(&ast)?;

                        exprs.try_push(expr(cx, &ast.expr)?).with_span(&ast.expr)?;

                        branches.try_push(hir::ExprSelectBranch {
                            pat,
                            body,
                            drop: iter!(layer.into_drop_order()),
                        })?;
                    }
                    ast::ExprSelectBranch::Default(ast) => {
                        if default.is_some() {
                            return Err(Error::new(
                                ast,
                                ErrorKind::SelectMultipleDefaults,
                            ));
                        }

                        default = Some(alloc!(expr(cx, &ast.body)?));
                    }
                }
            }

            hir::ExprKind::Select(alloc!(hir::ExprSelect {
                branches: iter!(branches),
                exprs: iter!(exprs),
                default: option!(default),
            }))
        }
        ast::Expr::Closure(ast) => expr_call_closure(cx, ast)?,
        ast::Expr::Lit(ast) => hir::ExprKind::Lit(lit(cx, &ast.lit)?),
        ast::Expr::Object(ast) => expr_object(cx, ast)?,
        ast::Expr::Tuple(ast) => hir::ExprKind::Tuple(alloc!(hir::ExprSeq {
            items: iter!(&ast.items, |(ast, _)| expr(cx, ast)?),
        })),
        ast::Expr::Vec(ast) => hir::ExprKind::Vec(alloc!(hir::ExprSeq {
            items: iter!(&ast.items, |(ast, _)| expr(cx, ast)?),
        })),
        ast::Expr::Range(ast) => hir::ExprKind::Range(alloc!(expr_range(cx, ast)?)),
        ast::Expr::Group(ast) => hir::ExprKind::Group(alloc!(expr(cx, &ast.expr)?)),
        ast::Expr::MacroCall(ast) => {
            let Some(id) = ast.id else {
                return Err(Error::msg(ast, "missing expanded macro id"));
            };

            match cx.q.builtin_macro_for(id).with_span(ast)?.as_ref() {
                query::BuiltInMacro::Template(ast) => {
                    let old = replace(&mut cx.in_template, true);

                    let result = hir::ExprKind::Template(alloc!(hir::BuiltInTemplate {
                        span: ast.span,
                        from_literal: ast.from_literal,
                        exprs: iter!(&ast.exprs, |ast| expr(cx, ast)?),
                    }));

                    cx.in_template = old;
                    result
                }
                query::BuiltInMacro::Format(ast) => {
                    hir::ExprKind::Format(alloc!(hir::BuiltInFormat {
                        span: ast.span,
                        fill: ast.fill,
                        align: ast.align,
                        width: ast.width,
                        precision: ast.precision,
                        flags: ast.flags,
                        format_type: ast.format_type,
                        value: expr(cx, &ast.value)?,
                    }))
                }
                query::BuiltInMacro::File(ast) => hir::ExprKind::Lit(lit(cx, &ast.value)?),
                query::BuiltInMacro::Line(ast) => hir::ExprKind::Lit(lit(cx, &ast.value)?),
            }
        }
    };

    Ok(hir::Expr {
        span: ast.span(),
        kind,
    })
}

#[instrument_ast(span = ast)]
fn expr_if<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::ExprIf,
) -> compile::Result<hir::Conditional<'hir>> {
    alloc_with!(cx, ast);

    let length = 1 + ast.expr_else_ifs.len();

    let then = [(
        ast.if_.span().join(ast.block.span()),
        &ast.condition,
        &ast.block,
    )]
    .into_iter();

    let else_ifs = ast
        .expr_else_ifs
        .iter()
        .map(|ast| (ast.span(), &ast.condition, &ast.block));

    let branches = iter!(then.chain(else_ifs), length, |(span, c, b)| {
        cx.scopes.push(None)?;

        let condition = condition(cx, c)?;
        let block = block(cx, None, b)?;

        let layer = cx.scopes.pop().with_span(ast)?;

        let condition = &*alloc!(condition);
        let drop = &*iter!(layer.into_drop_order());

        hir::ConditionalBranch {
            span,
            condition,
            block,
            drop,
        }
    });

    let fallback = match &ast.expr_else {
        Some(ast) => Some(&*alloc!(block(cx, None, &ast.block)?)),
        None => None,
    };

    Ok(hir::Conditional { branches, fallback })
}

#[instrument_ast(span = ast)]
fn expr_unary<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::ExprUnary,
) -> compile::Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, ast);

    // NB: special unary expressions.
    if let ast::UnOp::BorrowRef { .. } = ast.op {
        return Err(Error::new(ast, ErrorKind::UnsupportedRef));
    }

    let (
        ast::UnOp::Neg(..),
        ast::Expr::Lit(ast::ExprLit {
            lit: ast::Lit::Number(n),
            ..
        }),
    ) = (ast.op, &*ast.expr)
    else {
        return Ok(hir::ExprKind::Unary(alloc!(hir::ExprUnary {
            op: ast.op,
            expr: expr(cx, &ast.expr)?,
        })));
    };

    let number = n.resolve(resolve_context!(cx.q))?;

    match (number.value, number.suffix) {
        (ast::NumberValue::Float(n), Some(ast::NumberSuffix::Float(..)) | None) => {
            Ok(hir::ExprKind::Lit(hir::Lit::Float(-n)))
        }
        (ast::NumberValue::Integer(int), Some(ast::NumberSuffix::Int(..)) | None) => {
            let Some(n) = int.neg().to_i64() else {
                return Err(Error::new(ast, ErrorKind::BadNumberOutOfBounds));
            };

            Ok(hir::ExprKind::Lit(hir::Lit::Integer(n)))
        }
        _ => Err(Error::new(ast, ErrorKind::BadNumberOutOfBounds)),
    }
}

/// Lower a block expression.
#[instrument_ast(span = ast)]
fn expr_block<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::ExprBlock,
) -> compile::Result<hir::ExprKind<'hir>> {
    /// The kind of an [ExprBlock].
    #[derive(Debug, Clone, Copy, PartialEq)]
    #[non_exhaustive]
    pub(crate) enum ExprBlockKind {
        Default,
        Async,
        Const,
    }

    alloc_with!(cx, ast);

    let kind = match (&ast.async_token, &ast.const_token) {
        (Some(..), None) => ExprBlockKind::Async,
        (None, Some(..)) => ExprBlockKind::Const,
        _ => ExprBlockKind::Default,
    };

    if let ExprBlockKind::Default = kind {
        return Ok(hir::ExprKind::Block(alloc!(block(
            cx,
            ast.label.as_ref(),
            &ast.block
        )?)));
    }

    if cx.const_eval {
        // This only happens if the ast expression has not been indexed. Which
        // only occurs during certain kinds of constant evaluation. So we limit
        // evaluation to only support constant blocks.
        let ExprBlockKind::Const = kind else {
            return Err(Error::msg(
                ast,
                "Only constant blocks are supported in this context",
            ));
        };

        if let Some(label) = &ast.label {
            return Err(Error::msg(
                label,
                "Constant blocks cannot be labelled",
            ));
        };

        return Ok(hir::ExprKind::Block(alloc!(block(cx, None, &ast.block)?)));
    };

    let item = cx.q.item_for(ast.block.id).with_span(&ast.block)?;
    let meta = cx.lookup_meta(ast, item.item, GenericsParameters::default())?;

    match (kind, &meta.kind) {
        (ExprBlockKind::Async, &meta::Kind::AsyncBlock { call, do_move, .. }) => {
            tracing::trace!("queuing async block build entry");

            if let Some(label) = &ast.label {
                return Err(Error::msg(
                    label,
                    "Async blocks cannot be labelled",
                ));
            };

            cx.scopes.push_captures()?;
            let block = alloc!(block(cx, None, &ast.block)?);
            let layer = cx.scopes.pop().with_span(&ast.block)?;

            cx.q.set_used(&meta.item_meta)?;

            let captures = &*iter!(layer.captures().map(|(_, id)| id));

            let Some(queue) = cx.secondary_builds.as_mut() else {
                return Err(Error::new(ast, ErrorKind::AsyncBlockInConst));
            };

            queue.try_push(SecondaryBuildEntry {
                item_meta: meta.item_meta,
                build: SecondaryBuild::AsyncBlock(AsyncBlock {
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
        (ExprBlockKind::Const, meta::Kind::Const { .. }) => Ok(hir::ExprKind::Const(meta.hash)),
        _ => Err(Error::expected_meta(
            ast,
            meta.info(cx.q.pool)?,
            "async or const block",
        )),
    }
}

/// Unroll a break expression, capturing all variables which are in scope at
/// the time of it.
fn expr_break<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::ExprBreak,
) -> compile::Result<hir::ExprBreak<'hir>> {
    alloc_with!(cx, ast);

    let label = match &ast.label {
        Some(label) => Some(label.resolve(resolve_context!(cx.q))?),
        None => None,
    };

    let Some(drop) = cx.scopes.loop_drop(label)? else {
        if let Some(label) = label {
            return Err(Error::new(
                ast,
                ErrorKind::MissingLabel {
                    label: label.try_into()?,
                },
            ));
        } else {
            return Err(Error::new(ast, ErrorKind::BreakUnsupported));
        }
    };

    Ok(hir::ExprBreak {
        label: match label {
            Some(label) => Some(alloc_str!(label)),
            None => None,
        },
        expr: match &ast.expr {
            Some(ast) => Some(alloc!(expr(cx, ast)?)),
            None => None,
        },
        drop: iter!(drop),
    })
}

/// Unroll a continue expression, capturing all variables which are in scope at
/// the time of it.
fn expr_continue<'hir>(
    cx: &Ctxt<'hir, '_, '_>,
    ast: &ast::ExprContinue,
) -> compile::Result<hir::ExprContinue<'hir>> {
    alloc_with!(cx, ast);

    let label = match &ast.label {
        Some(label) => Some(label.resolve(resolve_context!(cx.q))?),
        None => None,
    };

    let Some(drop) = cx.scopes.loop_drop(label)? else {
        if let Some(label) = label {
            return Err(Error::new(
                ast,
                ErrorKind::MissingLabel {
                    label: label.try_into()?,
                },
            ));
        } else {
            return Err(Error::new(ast, ErrorKind::ContinueUnsupported));
        }
    };

    Ok(hir::ExprContinue {
        label: match label {
            Some(label) => Some(alloc_str!(label)),
            None => None,
        },
        drop: iter!(drop),
    })
}

/// Lower a function argument.
fn fn_arg<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::FnArg,
) -> compile::Result<hir::FnArg<'hir>> {
    alloc_with!(cx, ast);

    Ok(match ast {
        ast::FnArg::SelfValue(ast) => {
            let id = cx.scopes.define(hir::Name::SelfValue, ast)?;
            hir::FnArg::SelfValue(ast.span(), id)
        }
        ast::FnArg::Pat(ast) => hir::FnArg::Pat(alloc!(pat_binding(cx, ast)?)),
    })
}

/// The is a simple locals optimization which unpacks locals from a tuple and
/// assigns them directly to local.
fn unpack_locals(cx: &mut Ctxt<'_, '_, '_>, p: &ast::Pat, e: &ast::Expr) -> compile::Result<bool> {
    alloc_with!(cx, p);

    match (p, e) {
        (p @ ast::Pat::Path(inner), e) => {
            let Some(ast::PathKind::Ident(..)) = inner.path.as_kind() else {
                return Ok(false);
            };

            let e = expr(cx, e)?;
            let p = pat_binding(cx, p)?;

            cx.statement_buffer
                .try_push(hir::Stmt::Local(alloc!(hir::Local {
                    span: p.span().join(e.span()),
                    pat: p,
                    expr: e,
                })))?;

            return Ok(true);
        }
        (ast::Pat::Tuple(p), ast::Expr::Tuple(e)) => {
            if p.items.len() != e.items.len() {
                return Ok(false);
            }

            for ((_, _), (p, _)) in e.items.iter().zip(&p.items) {
                if matches!(p, ast::Pat::Rest(..)) {
                    return Ok(false);
                }
            }

            let mut exprs = Vec::new();

            for (e, _) in &e.items {
                exprs.try_push(expr(cx, e)?)?;
            }

            for (e, (p, _)) in exprs.into_iter().zip(&p.items) {
                let p = pat_binding(cx, p)?;

                cx.statement_buffer
                    .try_push(hir::Stmt::Local(alloc!(hir::Local {
                        span: p.span().join(e.span()),
                        pat: p,
                        expr: e,
                    })))?;
            }

            return Ok(true);
        }
        _ => {}
    };

    Ok(false)
}

fn pat<'hir>(cx: &mut Ctxt<'hir, '_, '_>, ast: &ast::Pat) -> compile::Result<hir::Pat<'hir>> {
    fn filter((ast, _): &(ast::Pat, Option<ast::Comma>)) -> Option<&ast::Pat> {
        if matches!(ast, ast::Pat::Binding(..) | ast::Pat::Rest(..)) {
            return None;
        }

        Some(ast)
    }

    alloc_with!(cx, ast);

    let kind = {
        match ast {
            ast::Pat::Ignore(..) => hir::PatKind::Ignore,
            ast::Pat::Path(ast) => {
                let named = cx.q.convert_path(&ast.path)?;
                let parameters = generics_parameters(cx, &named)?;

                let path = 'path: {
                    if let Some(meta) = cx.try_lookup_meta(&ast, named.item, &parameters)? {
                        match meta.kind {
                            meta::Kind::Const => {
                                let Some(const_value) = cx.q.get_const_value(meta.hash) else {
                                    return Err(Error::msg(
                                        ast,
                                        try_format!("Missing constant for hash {}", meta.hash),
                                    ));
                                };

                                let const_value = const_value.try_clone().with_span(ast)?;
                                return pat_const_value(cx, &const_value, ast);
                            }
                            _ => {
                                if let Some((0, kind)) = tuple_match_for(cx, &meta) {
                                    break 'path hir::PatPathKind::Kind(alloc!(kind));
                                }
                            }
                        }
                    };

                    if let Some(ident) = ast.path.try_as_ident() {
                        let name = alloc_str!(ident.resolve(resolve_context!(cx.q))?);
                        let name = cx.scopes.define(hir::Name::Str(name), ast)?;
                        cx.pattern_bindings.try_push(name)?;
                        break 'path hir::PatPathKind::Ident(name);
                    }

                    return Err(Error::new(ast, ErrorKind::UnsupportedBinding));
                };

                hir::PatKind::Path(alloc!(path))
            }
            ast::Pat::Lit(ast) => hir::PatKind::Lit(alloc!(expr(cx, &ast.expr)?)),
            ast::Pat::Vec(ast) => {
                let (is_open, count) = pat_items_count(ast.items.as_slice())?;
                let items = iter!(
                    ast.items.iter().filter_map(filter),
                    ast.items.len(),
                    |ast| pat(cx, ast)?
                );

                hir::PatKind::Sequence(alloc!(hir::PatSequence {
                    kind: hir::PatSequenceKind::Anonymous {
                        type_check: TypeCheck::Vec,
                        count,
                        is_open
                    },
                    items,
                }))
            }
            ast::Pat::Tuple(ast) => {
                let (is_open, count) = pat_items_count(ast.items.as_slice())?;
                let items = iter!(
                    ast.items.iter().filter_map(filter),
                    ast.items.len(),
                    |ast| pat(cx, ast)?
                );

                let kind = if let Some(path) = &ast.path {
                    let named = cx.q.convert_path(path)?;
                    let parameters = generics_parameters(cx, &named)?;
                    let meta = cx.lookup_meta(path, named.item, parameters)?;

                    // Treat the current meta as a tuple and get the number of arguments it
                    // should receive and the type check that applies to it.
                    let Some((args, kind)) = tuple_match_for(cx, &meta) else {
                        return Err(Error::expected_meta(
                            path,
                            meta.info(cx.q.pool)?,
                            "type that can be used in a tuple pattern",
                        ));
                    };

                    if !(args == count || count < args && is_open) {
                        return Err(Error::new(
                            path,
                            ErrorKind::UnsupportedArgumentCount {
                                expected: args,
                                actual: count,
                            },
                        ));
                    }

                    kind
                } else {
                    hir::PatSequenceKind::Anonymous {
                        type_check: TypeCheck::Tuple,
                        count,
                        is_open,
                    }
                };

                hir::PatKind::Sequence(alloc!(hir::PatSequence { kind, items }))
            }
            ast::Pat::Object(ast) => {
                let (is_open, count) = pat_items_count(ast.items.as_slice())?;

                let mut keys_dup = HashMap::new();

                let bindings = iter!(ast.items.iter().take(count), |(pat, _)| {
                    let (key, binding) = match pat {
                        ast::Pat::Binding(binding) => {
                            let (span, key) = object_key(cx, &binding.key)?;
                            (
                                key,
                                hir::Binding::Binding(
                                    span.span(),
                                    key,
                                    alloc!(self::pat(cx, &binding.pat)?),
                                ),
                            )
                        }
                        ast::Pat::Path(path) => {
                            let Some(ident) = path.path.try_as_ident() else {
                                return Err(Error::new(
                                    path,
                                    ErrorKind::UnsupportedPatternExpr,
                                ));
                            };

                            let key = alloc_str!(ident.resolve(resolve_context!(cx.q))?);
                            let id = cx.scopes.define(hir::Name::Str(key), ident)?;
                            cx.pattern_bindings.try_push(id)?;
                            (key, hir::Binding::Ident(path.span(), key, id))
                        }
                        _ => {
                            return Err(Error::new(
                                pat,
                                ErrorKind::UnsupportedPatternExpr,
                            ));
                        }
                    };

                    if let Some(_existing) = keys_dup.try_insert(key, pat)? {
                        return Err(Error::new(
                            pat,
                            ErrorKind::DuplicateObjectKey {
                                #[cfg(feature = "emit")]
                                existing: _existing.span(),
                                #[cfg(feature = "emit")]
                                object: pat.span(),
                            },
                        ));
                    }

                    binding
                });

                let kind = match &ast.ident {
                    ast::ObjectIdent::Named(path) => {
                        let named = cx.q.convert_path(path)?;
                        let parameters = generics_parameters(cx, &named)?;
                        let meta = cx.lookup_meta(path, named.item, parameters)?;

                        let Some((mut fields, kind)) =
                            struct_match_for(cx, &meta, is_open && count == 0)?
                        else {
                            return Err(Error::expected_meta(
                                path,
                                meta.info(cx.q.pool)?,
                                "type that can be used in a struct pattern",
                            ));
                        };

                        for binding in bindings.iter() {
                            if !fields.remove(binding.key()) {
                                return Err(Error::new(
                                    ast,
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
                                ast,
                                ErrorKind::PatternMissingFields {
                                    item: cx.q.pool.item(meta.item_meta.item).try_to_owned()?,
                                    #[cfg(feature = "emit")]
                                    fields,
                                },
                            ));
                        }

                        kind
                    }
                    ast::ObjectIdent::Anonymous(..) => hir::PatSequenceKind::Anonymous {
                        type_check: TypeCheck::Object,
                        count,
                        is_open,
                    },
                };

                hir::PatKind::Object(alloc!(hir::PatObject { kind, bindings }))
            }
            _ => {
                return Err(Error::new(ast, ErrorKind::UnsupportedPatternExpr));
            }
        }
    };

    Ok(hir::Pat {
        span: ast.span(),
        kind,
    })
}

fn object_key<'hir, 'ast>(
    cx: &Ctxt<'hir, '_, '_>,
    ast: &'ast ast::ObjectKey,
) -> compile::Result<(&'ast dyn Spanned, &'hir str)> {
    alloc_with!(cx, ast);

    Ok(match ast {
        ast::ObjectKey::LitStr(lit) => {
            let string = lit.resolve(resolve_context!(cx.q))?;
            (lit, alloc_str!(string.as_ref()))
        }
        ast::ObjectKey::Path(ast) => {
            let Some(ident) = ast.try_as_ident() else {
                return Err(Error::expected(ast, "object key"));
            };

            let string = ident.resolve(resolve_context!(cx.q))?;
            (ident, alloc_str!(string))
        }
    })
}

/// Lower the given path.
#[instrument_ast(span = ast)]
fn expr_path<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::Path,
    in_path: bool,
) -> compile::Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, ast);

    if let Some(ast::PathKind::SelfValue) = ast.as_kind() {
        let Some((id, _)) = cx.scopes.get(hir::Name::SelfValue)? else {
            return Err(Error::new(ast, ErrorKind::MissingSelf));
        };

        return Ok(hir::ExprKind::Variable(id));
    }

    if let Needs::Value = cx.needs {
        if let Some(name) = ast.try_as_ident() {
            let name = alloc_str!(name.resolve(resolve_context!(cx.q))?);

            if let Some((name, _)) = cx.scopes.get(hir::Name::Str(name))? {
                return Ok(hir::ExprKind::Variable(name));
            }
        }
    }

    // Caller has indicated that if they can't have a variable, they do indeed
    // want a path.
    if in_path {
        return Ok(hir::ExprKind::Path);
    }

    let named = cx.q.convert_path(ast)?;
    let parameters = generics_parameters(cx, &named)?;

    if let Some(meta) = cx.try_lookup_meta(ast, named.item, &parameters)? {
        return expr_path_meta(cx, &meta, ast);
    }

    if let (Needs::Value, Some(local)) = (cx.needs, ast.try_as_ident()) {
        let local = local.resolve(resolve_context!(cx.q))?;

        // light heuristics, treat it as a type error in case the first
        // character is uppercase.
        if !local.starts_with(char::is_uppercase) {
            return Err(Error::new(
                ast,
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

    Err(Error::new(ast, kind))
}

fn condition<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::Condition,
) -> compile::Result<hir::Condition<'hir>> {
    alloc_with!(cx, ast);

    Ok(match ast {
        ast::Condition::Expr(ast) => hir::Condition::Expr(alloc!(expr(cx, ast)?)),
        ast::Condition::ExprLet(ast) => hir::Condition::ExprLet(alloc!(hir::ExprLet {
            pat: pat_binding(cx, &ast.pat)?,
            expr: expr(cx, &ast.expr)?,
        })),
    })
}

/// Test if the given pattern is open or not.
fn pat_items_count(items: &[(ast::Pat, Option<ast::Comma>)]) -> compile::Result<(bool, usize)> {
    let mut it = items.iter();

    let (is_open, mut count) = match it.next_back() {
        Some((pat, _)) => matches!(pat, ast::Pat::Rest { .. })
            .then(|| (true, 0))
            .unwrap_or((false, 1)),
        None => return Ok((false, 0)),
    };

    for (pat, _) in it {
        if let ast::Pat::Rest { .. } = pat {
            return Err(Error::new(pat, ErrorKind::UnsupportedPatternRest));
        }

        count += 1;
    }

    Ok((is_open, count))
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

/// Convert into a call expression.
#[instrument_ast(span = ast)]
fn expr_call<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::ExprCall,
) -> compile::Result<hir::ExprCall<'hir>> {
    fn find_path(ast: &ast::Expr) -> Option<&ast::Path> {
        let mut current = ast;

        loop {
            match current {
                ast::Expr::Path(path) => return Some(path),
                ast::Expr::Empty(ast) => {
                    current = &*ast.expr;
                    continue;
                }
                _ => return None,
            }
        }
    }

    alloc_with!(cx, ast);

    let in_path = replace(&mut cx.in_path, true);
    let expr = expr(cx, &ast.expr)?;
    cx.in_path = in_path;

    let call = 'ok: {
        match expr.kind {
            hir::ExprKind::Variable(name) => {
                break 'ok hir::Call::Var { name };
            }
            hir::ExprKind::Path => {
                let Some(path) = find_path(&ast.expr) else {
                    return Err(Error::msg(&ast.expr, "Expected path"));
                };

                let named = cx.q.convert_path(path)?;
                let parameters = generics_parameters(cx, &named)?;

                let meta = cx.lookup_meta(path, named.item, parameters)?;
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
                        if !ast.args.is_empty() {
                            return Err(Error::new(
                                &ast.args,
                                ErrorKind::UnsupportedArgumentCount {
                                    expected: 0,
                                    actual: ast.args.len(),
                                },
                            ));
                        }
                    }
                    meta::Kind::Struct {
                        fields: meta::Fields::Unnamed(args),
                        ..
                    }
                    | meta::Kind::Variant {
                        fields: meta::Fields::Unnamed(args),
                        ..
                    } => {
                        if *args != ast.args.len() {
                            return Err(Error::new(
                                &ast.args,
                                ErrorKind::UnsupportedArgumentCount {
                                    expected: *args,
                                    actual: ast.args.len(),
                                },
                            ));
                        }

                        if *args == 0 {
                            cx.q.diagnostics.remove_tuple_call_parens(
                                cx.source_id,
                                &ast.args,
                                path,
                                None,
                            )?;
                        }
                    }
                    meta::Kind::Function { .. } => {
                        if let Some(message) = cx.q.lookup_deprecation(meta.hash) {
                            cx.q.diagnostics.used_deprecated(
                                cx.source_id,
                                &expr.span,
                                None,
                                message.try_into()?,
                            )?;
                        };
                    }
                    meta::Kind::ConstFn => {
                        let from = cx.q.item_for(ast.id).with_span(ast)?;

                        break 'ok hir::Call::ConstFn {
                            from_module: from.module,
                            from_item: from.item,
                            id: meta.item_meta.item,
                        };
                    }
                    _ => {
                        return Err(Error::expected_meta(
                            ast,
                            meta.info(cx.q.pool)?,
                            "something that can be called as a function",
                        ));
                    }
                };

                break 'ok hir::Call::Meta { hash: meta.hash };
            }
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

                break 'ok hir::Call::Associated {
                    target: alloc!(target),
                    hash,
                };
            }
            _ => {}
        }

        break 'ok hir::Call::Expr { expr: alloc!(expr) };
    };

    Ok(hir::ExprCall {
        call,
        args: iter!(&ast.args, |(ast, _)| self::expr(cx, ast)?),
    })
}

#[instrument_ast(span = ast)]
fn expr_field_access<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::ExprFieldAccess,
) -> compile::Result<hir::ExprFieldAccess<'hir>> {
    alloc_with!(cx, ast);

    let expr_field = match &ast.expr_field {
        ast::ExprField::LitNumber(ast) => {
            let number = ast.resolve(resolve_context!(cx.q))?;

            let Some(index) = number.as_tuple_index() else {
                return Err(Error::new(
                    ast,
                    ErrorKind::UnsupportedTupleIndex { number },
                ));
            };

            hir::ExprField::Index(index)
        }
        ast::ExprField::Path(ast) => {
            let Some((ident, generics)) = ast.try_as_ident_generics() else {
                return Err(Error::new(ast, ErrorKind::BadFieldAccess));
            };

            let ident = alloc_str!(ident.resolve(resolve_context!(cx.q))?);

            match generics {
                Some(generics) => {
                    let mut builder = ParametersBuilder::new();

                    for (s, _) in generics {
                        let hir::ExprKind::Type(ty) = expr(cx, &s.expr)?.kind else {
                            return Err(Error::new(s, ErrorKind::UnsupportedGenerics));
                        };

                        builder.add(ty.into_hash());
                    }

                    hir::ExprField::IdentGenerics(ident, builder.finish())
                }
                None => hir::ExprField::Ident(ident),
            }
        }
    };

    Ok(hir::ExprFieldAccess {
        expr: expr(cx, &ast.expr)?,
        expr_field,
    })
}
*/
