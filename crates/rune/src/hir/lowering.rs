use core::cell::Cell;
use core::mem::replace;
use core::ops::Neg;

use crate::no_std::collections::{HashMap, HashSet};
use crate::no_std::prelude::*;

use num::ToPrimitive;

use crate::ast::{self, Span, Spanned};
use crate::compile::meta;
use crate::compile::v1::GenericsParameters;
use crate::compile::{
    self, CompileErrorKind, HirErrorKind, Item, ItemId, Location, ParseErrorKind, WithSpan,
};
use crate::hash::{Hash, ParametersBuilder};
use crate::hir;
use crate::parse::Resolve;
use crate::query::{self, Named, Query};
use crate::SourceId;

/// Allocator indirection to simplify lifetime management.
#[rustfmt::skip]
macro_rules! alloc_with {
    ($ctx:expr, $span:expr) => {
        #[allow(unused)]
        macro_rules! alloc {
            ($value:expr) => {
                $ctx.arena.alloc($value).map_err(|e| {
                    compile::Error::new(
                        $span,
                        HirErrorKind::ArenaAllocError {
                            requested: e.requested,
                        },
                    )
                })?
            };
        }

        #[allow(unused)]
        macro_rules! option {
            ($value:expr, |$pat:pat_param| $closure:expr) => {
                match $value {
                    Some($pat) => {
                        Some(&*alloc!($closure))
                    }
                    None => {
                        None
                    }
                }
            };
        }

        #[allow(unused)]
        macro_rules! iter {
            ($iter:expr) => {
                iter!($iter, |value| value)
            };

            ($iter:expr, |$pat:pat_param| $closure:expr) => {
                iter!($iter, it, ExactSizeIterator::len(&it), |$pat| $closure)
            };

            ($iter:expr, $len:expr, |$pat:pat_param| $closure:expr) => {
                iter!($iter, it, $len, |$pat| $closure)
            };

            ($iter:expr, $it:ident, $len:expr, |$pat:pat_param| $closure:expr) => {{
                let mut $it = IntoIterator::into_iter($iter);

                let mut writer = match $ctx.arena.alloc_iter($len) {
                    Ok(writer) => writer,
                    Err(e) => {
                        return Err(compile::Error::new(
                            $span,
                            HirErrorKind::ArenaAllocError {
                                requested: e.requested,
                            },
                        ));
                    }
                };
        
                while let Some($pat) = $it.next() {
                    if let Err(e) = writer.write($closure) {
                        return Err(compile::Error::new(
                            $span,
                            HirErrorKind::ArenaWriteSliceOutOfBounds { index: e.index },
                        ));
                    }
                }

                writer.finish()
            }};
        }

        #[allow(unused)]
        macro_rules! alloc_str {
            ($value:expr) => {
                match $ctx.arena.alloc_str($value) {
                    Ok(string) => string,
                    Err(e) => return Err(compile::Error::new(
                        $span,
                        HirErrorKind::ArenaAllocError {
                            requested: e.requested,
                        },
                    )),
                }
            };
        }

        #[allow(unused)]
        macro_rules! alloc_bytes {
            ($value:expr) => {
                match $ctx.arena.alloc_bytes($value) {
                    Ok(bytes) => bytes,
                    Err(e) => return Err(compile::Error::new(
                        $span,
                        HirErrorKind::ArenaAllocError {
                            requested: e.requested,
                        },
                    )),
                }
            };
        }
    };
}

pub(crate) struct Ctx<'hir, 'a> {
    /// Arena used for allocations.
    arena: &'hir hir::arena::Arena,
    q: Query<'a>,
    source_id: SourceId,
    in_template: Cell<bool>,
    scope: hir::Scope,
    scopes: hir::Scopes<'hir>,
}

impl<'hir, 'a> Ctx<'hir, 'a> {
    /// Construct a new context.
    pub(crate) fn new(arena: &'hir hir::arena::Arena, q: Query<'a>, source_id: SourceId) -> Self {
        Self {
            arena,
            q,
            source_id,
            in_template: Cell::new(false),
            scope: hir::Scopes::ROOT,
            scopes: hir::Scopes::default(),
        }
    }

    #[allow(unused)]
    pub(crate) fn try_lookup_meta(
        &mut self,
        span: Span,
        item: ItemId,
        parameters: &GenericsParameters,
    ) -> compile::Result<Option<meta::Meta>> {
        self.q
            .try_lookup_meta(Location::new(self.source_id, span), item, parameters)
    }

    pub(crate) fn lookup_meta(
        &mut self,
        span: Span,
        item: ItemId,
        parameters: impl AsRef<GenericsParameters>,
    ) -> compile::Result<meta::Meta> {
        self.q
            .lookup_meta(Location::new(self.source_id, span), item, parameters)
    }
}

/// Lower a function item.
pub(crate) fn item_fn<'hir>(
    ctx: &mut Ctx<'hir, '_>,
    ast: &ast::ItemFn,
) -> compile::Result<hir::ItemFn<'hir>> {
    alloc_with!(ctx, ast);

    Ok(hir::ItemFn {
        id: ast.id,
        span: ast.span(),
        args: iter!(&ast.args, |(ast, _)| fn_arg(ctx, ast)?),
        body: alloc!(block(ctx, &ast.body)?),
    })
}

