use core::cell::Cell;
use core::ops::Neg;

use num::ToPrimitive;

use crate::alloc::prelude::*;
use crate::alloc::try_format;
use crate::alloc::{self, Box, HashMap, HashSet};
use crate::ast::{self, Spanned};
use crate::compile::meta;
use crate::compile::{self, DynLocation, ErrorKind, Item, ItemId, WithSpan};
use crate::hash::{Hash, ParametersBuilder};
use crate::hir;
use crate::indexing;
use crate::parse::Resolve;
use crate::query::{self, Build, BuildEntry, GenericsParameters, Named, Query};
use crate::runtime::{Type, TypeCheck};
use crate::SourceId;

use rune_macros::instrument;

#[derive(Default, Clone, Copy)]
enum Needs {
    #[default]
    Value,
    Type,
}

pub(crate) struct Ctxt<'hir, 'a, 'arena> {
    /// Arena used for allocations.
    arena: &'hir hir::arena::Arena,
    q: Query<'a, 'arena>,
    source_id: SourceId,
    in_template: Cell<bool>,
    in_path: Cell<bool>,
    needs: Cell<Needs>,
    scopes: hir::Scopes<'hir>,
    const_eval: bool,
}

impl<'hir, 'a, 'arena> Ctxt<'hir, 'a, 'arena> {
    #[inline(always)]
    fn in_path<F, O>(&mut self, in_path: bool, f: F) -> O
    where
        F: FnOnce(&mut Self) -> O,
    {
        let in_path = self.in_path.replace(in_path);
        let output = f(self);
        self.in_path.set(in_path);
        output
    }

    /// Construct a new context for used when constants are built separately
    /// through the query system.
    pub(crate) fn with_query(
        arena: &'hir hir::arena::Arena,
        q: Query<'a, 'arena>,
        source_id: SourceId,
    ) -> alloc::Result<Self> {
        Self::inner(arena, q, source_id, false)
    }

    /// Construct a new context used in a constant context where the resulting
    /// expression is expected to be converted into a constant.
    pub(crate) fn with_const(
        arena: &'hir hir::arena::Arena,
        q: Query<'a, 'arena>,
        source_id: SourceId,
    ) -> alloc::Result<Self> {
        Self::inner(arena, q, source_id, true)
    }

    fn inner(
        arena: &'hir hir::arena::Arena,
        q: Query<'a, 'arena>,
        source_id: SourceId,
        const_eval: bool,
    ) -> alloc::Result<Self> {
        Ok(Self {
            arena,
            q,
            source_id,
            in_template: Cell::new(false),
            in_path: Cell::new(false),
            needs: Cell::new(Needs::default()),
            scopes: hir::Scopes::new()?,
            const_eval,
        })
    }

    #[allow(unused)]
    #[instrument(span = ast)]
    pub(crate) fn try_lookup_meta(
        &mut self,
        span: &dyn Spanned,
        item: ItemId,
        parameters: &GenericsParameters,
    ) -> compile::Result<Option<meta::Meta>> {
        self.q
            .try_lookup_meta(&DynLocation::new(self.source_id, span), item, parameters)
    }

    #[instrument(span = ast)]
    pub(crate) fn lookup_meta(
        &mut self,
        span: &dyn Spanned,
        item: ItemId,
        parameters: impl AsRef<GenericsParameters>,
    ) -> compile::Result<meta::Meta> {
        self.q
            .lookup_meta(&DynLocation::new(self.source_id, span), item, parameters)
    }
}

