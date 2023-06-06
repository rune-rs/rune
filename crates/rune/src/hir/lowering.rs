use core::cell::Cell;
use core::ops::Neg;

use num::ToPrimitive;

use crate::ast::{self, Span, Spanned};
use crate::compile::{self, CompileErrorKind, HirErrorKind, ParseErrorKind};
use crate::hir;
use crate::parse::Resolve;
use crate::query::{self, Query};

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
            ($iter:expr, |$pat:pat_param| $closure:expr) => {{
                let mut it = IntoIterator::into_iter($iter);

                let mut writer = match $ctx.arena.alloc_iter(ExactSizeIterator::len(&it)) {
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
        
                while let Some($pat) = it.next() {
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

pub struct Ctx<'hir, 'a> {
    /// Arena used for allocations.
    arena: &'hir hir::arena::Arena,
    q: Query<'a>,
    in_template: Cell<bool>,
}

impl<'hir, 'a> Ctx<'hir, 'a> {
    /// Construct a new context.
    pub(crate) fn new(arena: &'hir hir::arena::Arena, query: Query<'a>) -> Self {
        Self {
            arena,
            q: query,
            in_template: Cell::new(false),
        }
    }
}

/// Lower a function item.
pub(crate) fn item_fn<'hir>(
    ctx: &Ctx<'hir, '_>,
    ast: &ast::ItemFn,
) -> compile::Result<hir::ItemFn<'hir>> {
    alloc_with!(ctx, ast);

    Ok(hir::ItemFn {
        id: ast.id,
        span: ast.span(),
        visibility: alloc!(match &ast.visibility {
            ast::Visibility::Inherited => hir::Visibility::Inherited,
            ast::Visibility::Public(_) => hir::Visibility::Public,
            ast::Visibility::Crate(_) => hir::Visibility::Crate,
            ast::Visibility::Super(_) => hir::Visibility::Super,
            ast::Visibility::SelfValue(_) => hir::Visibility::SelfValue,
            ast::Visibility::In(ast) =>
                hir::Visibility::In(alloc!(path(ctx, &ast.restriction.path)?)),
        }),
        name: alloc!(ast.name),
        args: iter!(&ast.args, |(ast, _)| fn_arg(ctx, ast)?),
        body: alloc!(block(ctx, &ast.body)?),
    })
}

/// Lower a closure expression.
pub(crate) fn expr_closure<'hir>(
    ctx: &Ctx<'hir, '_>,
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
    ctx: &Ctx<'hir, '_>,
    ast: &ast::Block,
) -> compile::Result<hir::Block<'hir>> {
    alloc_with!(ctx, ast);

    Ok(hir::Block {
        id: ast.id,
        span: ast.span(),
        statements: iter!(&ast.statements, |ast| stmt(ctx, ast)?),
    })
}