/// Lower a closure expression.
pub(crate) fn expr_closure<'hir>(
    ctx: &mut Ctx<'hir, '_>,
    ast: &ast::ExprClosure,
) -> compile::Result<hir::ExprClosure<'hir>> {
    alloc_with!(ctx, ast);

    Ok(hir::ExprClosure {
        id: ast.id,
        args: match &ast.args {
            ast::ExprClosureArgs::Empty { .. } => &[],
            ast::ExprClosureArgs::List { args, .. } => {
                iter!(args, |(ast, _)| fn_arg(ctx, ast)?)
            }
        },
        body: alloc!(expr(ctx, &ast.body)?),
    })
}

/// Lower the specified block.
pub(crate) fn block<'hir>(
    ctx: &mut Ctx<'hir, '_>,
    ast: &ast::Block,
) -> compile::Result<hir::Block<'hir>> {
    alloc_with!(ctx, ast);

    let scope = ctx.scopes.push(ctx.scope);
    let scope = replace(&mut ctx.scope, scope);

    let statements = iter!(&ast.statements, |ast| stmt(ctx, ast)?);
    let layer = ctx
        .scopes
        .pop(replace(&mut ctx.scope, scope))
        .with_span(ast)?;

    let block = hir::Block {
        id: ast.id,
        span: ast.span(),
        statements,
        drop: iter!(layer.into_drop_order()),
    };

    Ok(block)
}

pub(crate) fn expr_object<'hir>(
    ctx: &mut Ctx<'hir, '_>,
    ast: &ast::ExprObject,
) -> compile::Result<hir::ExprKind<'hir>> {
    alloc_with!(ctx, ast);

    let span = ast.span();
    let mut keys_dup = HashMap::new();

    let assignments = &*iter!(&ast.assignments, |(ast, _)| {
        let key = object_key(ctx, &ast.key)?;

        if let Some(existing) = keys_dup.insert(key.1, key.0) {
            return Err(compile::Error::new(
                key.0,
                CompileErrorKind::DuplicateObjectKey {
                    existing,
                    object: key.0,
                },
            ));
        }

        hir::FieldAssign {
            key,
            assign: option!(&ast.assign, |(_, ast)| expr(ctx, ast)?),
        }
    });

    let check_object_fields = |fields: &HashSet<_>, item: &Item| {
        let mut fields = fields.clone();

        for assign in assignments {
            if !fields.remove(assign.key.1) {
                return Err(compile::Error::new(
                    assign.key.0,
                    CompileErrorKind::LitObjectNotField {
                        field: assign.key.1.into(),
                        item: item.to_owned(),
                    },
                ));
            }
        }

        if let Some(field) = fields.into_iter().next() {
            return Err(compile::Error::new(
                span,
                CompileErrorKind::LitObjectMissingField {
                    field,
                    item: item.to_owned(),
                },
            ));
        }

        Ok(())
    };

    let path = object_ident(ctx, &ast.ident)?;

    let kind = match path {
        Some(path) => {
            let named = ctx.q.convert_path(path)?;
            let parameters = generics_parameters(ctx, &named)?;
            let meta = ctx.lookup_meta(path.span(), named.item, parameters)?;
            let item = ctx.q.pool.item(meta.item_meta.item);

            match &meta.kind {
                meta::Kind::Struct {
                    fields: meta::Fields::Empty,
                    ..
                } => {
                    check_object_fields(&HashSet::new(), item)?;
                    hir::ExprObjectKind::UnitStruct { hash: meta.hash }
                }
                meta::Kind::Struct {
                    fields: meta::Fields::Named(st),
                    ..
                } => {
                    check_object_fields(&st.fields, item)?;
                    hir::ExprObjectKind::Struct { hash: meta.hash }
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
                        CompileErrorKind::UnsupportedLitObject {
                            meta: meta.info(ctx.q.pool),
                        },
                    ));
                }
            }
        }
        None => hir::ExprObjectKind::Anonymous,
    };

    Ok(hir::ExprKind::Object(alloc!(hir::ExprObject {
        kind,
        assignments,
    })))
}