/// Lower an empty function.
#[instrument(span = span)]
pub(crate) fn empty_fn<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::EmptyBlock,
    span: &dyn Spanned,
) -> compile::Result<hir::ItemFn<'hir>> {
    alloc_with!(cx, span);

    cx.scopes.push()?;

    let statements = iter!(&ast.statements, |ast| stmt(cx, ast)?);

    let layer = cx.scopes.pop().with_span(span)?;

    let body = hir::Block {
        span: span.span(),
        statements,
        drop: iter!(layer.into_drop_order()),
    };

    Ok(hir::ItemFn {
        span: span.span(),
        args: &[],
        body,
    })
}
/// Lower a function item.
#[instrument(span = ast)]
pub(crate) fn item_fn<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::ItemFn,
) -> compile::Result<hir::ItemFn<'hir>> {
    alloc_with!(cx, ast);

    Ok(hir::ItemFn {
        span: ast.span(),
        args: iter!(&ast.args, |(ast, _)| fn_arg(cx, ast)?),
        body: block(cx, &ast.body)?,
    })
}

/// Lower the body of an async block.
///
/// This happens *after* it's been lowered as part of a closure expression.
#[instrument(span = ast)]
pub(crate) fn async_block_secondary<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::Block,
    captures: Hash,
) -> compile::Result<hir::AsyncBlock<'hir>> {
    alloc_with!(cx, ast);

    let Some(captures) = cx.q.get_captures(captures) else {
        return Err(compile::Error::msg(
            ast,
            try_format!("Missing captures for hash {captures}"),
        ));
    };

    let captures = &*iter!(captures, |capture| {
        match capture {
            hir::OwnedName::SelfValue => cx.scopes.define(hir::Name::SelfValue, ast)?,
            hir::OwnedName::Str(name) => {
                let name = alloc_str!(name.as_str());
                cx.scopes.define(hir::Name::Str(name), ast)?
            }
            hir::OwnedName::Id(id) => cx.scopes.define(hir::Name::Id(*id), ast)?,
        }
    });

    Ok(hir::AsyncBlock {
        block: block(cx, ast)?,
        captures,
    })
}

/// Lower the body of a closure.
///
/// This happens *after* it's been lowered as part of a closure expression.
#[instrument(span = ast)]
pub(crate) fn expr_closure_secondary<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::ExprClosure,
    captures: Hash,
) -> compile::Result<hir::ExprClosure<'hir>> {
    alloc_with!(cx, ast);

    let Some(captures) = cx.q.get_captures(captures) else {
        return Err(compile::Error::msg(
            ast,
            try_format!("Missing captures for hash {captures}"),
        ));
    };

    let captures = &*iter!(captures, |capture| match capture {
        hir::OwnedName::SelfValue => {
            cx.scopes.define(hir::Name::SelfValue, ast)?
        }
        hir::OwnedName::Str(name) => {
            let name = hir::Name::Str(alloc_str!(name.as_str()));
            cx.scopes.define(name, ast)?
        }
        hir::OwnedName::Id(id) => {
            cx.scopes.define(hir::Name::Id(*id), ast)?
        }
    });

    let args = iter!(ast.args.as_slice(), |(ast, _)| fn_arg(cx, ast)?);
    let body = expr(cx, &ast.body)?;

    Ok(hir::ExprClosure {
        args,
        body,
        captures,
    })
}