/// Lower an expression.
pub(crate) fn expr<'hir>(ctx: &Ctx<'hir, '_>, ast: &ast::Expr) -> compile::Result<hir::Expr<'hir>> {
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
        ast::Expr::While(ast) => hir::ExprKind::Loop(alloc!(hir::ExprLoop {
            label: option!(&ast.label, |(ast, _)| label(ctx, ast)?),
            condition: Some(alloc!(condition(ctx, &ast.condition)?)),
            body: alloc!(block(ctx, &ast.body)?),
        })),
        ast::Expr::Loop(ast) => hir::ExprKind::Loop(alloc!(hir::ExprLoop {
            label: option!(&ast.label, |(ast, _)| label(ctx, ast)?),
            condition: None,
            body: alloc!(block(ctx, &ast.body)?),
        })),
        ast::Expr::For(ast) => hir::ExprKind::For(alloc!(hir::ExprFor {
            label: option!(&ast.label, |(ast, _)| label(ctx, ast)?),
            binding: alloc!(pat(ctx, &ast.binding)?),
            iter: alloc!(expr(ctx, &ast.iter)?),
            body: alloc!(block(ctx, &ast.body)?),
        })),
        ast::Expr::Let(ast) => hir::ExprKind::Let(alloc!(hir::ExprLet {
            pat: alloc!(pat(ctx, &ast.pat)?),
            expr: alloc!(expr(ctx, &ast.expr)?),
        })),
        ast::Expr::If(ast) => hir::ExprKind::If(alloc!(hir::ExprIf {
            condition: alloc!(condition(ctx, &ast.condition)?),
            block: alloc!(block(ctx, &ast.block)?),
            expr_else_ifs: iter!(&ast.expr_else_ifs, |ast| hir::ExprElseIf {
                span: ast.span(),
                condition: alloc!(condition(ctx, &ast.condition)?),
                block: alloc!(block(ctx, &ast.block)?),
            }),
            expr_else: option!(&ast.expr_else, |ast| hir::ExprElse {
                span: ast.span(),
                block: alloc!(block(ctx, &ast.block)?)
            }),
        })),
        ast::Expr::Match(ast) => hir::ExprKind::Match(alloc!(hir::ExprMatch {
            expr: alloc!(expr(ctx, &ast.expr)?),
            branches: iter!(&ast.branches, |(ast, _)| hir::ExprMatchBranch {
                span: ast.span(),
                pat: alloc!(pat(ctx, &ast.pat)?),
                condition: option!(&ast.condition, |(_, ast)| expr(ctx, ast)?),
                body: alloc!(expr(ctx, &ast.body)?),
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
        ast::Expr::Block(ast) => hir::ExprKind::Block(alloc!(expr_block(ctx, ast)?)),
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
                        hir::ExprSelectBranch::Pat(alloc!(hir::ExprSelectPatBranch {
                            pat: alloc!(pat(ctx, &ast.pat)?),
                            expr: alloc!(expr(ctx, &ast.expr)?),
                            body: alloc!(expr(ctx, &ast.body)?),
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
        ast::Expr::Object(ast) => hir::ExprKind::Object(alloc!(hir::ExprObject {
            path: object_ident(ctx, &ast.ident)?,
            assignments: iter!(&ast.assignments, |(ast, _)| hir::FieldAssign {
                key: object_key(ctx, &ast.key)?,
                assign: option!(&ast.assign, |(_, ast)| expr(ctx, ast)?),
            })
        })),
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
        ast::Expr::MacroCall(ast) => match ctx.q.builtin_macro_for(ast)? {
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

pub(crate) fn lit<'hir>(
    span: &dyn Spanned,
    ctx: &Ctx<'hir, '_>,
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
    ctx: &Ctx<'hir, '_>,
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
    ctx: &Ctx<'hir, '_>,
    ast: &ast::ExprBlock,
) -> compile::Result<hir::ExprBlock<'hir>> {
    alloc_with!(ctx, ast);

    Ok(hir::ExprBlock {
        kind: match (&ast.async_token, &ast.const_token) {
            (Some(..), None) => hir::ExprBlockKind::Async,
            (None, Some(..)) => hir::ExprBlockKind::Const,
            _ => hir::ExprBlockKind::Default,
        },
        block_move: ast.move_token.is_some(),
        block: alloc!(block(ctx, &ast.block)?),
    })
}

/// Lower a function argument.
fn fn_arg<'hir>(ctx: &Ctx<'hir, '_>, ast: &ast::FnArg) -> compile::Result<hir::FnArg<'hir>> {
    alloc_with!(ctx, ast);

    Ok(match ast {
        ast::FnArg::SelfValue(ast) => hir::FnArg::SelfValue(ast.span()),
        ast::FnArg::Pat(ast) => hir::FnArg::Pat(alloc!(pat(ctx, ast)?)),
    })
}

/// Lower an assignment.
fn local<'hir>(ctx: &Ctx<'hir, '_>, ast: &ast::Local) -> compile::Result<hir::Local<'hir>> {
    alloc_with!(ctx, ast);

    Ok(hir::Local {
        span: ast.span(),
        pat: alloc!(pat(ctx, &ast.pat)?),
        expr: alloc!(expr(ctx, &ast.expr)?),
    })
}

/// Lower a statement
fn stmt<'hir>(ctx: &Ctx<'hir, '_>, ast: &ast::Stmt) -> compile::Result<hir::Stmt<'hir>> {
    alloc_with!(ctx, ast);

    Ok(match ast {
        ast::Stmt::Local(ast) => hir::Stmt::Local(alloc!(local(ctx, ast)?)),
        ast::Stmt::Expr(ast) => hir::Stmt::Expr(alloc!(expr(ctx, ast)?)),
        ast::Stmt::Semi(ast) => hir::Stmt::Semi(alloc!(expr(ctx, &ast.expr)?)),
        ast::Stmt::Item(..) => hir::Stmt::Item(ast.span()),
    })
}

fn pat<'hir>(ctx: &Ctx<'hir, '_>, ast: &ast::Pat) -> compile::Result<hir::Pat<'hir>> {
    alloc_with!(ctx, ast);

    Ok(hir::Pat {
        span: ast.span(),
        kind: match ast {
            ast::Pat::PatIgnore(..) => hir::PatKind::PatIgnore,
            ast::Pat::PatRest(..) => hir::PatKind::PatRest,
            ast::Pat::PatPath(ast) => hir::PatKind::PatPath(alloc!(path(ctx, &ast.path)?)),
            ast::Pat::PatLit(ast) => hir::PatKind::PatLit(alloc!(expr(ctx, &ast.expr)?)),
            ast::Pat::PatVec(ast) => {
                let items = iter!(&ast.items, |(ast, _)| pat(ctx, ast)?);
                let (is_open, count) = pat_items_count(items)?;

                hir::PatKind::PatVec(alloc!(hir::PatItems {
                    path: None,
                    items,
                    is_open,
                    count,
                }))
            }
            ast::Pat::PatTuple(ast) => {
                let items = iter!(&ast.items, |(ast, _)| pat(ctx, ast)?);
                let (is_open, count) = pat_items_count(items)?;

                hir::PatKind::PatTuple(alloc!(hir::PatItems {
                    path: option!(&ast.path, |ast| path(ctx, ast)?),
                    items,
                    is_open,
                    count,
                }))
            }
            ast::Pat::PatObject(ast) => {
                let items = iter!(&ast.items, |(ast, _)| pat(ctx, ast)?);
                let (is_open, count) = pat_items_count(items)?;

                hir::PatKind::PatObject(alloc!(hir::PatItems {
                    path: object_ident(ctx, &ast.ident)?,
                    items,
                    is_open,
                    count,
                }))
            }
            ast::Pat::PatBinding(ast) => hir::PatKind::PatBinding(alloc!(hir::PatBinding {
                key: object_key(ctx, &ast.key)?,
                pat: alloc!(pat(ctx, &ast.pat)?),
            })),
        },
    })
}

fn object_key<'hir>(
    ctx: &Ctx<'hir, '_>,
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
    ctx: &Ctx<'hir, '_>,
    ast: &ast::ObjectIdent,
) -> compile::Result<Option<&'hir hir::Path<'hir>>> {
    alloc_with!(ctx, ast);

    Ok(match ast {
        ast::ObjectIdent::Anonymous(_) => None,
        ast::ObjectIdent::Named(ast) => Some(alloc!(path(ctx, ast)?)),
    })
}

/// Lower the given path.
pub(crate) fn path<'hir>(ctx: &Ctx<'hir, '_>, ast: &ast::Path) -> compile::Result<hir::Path<'hir>> {
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
    ctx: &Ctx<'hir, '_>,
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

fn label(_: &Ctx<'_, '_>, ast: &ast::Label) -> compile::Result<ast::Label> {
    Ok(ast::Label {
        span: ast.span,
        source: ast.source,
    })
}

fn condition<'hir>(
    ctx: &Ctx<'hir, '_>,
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
        Some(pat) => matches!(pat.kind, hir::PatKind::PatRest)
            .then(|| (true, 0))
            .unwrap_or((false, 1)),
        None => return Ok((false, 0)),
    };

    for pat in it {
        if let hir::PatKind::PatRest = pat.kind {
            return Err(compile::Error::new(
                pat.span(),
                HirErrorKind::UnsupportedPatternRest,
            ));
        }

        count += 1;
    }

    Ok((is_open, count))
}