/// Lower an expression.
pub(crate) fn expr<'hir>(
    ctx: &mut Ctx<'hir, '_>,
    ast: &ast::Expr,
) -> compile::Result<hir::Expr<'hir>> {
    alloc_with!(ctx, ast);

    let kind = match ast {
        ast::Expr::Path(ast) => hir::ExprKind::Path(alloc!(path(ctx, ast)?)),
        ast::Expr::Assign(ast) => hir::ExprKind::Assign(alloc!(hir::ExprAssign {
            lhs: alloc!(expr(ctx, &ast.lhs)?),
            rhs: alloc!(expr(ctx, &ast.rhs)?),
        })),
        // TODO: lower all of these loop constructs to the same loop-like
        // representation. We only do different ones here right now since it's
        // easier when refactoring.
        ast::Expr::While(ast) => {
            let scope = ctx.scopes.push(ctx.scope);

            let condition = condition(ctx, &ast.condition)?;
            let body = block(ctx, &ast.body)?;

            let layer = ctx
                .scopes
                .pop(replace(&mut ctx.scope, scope))
                .with_span(ast)?;

            hir::ExprKind::Loop(alloc!(hir::ExprLoop {
                label: option!(&ast.label, |(ast, _)| label(ctx, ast)?),
                condition: Some(alloc!(condition)),
                body: alloc!(body),
                drop: iter!(layer.into_drop_order()),
            }))
        }
        ast::Expr::Loop(ast) => hir::ExprKind::Loop(alloc!(hir::ExprLoop {
            label: option!(&ast.label, |(ast, _)| label(ctx, ast)?),
            condition: None,
            body: alloc!(block(ctx, &ast.body)?),
            drop: &[],
        })),
        ast::Expr::For(ast) => {
            let iter = expr(ctx, &ast.iter)?;

            let scope = ctx.scopes.push(ctx.scope);

            let binding = pat(ctx, &ast.binding)?;
            let body = block(ctx, &ast.body)?;

            let layer = ctx
                .scopes
                .pop(replace(&mut ctx.scope, scope))
                .with_span(ast)?;

            hir::ExprKind::For(alloc!(hir::ExprFor {
                label: option!(&ast.label, |(ast, _)| label(ctx, ast)?),
                binding: alloc!(binding),
                iter: alloc!(iter),
                body: alloc!(body),
                drop: iter!(layer.into_drop_order()),
            }))
        }
        ast::Expr::Let(ast) => hir::ExprKind::Let(alloc!(hir::ExprLet {
            pat: alloc!(pat(ctx, &ast.pat)?),
            expr: alloc!(expr(ctx, &ast.expr)?),
        })),
        ast::Expr::If(ast) => hir::ExprKind::If(alloc!(expr_if(ctx, ast)?)),
        ast::Expr::Match(ast) => hir::ExprKind::Match(alloc!(hir::ExprMatch {
            expr: alloc!(expr(ctx, &ast.expr)?),
            branches: iter!(&ast.branches, |(ast, _)| {
                let scope = ctx.scopes.push(ctx.scope);

                let scope = replace(&mut ctx.scope, scope);
                let pat = alloc!(pat(ctx, &ast.pat)?);
                let condition = option!(&ast.condition, |(_, ast)| expr(ctx, ast)?);
                let body = alloc!(expr(ctx, &ast.body)?);

                let layer = ctx
                    .scopes
                    .pop(replace(&mut ctx.scope, scope))
                    .with_span(ast)?;

                hir::ExprMatchBranch {
                    span: ast.span(),
                    pat,
                    condition,
                    body,
                    drop: iter!(layer.into_drop_order()),
                }
            }),
        })),
        ast::Expr::Call(ast) => hir::ExprKind::Call(alloc!(hir::ExprCall {
            id: ast.id,
            expr: alloc!(expr(ctx, &ast.expr)?),
            args: iter!(&ast.args, |(ast, _)| expr(ctx, ast)?),
        })),
        ast::Expr::FieldAccess(ast) => hir::ExprKind::FieldAccess(alloc!(hir::ExprFieldAccess {
            expr: alloc!(expr(ctx, &ast.expr)?),
            expr_field: alloc!(match &ast.expr_field {
                ast::ExprField::Path(ast) => hir::ExprField::Path(alloc!(path(ctx, ast)?)),
                ast::ExprField::LitNumber(ast) => hir::ExprField::LitNumber(alloc!(*ast)),
            }),
        })),
        ast::Expr::Empty(ast) => hir::ExprKind::Group(alloc!(expr(ctx, &ast.expr)?)),
        ast::Expr::Binary(ast) => hir::ExprKind::Binary(alloc!(hir::ExprBinary {
            lhs: alloc!(expr(ctx, &ast.lhs)?),
            op: ast.op,
            rhs: alloc!(expr(ctx, &ast.rhs)?),
        })),
        ast::Expr::Unary(ast) => expr_unary(ctx, ast)?,
        ast::Expr::Index(ast) => hir::ExprKind::Index(alloc!(hir::ExprIndex {
            target: alloc!(expr(ctx, &ast.target)?),
            index: alloc!(expr(ctx, &ast.index)?),
        })),
        ast::Expr::Block(ast) => expr_block(ctx, ast)?,
        ast::Expr::Break(ast) => {
            hir::ExprKind::Break(option!(ast.expr.as_deref(), |ast| match ast {
                ast::ExprBreakValue::Expr(ast) =>
                    hir::ExprBreakValue::Expr(alloc!(expr(ctx, ast)?)),
                ast::ExprBreakValue::Label(ast) =>
                    hir::ExprBreakValue::Label(alloc!(label(ctx, ast)?)),
            }))
        }
        ast::Expr::Continue(ast) => {
            hir::ExprKind::Continue(option!(&ast.label, |ast| label(ctx, ast)?))
        }
        ast::Expr::Yield(ast) => hir::ExprKind::Yield(option!(&ast.expr, |ast| expr(ctx, ast)?)),
        ast::Expr::Return(ast) => hir::ExprKind::Return(option!(&ast.expr, |ast| expr(ctx, ast)?)),
        ast::Expr::Await(ast) => hir::ExprKind::Await(alloc!(expr(ctx, &ast.expr)?)),
        ast::Expr::Try(ast) => hir::ExprKind::Try(alloc!(expr(ctx, &ast.expr)?)),
        ast::Expr::Select(ast) => hir::ExprKind::Select(alloc!(hir::ExprSelect {
            branches: iter!(&ast.branches, |(ast, _)| {
                match ast {
                    ast::ExprSelectBranch::Pat(ast) => {
                        let scope = ctx.scopes.push(ctx.scope);

                        let pat = alloc!(pat(ctx, &ast.pat)?);
                        let body = alloc!(expr(ctx, &ast.body)?);

                        let layer = ctx
                            .scopes
                            .pop(replace(&mut ctx.scope, scope))
                            .with_span(ast)?;

                        hir::ExprSelectBranch::Pat(alloc!(hir::ExprSelectPatBranch {
                            pat,
                            expr: alloc!(expr(ctx, &ast.expr)?),
                            body,
                            drop: iter!(layer.into_drop_order()),
                        }))
                    }
                    ast::ExprSelectBranch::Default(ast) => {
                        hir::ExprSelectBranch::Default(alloc!(expr(ctx, &ast.body)?))
                    }
                }
            })
        })),
        ast::Expr::Closure(ast) => hir::ExprKind::Closure(alloc!(expr_closure(ctx, ast)?)),
        ast::Expr::Lit(ast) => hir::ExprKind::Lit(lit(ast, ctx, &ast.lit)?),
        ast::Expr::Object(ast) => expr_object(ctx, ast)?,
        ast::Expr::Tuple(ast) => hir::ExprKind::Tuple(alloc!(hir::ExprSeq {
            items: iter!(&ast.items, |(ast, _)| expr(ctx, ast)?),
        })),
        ast::Expr::Vec(ast) => hir::ExprKind::Vec(alloc!(hir::ExprSeq {
            items: iter!(&ast.items, |(ast, _)| expr(ctx, ast)?),
        })),
        ast::Expr::Range(ast) => hir::ExprKind::Range(alloc!(hir::ExprRange {
            from: option!(&ast.from, |ast| expr(ctx, ast)?),
            limits: match ast.limits {
                ast::ExprRangeLimits::HalfOpen(_) => hir::ExprRangeLimits::HalfOpen,
                ast::ExprRangeLimits::Closed(_) => hir::ExprRangeLimits::Closed,
            },
            to: option!(&ast.to, |ast| expr(ctx, ast)?),
        })),
        ast::Expr::Group(ast) => hir::ExprKind::Group(alloc!(expr(ctx, &ast.expr)?)),
        ast::Expr::MacroCall(ast) => match ctx.q.builtin_macro_for(ast)?.as_ref() {
            query::BuiltInMacro::Template(ast) => {
                let old = ctx.in_template.replace(true);

                let result = hir::ExprKind::Template(alloc!(hir::BuiltInTemplate {
                    span: ast.span,
                    from_literal: ast.from_literal,
                    exprs: iter!(&ast.exprs, |ast| expr(ctx, ast)?),
                }));

                ctx.in_template.set(old);
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
                value: alloc!(expr(ctx, &ast.value)?),
            })),
            query::BuiltInMacro::File(ast) => hir::ExprKind::Lit(lit(ast, ctx, &ast.value)?),
            query::BuiltInMacro::Line(ast) => hir::ExprKind::Lit(lit(ast, ctx, &ast.value)?),
        },
    };

    Ok(hir::Expr {
        span: ast.span(),
        kind,
    })
}