/// Assemble a closure expression.
#[instrument(span = ast)]
fn expr_call_closure<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::ExprClosure,
) -> compile::Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, ast);

    let item = cx.q.item_for(ast.id).with_span(ast)?;

    let Some(meta) = cx.q.query_meta(ast, item.item, Default::default())? else {
        return Err(compile::Error::new(
            ast,
            ErrorKind::MissingItem {
                item: cx.q.pool.item(item.item).try_to_owned()?,
            },
        ));
    };

    let meta::Kind::Closure { call, do_move, .. } = meta.kind else {
        return Err(compile::Error::expected_meta(
            ast,
            meta.info(cx.q.pool)?,
            "a closure",
        ));
    };

    let captures = match cx.q.get_captures(meta.hash) {
        None => {
            tracing::trace!("queuing closure build entry");

            cx.scopes.push_captures()?;

            for (arg, _) in ast.args.as_slice() {
                fn_arg(cx, arg)?;
            }

            expr(cx, &ast.body)?;
            let layer = cx.scopes.pop().with_span(&ast.body)?;

            cx.q.set_used(&meta.item_meta)?;
            cx.q.inner.queue.try_push_back(BuildEntry {
                item_meta: meta.item_meta,
                build: Build::Closure(indexing::Closure {
                    ast: Box::try_new(ast.try_clone()?)?,
                    call,
                }),
            })?;

            cx.q.insert_captures(meta.hash, layer.captures())?;
            iter!(layer.captures())
        }
        Some(captures) => {
            iter!(captures, |capture| match capture {
                hir::OwnedName::SelfValue => hir::Name::SelfValue,
                hir::OwnedName::Str(name) => hir::Name::Str(alloc_str!(name.as_str())),
                hir::OwnedName::Id(id) => hir::Name::Id(*id),
            })
        }
    };

    if captures.is_empty() {
        return Ok(hir::ExprKind::Fn(meta.hash));
    }

    Ok(hir::ExprKind::CallClosure(alloc!(hir::ExprCallClosure {
        hash: meta.hash,
        do_move,
        captures,
    })))
}

#[instrument(span = ast)]
pub(crate) fn block<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::Block,
) -> compile::Result<hir::Block<'hir>> {
    alloc_with!(cx, ast);

    cx.scopes.push()?;

    let statements = iter!(&ast.statements, |ast| stmt(cx, ast)?);

    let layer = cx.scopes.pop().with_span(ast)?;

    let block = hir::Block {
        span: ast.span(),
        statements,
        drop: iter!(layer.into_drop_order()),
    };

    Ok(block)
}

#[instrument(span = ast)]
pub(crate) fn expr_range<'hir>(
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
        (Some(..) | None, None, ast::ExprRangeLimits::Closed(..)) => Err(compile::Error::msg(
            ast,
            "Unsupported range, you probably want `..` instead of `..=`",
        )),
    }
}