pub(crate) fn expr_if<'hir>(
    ctx: &mut Ctx<'hir, '_>,
    ast: &ast::ExprIf,
) -> compile::Result<hir::Conditional<'hir>> {
    alloc_with!(ctx, ast);

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
                let scope = ctx.scopes.push(ctx.scope);

                let condition = condition(ctx, c)?;
                let block = block(ctx, b)?;

                let layer = ctx
                    .scopes
                    .pop(replace(&mut ctx.scope, scope))
                    .with_span(ast)?;

                (
                    Some(&*alloc!(condition)),
                    &*alloc!(block),
                    &*iter!(layer.into_drop_order()),
                )
            }
            None => {
                let block = block(ctx, b)?;
                (None, &*alloc!(block), &[][..])
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

pub(crate) fn lit<'hir>(
    span: &dyn Spanned,
    ctx: &mut Ctx<'hir, '_>,
    ast: &ast::Lit,
) -> compile::Result<hir::Lit<'hir>> {
    alloc_with!(ctx, span);

    match ast {
        ast::Lit::Bool(lit) => Ok(hir::Lit::Bool(lit.value)),
        ast::Lit::Number(lit) => match lit.resolve(resolve_context!(ctx.q))? {
            ast::Number::Float(n) => Ok(hir::Lit::Float(n)),
            ast::Number::Integer(int) => {
                let n = match int.to_i64() {
                    Some(n) => n,
                    None => {
                        return Err(compile::Error::new(
                            ast,
                            ParseErrorKind::BadNumberOutOfBounds,
                        ));
                    }
                };

                Ok(hir::Lit::Integer(n))
            }
        },
        ast::Lit::Byte(lit) => {
            let b = lit.resolve(resolve_context!(ctx.q))?;
            Ok(hir::Lit::Byte(b))
        }
        ast::Lit::Char(lit) => {
            let ch = lit.resolve(resolve_context!(ctx.q))?;
            Ok(hir::Lit::Char(ch))
        }
        ast::Lit::Str(lit) => {
            let string = if ctx.in_template.get() {
                lit.resolve_template_string(resolve_context!(ctx.q))?
            } else {
                lit.resolve_string(resolve_context!(ctx.q))?
            };

            Ok(hir::Lit::Str(alloc_str!(string.as_ref())))
        }
        ast::Lit::ByteStr(lit) => {
            let bytes = lit.resolve(resolve_context!(ctx.q))?;
            Ok(hir::Lit::ByteStr(alloc_bytes!(bytes.as_ref())))
        }
    }
}

pub(crate) fn expr_unary<'hir>(
    ctx: &mut Ctx<'hir, '_>,
    ast: &ast::ExprUnary,
) -> compile::Result<hir::ExprKind<'hir>> {
    alloc_with!(ctx, ast);

    // NB: special unary expressions.
    if let ast::UnOp::BorrowRef { .. } = ast.op {
        return Err(compile::Error::new(ast, CompileErrorKind::UnsupportedRef));
    }

    let (ast::UnOp::Neg(..), ast::Expr::Lit(ast::ExprLit { lit: ast::Lit::Number(n), .. })) = (ast.op, &*ast.expr) else {
        return Ok(hir::ExprKind::Unary(alloc!(hir::ExprUnary {
            op: ast.op,
            expr: alloc!(expr(ctx, &ast.expr)?),
        })));
    };

    match n.resolve(resolve_context!(ctx.q))? {
        ast::Number::Float(n) => Ok(hir::ExprKind::Lit(hir::Lit::Float(-n))),
        ast::Number::Integer(int) => {
            let n = match int.neg().to_i64() {
                Some(n) => n,
                None => {
                    return Err(compile::Error::new(
                        ast,
                        ParseErrorKind::BadNumberOutOfBounds,
                    ));
                }
            };

            Ok(hir::ExprKind::Lit(hir::Lit::Integer(n)))
        }
    }
}

/// Lower a block expression.
pub(crate) fn expr_block<'hir>(
    ctx: &mut Ctx<'hir, '_>,
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

    alloc_with!(ctx, ast);

    let kind = match (&ast.async_token, &ast.const_token) {
        (Some(..), None) => ExprBlockKind::Async,
        (None, Some(..)) => ExprBlockKind::Const,
        _ => ExprBlockKind::Default,
    };

    if let ExprBlockKind::Default = kind {
        return Ok(hir::ExprKind::Block(alloc!(block(ctx, &ast.block)?)));
    }

    let Some(id) = ast.block.id.get() else {
        // This only happens if the ast expression has not been indexed. Which
        // only occurs during certain kinds of constant evaluation. So we limit
        // evaluation to only support constant blocks.
        let ExprBlockKind::Const = kind else {
            return Err(compile::Error::msg(
                ast,
                "only constant blocks are supported in this context",
            ));
        };

        return Ok(hir::ExprKind::Block(alloc!(block(ctx, &ast.block)?)));
    };

    let item = ctx.q.item_for(&ast.block)?;
    let meta = ctx.lookup_meta(ast.span(), item.item, GenericsParameters::default())?;

    match (kind, &meta.kind) {
        (
            ExprBlockKind::Async,
            meta::Kind::AsyncBlock {
                captures, do_move, ..
            },
        ) => {
            let captures = iter!(captures.as_ref(), |string| alloc_str!(string));

            Ok(hir::ExprKind::AsyncBlock(alloc!(hir::ExprAsyncBlock {
                hash: meta.hash,
                do_move: *do_move,
                captures,
            })))
        }
        (ExprBlockKind::Const, meta::Kind::Const { const_value }) => {
            ctx.q.insert_const_value(id, const_value.clone());
            Ok(hir::ExprKind::Const(id))
        }
        _ => Err(compile::Error::expected_meta(
            ast,
            meta.info(ctx.q.pool),
            "async or const block",
        )),
    }
}

/// Lower a function argument.
fn fn_arg<'hir>(ctx: &mut Ctx<'hir, '_>, ast: &ast::FnArg) -> compile::Result<hir::FnArg<'hir>> {
    alloc_with!(ctx, ast);

    Ok(match ast {
        ast::FnArg::SelfValue(ast) => hir::FnArg::SelfValue(ast.span()),
        ast::FnArg::Pat(ast) => hir::FnArg::Pat(alloc!(pat(ctx, ast)?)),
    })
}