#[instrument(span = ast)]
pub(crate) fn expr_object<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::ExprObject,
) -> compile::Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, ast);

    let span = ast;
    let mut keys_dup = HashMap::new();

    let assignments = &mut *iter!(&ast.assignments, |(ast, _)| {
        let key = object_key(cx, &ast.key)?;

        if let Some(_existing) = keys_dup.try_insert(key.1, key.0)? {
            return Err(compile::Error::new(
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
                    return Err(compile::Error::new(
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
                    return Err(compile::Error::new(
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
            return Err(compile::Error::new(
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
                    return Err(compile::Error::new(
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
#[instrument(span = ast)]
pub(crate) fn expr<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::Expr,
) -> compile::Result<hir::Expr<'hir>> {
    alloc_with!(cx, ast);

    let in_path = cx.in_path.take();

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
            let body = block(cx, &ast.body)?;
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
            let body = block(cx, &ast.body)?;
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
            let binding = pat(cx, &ast.binding)?;
            let body = block(cx, &ast.body)?;

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
            pat: pat(cx, &ast.pat)?,
            expr: expr(cx, &ast.expr)?,
        })),
        ast::Expr::If(ast) => hir::ExprKind::If(alloc!(expr_if(cx, ast)?)),
        ast::Expr::Match(ast) => hir::ExprKind::Match(alloc!(hir::ExprMatch {
            expr: expr(cx, &ast.expr)?,
            branches: iter!(&ast.branches, |(ast, _)| {
                cx.scopes.push()?;

                let pat = pat(cx, &ast.pat)?;
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
            cx.in_path.set(in_path);
            hir::ExprKind::Group(alloc!(expr(cx, &ast.expr)?))
        }
        ast::Expr::Binary(ast) => {
            let rhs_needs = match &ast.op {
                ast::BinOp::As(..) | ast::BinOp::Is(..) | ast::BinOp::IsNot(..) => Needs::Type,
                _ => Needs::Value,
            };

            let lhs = expr(cx, &ast.lhs)?;

            let needs = cx.needs.replace(rhs_needs);
            let rhs = expr(cx, &ast.rhs)?;
            cx.needs.set(needs);

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
        ast::Expr::Select(ast) => hir::ExprKind::Select(alloc!(hir::ExprSelect {
            branches: iter!(&ast.branches, |(ast, _)| {
                match ast {
                    ast::ExprSelectBranch::Pat(ast) => {
                        cx.scopes.push()?;

                        let pat = pat(cx, &ast.pat)?;
                        let body = expr(cx, &ast.body)?;

                        let layer = cx.scopes.pop().with_span(ast)?;

                        hir::ExprSelectBranch::Pat(alloc!(hir::ExprSelectPatBranch {
                            pat,
                            expr: expr(cx, &ast.expr)?,
                            body,
                            drop: iter!(layer.into_drop_order()),
                        }))
                    }
                    ast::ExprSelectBranch::Default(ast) => {
                        hir::ExprSelectBranch::Default(alloc!(expr(cx, &ast.body)?))
                    }
                }
            })
        })),
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
        ast::Expr::MacroCall(ast) => match cx.q.builtin_macro_for(ast).with_span(ast)?.as_ref() {
            query::BuiltInMacro::Template(ast) => {
                let old = cx.in_template.replace(true);

                let result = hir::ExprKind::Template(alloc!(hir::BuiltInTemplate {
                    span: ast.span,
                    from_literal: ast.from_literal,
                    exprs: iter!(&ast.exprs, |ast| expr(cx, ast)?),
                }));

                cx.in_template.set(old);
                result
            }
            query::BuiltInMacro::Format(ast) => hir::ExprKind::Format(alloc!(hir::BuiltInFormat {
                span: ast.span,
                fill: ast.fill,
                align: ast.align,
                width: ast.width,
                precision: ast.precision,
                flags: ast.flags,
                format_type: ast.format_type,
                value: expr(cx, &ast.value)?,
            })),
            query::BuiltInMacro::File(ast) => hir::ExprKind::Lit(lit(cx, &ast.value)?),
            query::BuiltInMacro::Line(ast) => hir::ExprKind::Lit(lit(cx, &ast.value)?),
        },
    };

    Ok(hir::Expr {
        span: ast.span(),
        kind,
    })
}

#[instrument(span = ast)]
pub(crate) fn expr_if<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::ExprIf,
) -> compile::Result<hir::Conditional<'hir>> {
    alloc_with!(cx, ast);

    let length = 1 + ast.expr_else_ifs.len() + usize::from(ast.expr_else.is_some());

    let then = [(
        ast.if_.span().join(ast.block.span()),
        Some(&ast.condition),
        &ast.block,
    )]
    .into_iter();

    let else_ifs = ast
        .expr_else_ifs
        .iter()
        .map(|ast| (ast.span(), Some(&ast.condition), &ast.block));

    let fallback = ast
        .expr_else
        .iter()
        .map(|ast| (ast.span(), None, &ast.block));

    let branches = then.chain(else_ifs).chain(fallback);

    let branches = iter!(branches, length, |(span, c, b)| {
        let (condition, block, drop) = match c {
            Some(c) => {
                cx.scopes.push()?;

                let condition = condition(cx, c)?;
                let block = block(cx, b)?;

                let layer = cx.scopes.pop().with_span(ast)?;

                (
                    Some(&*alloc!(condition)),
                    block,
                    &*iter!(layer.into_drop_order()),
                )
            }
            None => {
                let block = block(cx, b)?;
                (None, block, &[][..])
            }
        };

        hir::ConditionalBranch {
            span,
            condition,
            block,
            drop,
        }
    });

    Ok(hir::Conditional { branches })
}

#[instrument(span = ast)]
pub(crate) fn lit<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::Lit,
) -> compile::Result<hir::Lit<'hir>> {
    alloc_with!(cx, ast);

    match ast {
        ast::Lit::Bool(lit) => Ok(hir::Lit::Bool(lit.value)),
        ast::Lit::Number(lit) => {
            let n = lit.resolve(resolve_context!(cx.q))?;

            match (n.value, n.suffix) {
                (ast::NumberValue::Float(n), _) => Ok(hir::Lit::Float(n)),
                (ast::NumberValue::Integer(int), Some(ast::NumberSuffix::Byte(..))) => {
                    let Some(n) = int.to_u8() else {
                        return Err(compile::Error::new(ast, ErrorKind::BadNumberOutOfBounds));
                    };

                    Ok(hir::Lit::Byte(n))
                }
                (ast::NumberValue::Integer(int), _) => {
                    let Some(n) = int.to_i64() else {
                        return Err(compile::Error::new(ast, ErrorKind::BadNumberOutOfBounds));
                    };

                    Ok(hir::Lit::Integer(n))
                }
            }
        }
        ast::Lit::Byte(lit) => {
            let b = lit.resolve(resolve_context!(cx.q))?;
            Ok(hir::Lit::Byte(b))
        }
        ast::Lit::Char(lit) => {
            let ch = lit.resolve(resolve_context!(cx.q))?;
            Ok(hir::Lit::Char(ch))
        }
        ast::Lit::Str(lit) => {
            let string = if cx.in_template.get() {
                lit.resolve_template_string(resolve_context!(cx.q))?
            } else {
                lit.resolve_string(resolve_context!(cx.q))?
            };

            Ok(hir::Lit::Str(alloc_str!(string.as_ref())))
        }
        ast::Lit::ByteStr(lit) => {
            let bytes = lit.resolve(resolve_context!(cx.q))?;
            Ok(hir::Lit::ByteStr(alloc_bytes!(bytes.as_ref())))
        }
    }
}

#[instrument(span = ast)]
pub(crate) fn expr_unary<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::ExprUnary,
) -> compile::Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, ast);

    // NB: special unary expressions.
    if let ast::UnOp::BorrowRef { .. } = ast.op {
        return Err(compile::Error::new(ast, ErrorKind::UnsupportedRef));
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
                return Err(compile::Error::new(ast, ErrorKind::BadNumberOutOfBounds));
            };

            Ok(hir::ExprKind::Lit(hir::Lit::Integer(n)))
        }
        _ => Err(compile::Error::new(ast, ErrorKind::BadNumberOutOfBounds)),
    }
}

/// Lower a block expression.
#[instrument(span = ast)]
pub(crate) fn expr_block<'hir>(
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
        return Ok(hir::ExprKind::Block(alloc!(block(cx, &ast.block)?)));
    }

    if cx.const_eval {
        // This only happens if the ast expression has not been indexed. Which
        // only occurs during certain kinds of constant evaluation. So we limit
        // evaluation to only support constant blocks.
        let ExprBlockKind::Const = kind else {
            return Err(compile::Error::msg(
                ast,
                "Only constant blocks are supported in this context",
            ));
        };

        return Ok(hir::ExprKind::Block(alloc!(block(cx, &ast.block)?)));
    };

    let item = cx.q.item_for(&ast.block).with_span(&ast.block)?;
    let meta = cx.lookup_meta(ast, item.item, GenericsParameters::default())?;

    match (kind, &meta.kind) {
        (ExprBlockKind::Async, &meta::Kind::AsyncBlock { call, do_move, .. }) => {
            let captures = match cx.q.get_captures(meta.hash) {
                None => {
                    tracing::trace!("queuing async block build entry");

                    cx.scopes.push_captures()?;
                    block(cx, &ast.block)?;
                    let layer = cx.scopes.pop().with_span(&ast.block)?;

                    cx.q.insert_captures(meta.hash, layer.captures())?;

                    cx.q.set_used(&meta.item_meta)?;
                    cx.q.inner.queue.try_push_back(BuildEntry {
                        item_meta: meta.item_meta,
                        build: Build::AsyncBlock(indexing::AsyncBlock {
                            ast: ast.block.try_clone()?,
                            call,
                        }),
                    })?;

                    iter!(layer.captures())
                }
                Some(captures) => {
                    iter!(captures, |capture| match capture {
                        hir::OwnedName::SelfValue => hir::Name::SelfValue,
                        hir::OwnedName::Str(name) => hir::Name::Str(alloc_str!(name.as_str())),
                        hir::OwnedName::Id(id) => hir::Name::Id(*id),
                    })
                }
            };

            Ok(hir::ExprKind::AsyncBlock(alloc!(hir::ExprAsyncBlock {
                hash: meta.hash,
                do_move,
                captures,
            })))
        }
        (ExprBlockKind::Const, meta::Kind::Const { .. }) => Ok(hir::ExprKind::Const(meta.hash)),
        _ => Err(compile::Error::expected_meta(
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
            return Err(compile::Error::new(
                ast,
                ErrorKind::MissingLoopLabel {
                    label: label.try_into()?,
                },
            ));
        } else {
            return Err(compile::Error::new(ast, ErrorKind::BreakOutsideOfLoop));
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
            return Err(compile::Error::new(
                ast,
                ErrorKind::MissingLoopLabel {
                    label: label.try_into()?,
                },
            ));
        } else {
            return Err(compile::Error::new(ast, ErrorKind::ContinueOutsideOfLoop));
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
            cx.scopes.define(hir::Name::SelfValue, ast)?;
            hir::FnArg::SelfValue(ast.span())
        }
        ast::FnArg::Pat(ast) => hir::FnArg::Pat(alloc!(pat(cx, ast)?)),
    })
}

/// Lower an assignment.
fn local<'hir>(cx: &mut Ctxt<'hir, '_, '_>, ast: &ast::Local) -> compile::Result<hir::Local<'hir>> {
    // Note: expression needs to be assembled before pattern, otherwise the
    // expression will see declarations in the pattern.
    let expr = expr(cx, &ast.expr)?;
    let pat = pat(cx, &ast.pat)?;

    Ok(hir::Local {
        span: ast.span(),
        pat,
        expr,
    })
}