/// Lower an assignment.
fn local<'hir>(ctx: &mut Ctx<'hir, '_>, ast: &ast::Local) -> compile::Result<hir::Local<'hir>> {
    alloc_with!(ctx, ast);

    Ok(hir::Local {
        span: ast.span(),
        pat: alloc!(pat(ctx, &ast.pat)?),
        expr: alloc!(expr(ctx, &ast.expr)?),
    })
}

/// Lower a statement
fn stmt<'hir>(ctx: &mut Ctx<'hir, '_>, ast: &ast::Stmt) -> compile::Result<hir::Stmt<'hir>> {
    alloc_with!(ctx, ast);

    Ok(match ast {
        ast::Stmt::Local(ast) => hir::Stmt::Local(alloc!(local(ctx, ast)?)),
        ast::Stmt::Expr(ast) => hir::Stmt::Expr(alloc!(expr(ctx, ast)?)),
        ast::Stmt::Semi(ast) => hir::Stmt::Semi(alloc!(expr(ctx, &ast.expr)?)),
        ast::Stmt::Item(..) => hir::Stmt::Item(ast.span()),
    })
}

fn pat<'hir>(ctx: &mut Ctx<'hir, '_>, ast: &ast::Pat) -> compile::Result<hir::Pat<'hir>> {
    alloc_with!(ctx, ast);

    Ok(hir::Pat {
        span: ast.span(),
        kind: match ast {
            ast::Pat::Ignore(..) => hir::PatKind::Ignore,
            ast::Pat::Rest(..) => hir::PatKind::Rest,
            ast::Pat::Path(ast) => {
                let path = path(ctx, &ast.path)?;
                let named = ctx.q.convert_path(&path)?;
                let parameters = generics_parameters(ctx, &named)?;

                let kind = 'ok: {
                    if let Some(meta) = ctx.try_lookup_meta(path.span, named.item, &parameters)? {
                        if let Some((0, kind)) = tuple_match_for(ctx, &meta) {
                            break 'ok hir::PatPathKind::Kind(alloc!(kind));
                        }
                    }

                    if let Some(ident) = path.try_as_ident() {
                        let ident = alloc_str!(ident.resolve(resolve_context!(ctx.q))?);
                        let variable = ctx.scopes.define(ctx.scope, ident).with_span(ast)?;
                        break 'ok hir::PatPathKind::Ident(ident, variable);
                    }

                    return Err(compile::Error::new(
                        path.span,
                        CompileErrorKind::UnsupportedBinding,
                    ));
                };

                hir::PatKind::Path(alloc!(kind))
            }
            ast::Pat::Lit(ast) => hir::PatKind::Lit(alloc!(expr(ctx, &ast.expr)?)),
            ast::Pat::Vec(ast) => {
                let items = iter!(&ast.items, |(ast, _)| pat(ctx, ast)?);
                let (is_open, count) = pat_items_count(items)?;

                hir::PatKind::Vec(alloc!(hir::PatItems {
                    kind: hir::PatItemsKind::Anonymous { count, is_open },
                    items,
                    is_open,
                    count,
                    bindings: &[],
                }))
            }
            ast::Pat::Tuple(ast) => {
                let items = iter!(&ast.items, |(ast, _)| pat(ctx, ast)?);
                let (is_open, count) = pat_items_count(items)?;

                let path = option!(&ast.path, |ast| path(ctx, ast)?);

                let kind = if let Some(path) = path {
                    let named = ctx.q.convert_path(path)?;
                    let parameters = generics_parameters(ctx, &named)?;
                    let meta = ctx.lookup_meta(path.span(), named.item, parameters)?;

                    // Treat the current meta as a tuple and get the number of arguments it
                    // should receive and the type check that applies to it.
                    let Some((args, kind)) = tuple_match_for(ctx, &meta) else {
                        return Err(compile::Error::expected_meta(
                            path,
                            meta.info(ctx.q.pool),
                            "type that can be used in a tuple pattern",
                        ));
                    };

                    if !(args == count || count < args && is_open) {
                        return Err(compile::Error::new(
                            path,
                            CompileErrorKind::UnsupportedArgumentCount {
                                meta: meta.info(ctx.q.pool),
                                expected: args,
                                actual: count,
                            },
                        ));
                    }

                    kind
                } else {
                    hir::PatItemsKind::Anonymous { count, is_open }
                };

                hir::PatKind::Tuple(alloc!(hir::PatItems {
                    kind,
                    items,
                    is_open,
                    count,
                    bindings: &[],
                }))
            }
            ast::Pat::Object(ast) => {
                let items = iter!(&ast.items, |(ast, _)| pat(ctx, ast)?);
                let (is_open, count) = pat_items_count(items)?;

                let mut keys_dup = HashMap::new();

                let bindings = iter!(ast.items.iter().take(count), |(pat, _)| {
                    let span = pat.span();

                    let (key, binding) = match pat {
                        ast::Pat::Binding(binding) => {
                            let (span, key) = object_key(ctx, &binding.key)?;
                            (
                                key,
                                hir::Binding::Binding(
                                    span,
                                    key,
                                    alloc!(self::pat(ctx, &binding.pat)?),
                                ),
                            )
                        }
                        ast::Pat::Path(path) => {
                            let Some(ident) = path.path.try_as_ident() else {
                                return Err(compile::Error::new(
                                    path,
                                    CompileErrorKind::UnsupportedPatternExpr,
                                ));
                            };

                            let key = alloc_str!(ident.resolve(resolve_context!(ctx.q))?);
                            (key, hir::Binding::Ident(path.span(), key))
                        }
                        _ => {
                            return Err(compile::Error::new(
                                span,
                                CompileErrorKind::UnsupportedPatternExpr,
                            ));
                        }
                    };

                    if let Some(existing) = keys_dup.insert(key, span) {
                        return Err(compile::Error::new(
                            span,
                            CompileErrorKind::DuplicateObjectKey {
                                existing,
                                object: span,
                            },
                        ));
                    }

                    binding
                });

                let path = object_ident(ctx, &ast.ident)?;

                let kind = match path {
                    Some(path) => {
                        let span = path.span();

                        let named = ctx.q.convert_path(path)?;
                        let parameters = generics_parameters(ctx, &named)?;
                        let meta = ctx.lookup_meta(span, named.item, parameters)?;

                        let Some((st, kind)) = struct_match_for(ctx, &meta) else {
                            return Err(compile::Error::expected_meta(
                                span,
                                meta.info(ctx.q.pool),
                                "type that can be used in a struct pattern",
                            ));
                        };

                        let mut fields = st.fields.clone();

                        for binding in bindings.iter() {
                            if !fields.remove(binding.key()) {
                                return Err(compile::Error::new(
                                    ast,
                                    CompileErrorKind::LitObjectNotField {
                                        field: binding.key().into(),
                                        item: ctx.q.pool.item(meta.item_meta.item).to_owned(),
                                    },
                                ));
                            }
                        }

                        if !is_open && !fields.is_empty() {
                            let mut fields = fields
                                .into_iter()
                                .map(Box::<str>::from)
                                .collect::<Box<[_]>>();
                            fields.sort();

                            return Err(compile::Error::new(
                                ast,
                                CompileErrorKind::PatternMissingFields {
                                    item: ctx.q.pool.item(meta.item_meta.item).to_owned(),
                                    fields,
                                },
                            ));
                        }

                        kind
                    }
                    None => hir::PatItemsKind::Anonymous { count, is_open },
                };

                hir::PatKind::Object(alloc!(hir::PatItems {
                    kind,
                    items,
                    is_open,
                    count,
                    bindings,
                }))
            }
            ast::Pat::Binding(..) => hir::PatKind::Binding,
        },
    })
}