/// Lower a statement
fn stmt<'hir>(cx: &mut Ctxt<'hir, '_, '_>, ast: &ast::Stmt) -> compile::Result<hir::Stmt<'hir>> {
    alloc_with!(cx, ast);

    Ok(match ast {
        ast::Stmt::Local(ast) => hir::Stmt::Local(alloc!(local(cx, ast)?)),
        ast::Stmt::Expr(ast) => hir::Stmt::Expr(alloc!(expr(cx, ast)?)),
        ast::Stmt::Semi(ast) => hir::Stmt::Semi(alloc!(expr(cx, &ast.expr)?)),
        ast::Stmt::Item(..) => hir::Stmt::Item(ast.span()),
    })
}

fn pat<'hir>(cx: &mut Ctxt<'hir, '_, '_>, ast: &ast::Pat) -> compile::Result<hir::Pat<'hir>> {
    fn filter((ast, _): &(ast::Pat, Option<ast::Comma>)) -> Option<&ast::Pat> {
        if matches!(ast, ast::Pat::Binding(..) | ast::Pat::Rest(..)) {
            return None;
        }

        Some(ast)
    }

    alloc_with!(cx, ast);

    let kind = match ast {
        ast::Pat::Ignore(..) => hir::PatKind::Ignore,
        ast::Pat::Path(ast) => {
            let named = cx.q.convert_path(&ast.path)?;
            let parameters = generics_parameters(cx, &named)?;

            let kind = 'ok: {
                if let Some(meta) = cx.try_lookup_meta(&ast, named.item, &parameters)? {
                    if let Some((0, kind)) = tuple_match_for(cx, &meta) {
                        break 'ok hir::PatPathKind::Kind(alloc!(kind));
                    }
                }

                if let Some(ident) = ast.path.try_as_ident() {
                    let name = alloc_str!(ident.resolve(resolve_context!(cx.q))?);
                    cx.scopes.define(hir::Name::Str(name), ast)?;
                    break 'ok hir::PatPathKind::Ident(name);
                }

                return Err(compile::Error::new(ast, ErrorKind::UnsupportedBinding));
            };

            hir::PatKind::Path(alloc!(kind))
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
                    return Err(compile::Error::expected_meta(
                        path,
                        meta.info(cx.q.pool)?,
                        "type that can be used in a tuple pattern",
                    ));
                };

                if !(args == count || count < args && is_open) {
                    return Err(compile::Error::new(
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
                            return Err(compile::Error::new(
                                path,
                                ErrorKind::UnsupportedPatternExpr,
                            ));
                        };

                        let key = alloc_str!(ident.resolve(resolve_context!(cx.q))?);
                        cx.scopes.define(hir::Name::Str(key), ident)?;
                        (key, hir::Binding::Ident(path.span(), key))
                    }
                    _ => {
                        return Err(compile::Error::new(pat, ErrorKind::UnsupportedPatternExpr));
                    }
                };

                if let Some(_existing) = keys_dup.try_insert(key, pat)? {
                    return Err(compile::Error::new(
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
                        return Err(compile::Error::expected_meta(
                            path,
                            meta.info(cx.q.pool)?,
                            "type that can be used in a struct pattern",
                        ));
                    };

                    for binding in bindings.iter() {
                        if !fields.remove(binding.key()) {
                            return Err(compile::Error::new(
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

                        return Err(compile::Error::new(
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
            return Err(compile::Error::new(ast, ErrorKind::UnsupportedPatternExpr));
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
                return Err(compile::Error::expected(ast, "object key"));
            };

            let string = ident.resolve(resolve_context!(cx.q))?;
            (ident, alloc_str!(string))
        }
    })
}

/// Lower the given path.
#[instrument(span = ast)]
pub(crate) fn expr_path<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::Path,
    in_path: bool,
) -> compile::Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, ast);

    if let Some(ast::PathKind::SelfValue) = ast.as_kind() {
        let Some(..) = cx.scopes.get(hir::Name::SelfValue)? else {
            return Err(compile::Error::new(ast, ErrorKind::MissingSelf));
        };

        return Ok(hir::ExprKind::Variable(hir::Name::SelfValue));
    }

    if let Needs::Value = cx.needs.get() {
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

    if let (Needs::Value, Some(local)) = (cx.needs.get(), ast.try_as_ident()) {
        let local = local.resolve(resolve_context!(cx.q))?;

        // light heuristics, treat it as a type error in case the first
        // character is uppercase.
        if !local.starts_with(char::is_uppercase) {
            return Err(compile::Error::new(
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
            parameters: parameters.parameters.into_iter().try_collect()?,
        }
    } else {
        ErrorKind::MissingItem {
            item: cx.q.pool.item(named.item).try_to_owned()?,
        }
    };

    Err(compile::Error::new(ast, kind))
}

/// Compile an item.
#[instrument(span = span)]
fn expr_path_meta<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    meta: &meta::Meta,
    span: &dyn Spanned,
) -> compile::Result<hir::ExprKind<'hir>> {
    alloc_with!(cx, span);

    if let Needs::Value = cx.needs.get() {
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
            _ => Err(compile::Error::expected_meta(
                span,
                meta.info(cx.q.pool)?,
                "something that can be used as a value",
            )),
        }
    } else {
        let Some(type_hash) = meta.type_hash_of() else {
            return Err(compile::Error::expected_meta(
                span,
                meta.info(cx.q.pool)?,
                "something that has a type",
            ));
        };

        Ok(hir::ExprKind::Type(Type::new(type_hash)))
    }
}

fn condition<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::Condition,
) -> compile::Result<hir::Condition<'hir>> {
    alloc_with!(cx, ast);

    Ok(match ast {
        ast::Condition::Expr(ast) => hir::Condition::Expr(alloc!(expr(cx, ast)?)),
        ast::Condition::ExprLet(ast) => hir::Condition::ExprLet(alloc!(hir::ExprLet {
            pat: pat(cx, &ast.pat)?,
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
            return Err(compile::Error::new(pat, ErrorKind::UnsupportedPatternRest));
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

fn generics_parameters(
    cx: &mut Ctxt<'_, '_, '_>,
    named: &Named<'_>,
) -> compile::Result<GenericsParameters> {
    let mut parameters = GenericsParameters {
        trailing: named.trailing,
        parameters: [None, None],
    };

    for (value, o) in named
        .parameters
        .iter()
        .zip(parameters.parameters.iter_mut())
    {
        if let &Some((_, generics)) = value {
            let mut builder = ParametersBuilder::new();

            for (s, _) in generics {
                let hir::ExprKind::Type(ty) = expr(cx, &s.expr)?.kind else {
                    return Err(compile::Error::new(s, ErrorKind::UnsupportedGenerics));
                };

                builder.add(ty.into_hash());
            }

            *o = Some(builder.finish());
        }
    }

    Ok(parameters)
}

/// Convert into a call expression.
#[instrument(span = ast)]
fn expr_call<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::ExprCall,
) -> compile::Result<hir::ExprCall<'hir>> {
    pub(crate) fn find_path(ast: &ast::Expr) -> Option<&ast::Path> {
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

    let expr = cx.in_path(true, |cx| expr(cx, &ast.expr))?;

    let call = 'ok: {
        match expr.kind {
            hir::ExprKind::Variable(name) => {
                break 'ok hir::Call::Var { name };
            }
            hir::ExprKind::Path => {
                let Some(path) = find_path(&ast.expr) else {
                    return Err(compile::Error::msg(&ast.expr, "Expected path"));
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
                            return Err(compile::Error::new(
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
                            return Err(compile::Error::new(
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
                    meta::Kind::Function { .. } => (),
                    meta::Kind::ConstFn { id, .. } => {
                        let id = *id;
                        let from = cx.q.item_for(ast.id).with_span(ast)?;

                        break 'ok hir::Call::ConstFn {
                            from_module: from.module,
                            from_item: from.item,
                            id,
                        };
                    }
                    _ => {
                        return Err(compile::Error::expected_meta(
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

#[instrument(span = ast)]
fn expr_field_access<'hir>(
    cx: &mut Ctxt<'hir, '_, '_>,
    ast: &ast::ExprFieldAccess,
) -> compile::Result<hir::ExprFieldAccess<'hir>> {
    alloc_with!(cx, ast);

    let expr_field = match &ast.expr_field {
        ast::ExprField::LitNumber(ast) => {
            let number = ast.resolve(resolve_context!(cx.q))?;

            let Some(index) = number.as_tuple_index() else {
                return Err(compile::Error::new(
                    ast,
                    ErrorKind::UnsupportedTupleIndex { number },
                ));
            };

            hir::ExprField::Index(index)
        }
        ast::ExprField::Path(ast) => {
            let Some((ident, generics)) = ast.try_as_ident_generics() else {
                return Err(compile::Error::new(ast, ErrorKind::BadFieldAccess));
            };

            let ident = alloc_str!(ident.resolve(resolve_context!(cx.q))?);

            match generics {
                Some(generics) => {
                    let mut builder = ParametersBuilder::new();

                    for (s, _) in generics {
                        let hir::ExprKind::Type(ty) = expr(cx, &s.expr)?.kind else {
                            return Err(compile::Error::new(s, ErrorKind::UnsupportedGenerics));
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