fn object_key<'hir>(
    ctx: &mut Ctx<'hir, '_>,
    ast: &ast::ObjectKey,
) -> compile::Result<(Span, &'hir str)> {
    alloc_with!(ctx, ast);

    Ok(match ast {
        ast::ObjectKey::LitStr(lit) => {
            let string = lit.resolve(resolve_context!(ctx.q))?;
            (lit.span(), alloc_str!(string.as_ref()))
        }
        ast::ObjectKey::Path(ast) => {
            let Some(ident) = ast.try_as_ident() else {
                return Err(compile::Error::expected(ast, "object key"));
            };

            let string = ident.resolve(resolve_context!(ctx.q))?;
            (ident.span(), alloc_str!(string))
        }
    })
}

/// Lower an object identifier to an optional path.
fn object_ident<'hir>(
    ctx: &mut Ctx<'hir, '_>,
    ast: &ast::ObjectIdent,
) -> compile::Result<Option<&'hir hir::Path<'hir>>> {
    alloc_with!(ctx, ast);

    Ok(match ast {
        ast::ObjectIdent::Anonymous(_) => None,
        ast::ObjectIdent::Named(ast) => Some(alloc!(path(ctx, ast)?)),
    })
}

/// Lower the given path.
pub(crate) fn path<'hir>(
    ctx: &mut Ctx<'hir, '_>,
    ast: &ast::Path,
) -> compile::Result<hir::Path<'hir>> {
    alloc_with!(ctx, ast);

    Ok(hir::Path {
        id: ast.id,
        span: ast.span(),
        global: ast.global.as_ref().map(Spanned::span),
        trailing: ast.trailing.as_ref().map(Spanned::span),
        first: alloc!(path_segment(ctx, &ast.first)?),
        rest: iter!(&ast.rest, |(_, s)| path_segment(ctx, s)?),
    })
}

fn path_segment<'hir>(
    ctx: &mut Ctx<'hir, '_>,
    ast: &ast::PathSegment,
) -> compile::Result<hir::PathSegment<'hir>> {
    alloc_with!(ctx, ast);

    let kind = match ast {
        ast::PathSegment::SelfType(..) => hir::PathSegmentKind::SelfType,
        ast::PathSegment::SelfValue(..) => hir::PathSegmentKind::SelfValue,
        ast::PathSegment::Ident(ast) => hir::PathSegmentKind::Ident(alloc!(*ast)),
        ast::PathSegment::Crate(..) => hir::PathSegmentKind::Crate,
        ast::PathSegment::Super(..) => hir::PathSegmentKind::Super,
        ast::PathSegment::Generics(ast) => {
            hir::PathSegmentKind::Generics(iter!(ast, |(e, _)| expr(ctx, &e.expr)?))
        }
    };

    Ok(hir::PathSegment {
        span: ast.span(),
        kind,
    })
}

fn label(_: &mut Ctx<'_, '_>, ast: &ast::Label) -> compile::Result<ast::Label> {
    Ok(ast::Label {
        span: ast.span,
        source: ast.source,
    })
}

fn condition<'hir>(
    ctx: &mut Ctx<'hir, '_>,
    ast: &ast::Condition,
) -> compile::Result<hir::Condition<'hir>> {
    alloc_with!(ctx, ast);

    Ok(match ast {
        ast::Condition::Expr(ast) => hir::Condition::Expr(alloc!(expr(ctx, ast)?)),
        ast::Condition::ExprLet(ast) => hir::Condition::ExprLet(alloc!(hir::ExprLet {
            pat: alloc!(pat(ctx, &ast.pat)?),
            expr: alloc!(expr(ctx, &ast.expr)?),
        })),
    })
}

/// Test if the given pattern is open or not.
fn pat_items_count(items: &[hir::Pat<'_>]) -> compile::Result<(bool, usize)> {
    let mut it = items.iter();

    let (is_open, mut count) = match it.next_back() {
        Some(pat) => matches!(pat.kind, hir::PatKind::Rest)
            .then(|| (true, 0))
            .unwrap_or((false, 1)),
        None => return Ok((false, 0)),
    };

    for pat in it {
        if let hir::PatKind::Rest = pat.kind {
            return Err(compile::Error::new(
                pat.span(),
                HirErrorKind::UnsupportedPatternRest,
            ));
        }

        count += 1;
    }

    Ok((is_open, count))
}

fn struct_match_for<'a>(
    ctx: &Ctx<'_, '_>,
    meta: &'a meta::Meta,
) -> Option<(&'a meta::FieldsNamed, hir::PatItemsKind)> {
    Some(match &meta.kind {
        meta::Kind::Struct {
            fields: meta::Fields::Named(st),
            ..
        } => (st, hir::PatItemsKind::Type { hash: meta.hash }),
        meta::Kind::Variant {
            enum_hash,
            index,
            fields: meta::Fields::Named(st),
            ..
        } => {
            let kind = if let Some(type_check) = ctx.q.context.type_check_for(meta.hash) {
                hir::PatItemsKind::BuiltInVariant { type_check }
            } else {
                hir::PatItemsKind::Variant {
                    variant_hash: meta.hash,
                    enum_hash: *enum_hash,
                    index: *index,
                }
            };

            (st, kind)
        }
        _ => {
            return None;
        }
    })
}

fn tuple_match_for(ctx: &Ctx<'_, '_>, meta: &meta::Meta) -> Option<(usize, hir::PatItemsKind)> {
    Some(match &meta.kind {
        meta::Kind::Struct {
            fields: meta::Fields::Empty,
            ..
        } => (0, hir::PatItemsKind::Type { hash: meta.hash }),
        meta::Kind::Struct {
            fields: meta::Fields::Unnamed(args),
            ..
        } => (*args, hir::PatItemsKind::Type { hash: meta.hash }),
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

            let kind = if let Some(type_check) = ctx.q.context.type_check_for(meta.hash) {
                hir::PatItemsKind::BuiltInVariant { type_check }
            } else {
                hir::PatItemsKind::Variant {
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

fn generics_parameters<'hir>(
    ctx: &mut Ctx<'hir, '_>,
    named: &Named<'hir>,
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
        if let &Some((_, expr)) = value {
            *o = Some(generics_parameter(ctx, expr)?);
        }
    }

    Ok(parameters)
}

fn generics_parameter<'hir>(
    ctx: &mut Ctx<'hir, '_>,
    generics: &[hir::Expr<'hir>],
) -> compile::Result<Hash> {
    let mut builder = ParametersBuilder::new();

    for expr in generics {
        let hir::ExprKind::Path(path) = expr.kind else {
            return Err(compile::Error::new(
                expr.span,
                CompileErrorKind::UnsupportedGenerics,
            ));
        };

        let named = ctx.q.convert_path(path)?;
        let parameters = generics_parameters(ctx, &named)?;
        let meta = ctx.lookup_meta(expr.span(), named.item, parameters)?;

        let (meta::Kind::Type { .. } | meta::Kind::Struct { .. } | meta::Kind::Enum { .. }) = meta.kind else {
            return Err(compile::Error::new(
                expr.span,
                CompileErrorKind::UnsupportedGenerics,
            ));
        };

        builder.add(meta.hash);
    }

    Ok(builder.finish())
}
