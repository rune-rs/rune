use crate::ast::{self, Spanned};
use crate::hir;
use crate::hir::{HirError, HirErrorKind};
use crate::query::{self, Query};

/// Allocate a single object in the arena.
macro_rules! alloc {
    ($ctx:expr, $span:expr; $value:expr) => {
        $ctx.arena.alloc($value).map_err(|e| {
            HirError::new(
                $span,
                HirErrorKind::ArenaAllocError {
                    requested: e.requested,
                },
            )
        })?
    };
}

/// Unpacks an optional value and allocates it in the arena.
macro_rules! option {
    ($ctx:expr, $span:expr; $value:expr, |$pat:pat_param| $closure:expr) => {
        match $value {
            Some($pat) => {
                Some(&*alloc!($ctx, $span; $closure))
            }
            None => {
                None
            }
        }
    };
}

/// Unpacks an iterator value and allocates it in the arena as a slice.
macro_rules! iter {
    ($ctx:expr, $span:expr; $iter:expr, |$pat:pat_param| $closure:expr) => {{
        let mut it = IntoIterator::into_iter($iter);

        let mut writer = match $ctx.arena.alloc_iter(ExactSizeIterator::len(&it)) {
            Ok(writer) => writer,
            Err(e) => {
                return Err(HirError::new(
                    $span,
                    HirErrorKind::ArenaAllocError {
                        requested: e.requested,
                    },
                ));
            }
        };

        while let Some($pat) = it.next() {
            if let Err(e) = writer.write($closure) {
                return Err(HirError::new(
                    $span,
                    HirErrorKind::ArenaWriteSliceOutOfBounds { index: e.index },
                ));
            }
        }

        writer.finish()
    }};
}

pub struct Ctx<'hir, 'a> {
    /// Arena used for allocations.
    arena: &'hir hir::arena::Arena,
    q: Query<'a>,
}

impl<'hir, 'a> Ctx<'hir, 'a> {
    /// Construct a new contctx.
    pub(crate) fn new(arena: &'hir hir::arena::Arena, query: Query<'a>) -> Self {
        Self { arena, q: query }
    }
}

/// Lower a function item.
pub fn item_fn<'hir>(
    ctx: &Ctx<'hir, '_>,
    ast: &ast::ItemFn,
) -> Result<hir::ItemFn<'hir>, HirError> {
    Ok(hir::ItemFn {
        id: ast.id,
        span: ast.span(),
        visibility: alloc!(ctx, ast; visibility(ctx, &ast.visibility)?),
        name: alloc!(ctx, ast; ast.name),
        args: iter!(ctx, ast; &ast.args, |(ast, _)| fn_arg(ctx, ast)?),
        body: alloc!(ctx, ast; block(ctx, &ast.body)?),
    })
}

/// Lower a closure expression.
pub fn expr_closure<'hir>(
    ctx: &Ctx<'hir, '_>,
    ast: &ast::ExprClosure,
) -> Result<hir::ExprClosure<'hir>, HirError> {
    Ok(hir::ExprClosure {
        id: ast.id,
        args: match &ast.args {
            ast::ExprClosureArgs::Empty { .. } => &[],
            ast::ExprClosureArgs::List { args, .. } => {
                iter!(ctx, ast; args, |(ast, _)| fn_arg(ctx, ast)?)
            }
        },
        body: alloc!(ctx, ast; expr(ctx, &ast.body)?),
    })
}

/// Lower the specified block.
pub fn block<'hir>(ctx: &Ctx<'hir, '_>, ast: &ast::Block) -> Result<hir::Block<'hir>, HirError> {
    Ok(hir::Block {
        id: ast.id,
        span: ast.span(),
        statements: iter!(ctx, ast; &ast.statements, |ast| stmt(ctx, ast)?),
    })
}

/// Lower an expression.
pub fn expr<'hir>(ctx: &Ctx<'hir, '_>, ast: &ast::Expr) -> Result<hir::Expr<'hir>, HirError> {
    let kind = match ast {
        ast::Expr::Path(ast) => hir::ExprKind::Path(alloc!(ctx, ast; path(ctx, ast)?)),
        ast::Expr::Assign(ast) => hir::ExprKind::Assign(alloc!(ctx, ast; hir::ExprAssign {
            lhs: alloc!(ctx, ast; expr(ctx, &ast.lhs)?),
            rhs: alloc!(ctx, ast; expr(ctx, &ast.rhs)?),
        })),
        // TODO: lower all of these loop constructs to the same loop-like
        // representation. We only do different ones here right now since it's
        // easier when refactoring.
        ast::Expr::While(ast) => hir::ExprKind::Loop(alloc!(ctx, ast; hir::ExprLoop {
            label: option!(ctx, ast; &ast.label, |(ast, _)| label(ctx, ast)?),
            condition: Some(alloc!(ctx, ast; condition(ctx, &ast.condition)?)),
            body: alloc!(ctx, ast; block(ctx, &ast.body)?),
        })),
        ast::Expr::Loop(ast) => hir::ExprKind::Loop(alloc!(ctx, ast; hir::ExprLoop {
            label: option!(ctx, ast; &ast.label, |(ast, _)| label(ctx, ast)?),
            condition: None,
            body: alloc!(ctx, ast; block(ctx, &ast.body)?),
        })),
        ast::Expr::For(ast) => hir::ExprKind::For(alloc!(ctx, ast; hir::ExprFor {
            label: option!(ctx, ast; &ast.label, |(ast, _)| label(ctx, ast)?),
            binding: alloc!(ctx, ast; pat(ctx, &ast.binding)?),
            iter: alloc!(ctx, ast; expr(ctx, &ast.iter)?),
            body: alloc!(ctx, ast; block(ctx, &ast.body)?),
        })),
        ast::Expr::Let(ast) => hir::ExprKind::Let(alloc!(ctx, ast; hir::ExprLet {
            pat: alloc!(ctx, ast; pat(ctx, &ast.pat)?),
            expr: alloc!(ctx, ast; expr(ctx, &ast.expr)?),
        })),
        ast::Expr::If(ast) => hir::ExprKind::If(alloc!(ctx, ast; hir::ExprIf {
            condition: alloc!(ctx, ast; condition(ctx, &ast.condition)?),
            block: alloc!(ctx, ast; block(ctx, &ast.block)?),
            expr_else_ifs: iter!(ctx, ast; &ast.expr_else_ifs, |ast| hir::ExprElseIf {
                span: ast.span(),
                condition: alloc!(ctx, ast; condition(ctx, &ast.condition)?),
                block: alloc!(ctx, ast; block(ctx, &ast.block)?),
            }),
            expr_else: option!(ctx, ast; &ast.expr_else, |ast| hir::ExprElse {
                span: ast.span(),
                block: alloc!(ctx, ast; block(ctx, &ast.block)?)
            }),
        })),
        ast::Expr::Match(ast) => hir::ExprKind::Match(alloc!(ctx, ast; hir::ExprMatch {
            expr: alloc!(ctx, ast; expr(ctx, &ast.expr)?),
            branches: iter!(ctx, ast; &ast.branches, |(ast, _)| hir::ExprMatchBranch {
                span: ast.span(),
                pat: alloc!(ctx, ast; pat(ctx, &ast.pat)?),
                condition: option!(ctx, ast; &ast.condition, |(_, ast)| expr(ctx, ast)?),
                body: alloc!(ctx, ast; expr(ctx, &ast.body)?),
            }),
        })),
        ast::Expr::Call(ast) => hir::ExprKind::Call(alloc!(ctx, ast; hir::ExprCall {
            id: ast.id,
            expr: alloc!(ctx, ast; expr(ctx, &ast.expr)?),
            args: iter!(ctx, ast; &ast.args, |(ast, _)| expr(ctx, ast)?),
        })),
        ast::Expr::FieldAccess(ast) => {
            hir::ExprKind::FieldAccess(alloc!(ctx, ast; hir::ExprFieldAccess {
                expr: alloc!(ctx, ast; expr(ctx, &ast.expr)?),
                expr_field: alloc!(ctx, ast; match &ast.expr_field {
                    ast::ExprField::Path(ast) => hir::ExprField::Path(alloc!(ctx, ast; path(ctx, ast)?)),
                    ast::ExprField::LitNumber(ast) => hir::ExprField::LitNumber(alloc!(ctx, ast; *ast)),
                }),
            }))
        }
        ast::Expr::Empty(ast) => hir::ExprKind::Group(alloc!(ctx, ast; expr(ctx, &ast.expr)?)),
        ast::Expr::Binary(ast) => hir::ExprKind::Binary(alloc!(ctx, ast; hir::ExprBinary {
            lhs: alloc!(ctx, ast; expr(ctx, &ast.lhs)?),
            op: ast.op,
            rhs: alloc!(ctx, ast; expr(ctx, &ast.rhs)?),
        })),
        ast::Expr::Unary(ast) => hir::ExprKind::Unary(alloc!(ctx, ast; hir::ExprUnary {
            op: ast.op,
            expr: alloc!(ctx, ast; expr(ctx, &ast.expr)?),
        })),
        ast::Expr::Index(ast) => hir::ExprKind::Index(alloc!(ctx, ast; hir::ExprIndex {
            target: alloc!(ctx, ast; expr(ctx, &ast.target)?),
            index: alloc!(ctx, ast; expr(ctx, &ast.index)?),
        })),
        ast::Expr::Block(ast) => hir::ExprKind::Block(alloc!(ctx, ast; expr_block(ctx, ast)?)),
        ast::Expr::Break(ast) => {
            hir::ExprKind::Break(option!(ctx, ast; ast.expr.as_deref(), |ast| match ast {
                ast::ExprBreakValue::Expr(ast) => hir::ExprBreakValue::Expr(alloc!(ctx, ast; expr(ctx, ast)?)),
                ast::ExprBreakValue::Label(ast) => hir::ExprBreakValue::Label(alloc!(ctx, ast; label(ctx, ast)?)),
            }))
        }
        ast::Expr::Continue(ast) => {
            hir::ExprKind::Continue(option!(ctx, ast; &ast.label, |ast| label(ctx, ast)?))
        }
        ast::Expr::Yield(ast) => {
            hir::ExprKind::Yield(option!(ctx, ast; &ast.expr, |ast| expr(ctx, ast)?))
        }
        ast::Expr::Return(ast) => {
            hir::ExprKind::Return(option!(ctx, ast; &ast.expr, |ast| expr(ctx, ast)?))
        }
        ast::Expr::Await(ast) => hir::ExprKind::Await(alloc!(ctx, ast; expr(ctx, &ast.expr)?)),
        ast::Expr::Try(ast) => hir::ExprKind::Try(alloc!(ctx, ast; expr(ctx, &ast.expr)?)),
        ast::Expr::Select(ast) => hir::ExprKind::Select(alloc!(ctx, ast; hir::ExprSelect {
            branches: iter!(ctx, ast; &ast.branches, |(ast, _)| {
                match ast {
                    ast::ExprSelectBranch::Pat(ast) => hir::ExprSelectBranch::Pat(alloc!(ctx, ast; hir::ExprSelectPatBranch {
                        pat: alloc!(ctx, &ast.pat; pat(ctx, &ast.pat)?),
                        expr: alloc!(ctx, &ast.expr; expr(ctx, &ast.expr)?),
                        body: alloc!(ctx, &ast.body; expr(ctx, &ast.body)?),
                    })),
                    ast::ExprSelectBranch::Default(ast) => hir::ExprSelectBranch::Default(alloc!(ctx, &ast.body; expr(ctx, &ast.body)?)),
                }
            })
        })),
        ast::Expr::Closure(ast) => {
            hir::ExprKind::Closure(alloc!(ctx, ast; expr_closure(ctx, ast)?))
        }
        ast::Expr::Lit(ast) => hir::ExprKind::Lit(alloc!(ctx, &ast.lit; ast.lit)),
        ast::Expr::Object(ast) => hir::ExprKind::Object(alloc!(ctx, ast; hir::ExprObject {
            path: object_ident(ctx, &ast.ident)?,
            assignments: iter!(ctx, ast; &ast.assignments, |(ast, _)| hir::FieldAssign {
                span: ast.span(),
                key: alloc!(ctx, ast; object_key(ctx, &ast.key)?),
                assign: option!(ctx, ast; &ast.assign, |(_, ast)| expr(ctx, ast)?),
            })
        })),
        ast::Expr::Tuple(ast) => hir::ExprKind::Tuple(alloc!(ctx, ast; hir::ExprSeq {
            items: iter!(ctx, ast; &ast.items, |(ast, _)| expr(ctx, ast)?),
        })),
        ast::Expr::Vec(ast) => hir::ExprKind::Vec(alloc!(ctx, ast; hir::ExprSeq {
            items: iter!(ctx, ast; &ast.items, |(ast, _)| expr(ctx, ast)?),
        })),
        ast::Expr::Range(ast) => hir::ExprKind::Range(alloc!(ctx, ast; hir::ExprRange {
            from: option!(ctx, ast; &ast.from, |ast| expr(ctx, ast)?),
            limits: match ast.limits {
                ast::ExprRangeLimits::HalfOpen(_) => hir::ExprRangeLimits::HalfOpen,
                ast::ExprRangeLimits::Closed(_) => hir::ExprRangeLimits::Closed,
            },
            to: option!(ctx, ast; &ast.to, |ast| expr(ctx, ast)?),
        })),
        ast::Expr::Group(ast) => hir::ExprKind::Group(alloc!(ctx, ast; expr(ctx, &ast.expr)?)),
        ast::Expr::MacroCall(ast) => {
            hir::ExprKind::MacroCall(alloc!(ctx, ast; match ctx.q.builtin_macro_for(ast)? {
                query::BuiltInMacro::Template(ast) => hir::MacroCall::Template(alloc!(ctx, ast; hir::BuiltInTemplate {
                    span: ast.span,
                    from_literal: ast.from_literal,
                    exprs: iter!(ctx, ast; &ast.exprs, |ast| expr(ctx, ast)?),
                })),
                query::BuiltInMacro::Format(ast) => hir::MacroCall::Format(alloc!(ctx, ast; hir::BuiltInFormat {
                    span: ast.span,
                    fill: ast.fill,
                    align: ast.align,
                    width: ast.width,
                    precision: ast.precision,
                    flags: ast.flags,
                    format_type: ast.format_type,
                    value: alloc!(ctx, &ast.value; expr(ctx, &ast.value)?),
                })),
                query::BuiltInMacro::File(ast) => hir::MacroCall::File(alloc!(ctx, ast; hir::BuiltInFile {
                    span: ast.span,
                    value: ast.value,
                })),
                query::BuiltInMacro::Line(ast) => hir::MacroCall::Line(alloc!(ctx, ast; hir::BuiltInLine {
                    span: ast.span,
                    value: ast.value,
                })),
            }
            ))
        }
    };

    Ok(hir::Expr {
        span: ast.span(),
        kind,
    })
}

/// Lower a block expression.
pub fn expr_block<'hir>(
    ctx: &Ctx<'hir, '_>,
    ast: &ast::ExprBlock,
) -> Result<hir::ExprBlock<'hir>, HirError> {
    Ok(hir::ExprBlock {
        kind: match (&ast.async_token, &ast.const_token) {
            (Some(..), None) => hir::ExprBlockKind::Async,
            (None, Some(..)) => hir::ExprBlockKind::Const,
            _ => hir::ExprBlockKind::Default,
        },
        block_move: ast.move_token.is_some(),
        block: alloc!(ctx, ast; block(ctx, &ast.block)?),
    })
}

/// Visibility covnersion.
fn visibility<'hir>(
    ctx: &Ctx<'hir, '_>,
    ast: &ast::Visibility,
) -> Result<hir::Visibility<'hir>, HirError> {
    Ok(match ast {
        ast::Visibility::Inherited => hir::Visibility::Inherited,
        ast::Visibility::Public(_) => hir::Visibility::Public,
        ast::Visibility::Crate(_) => hir::Visibility::Crate,
        ast::Visibility::Super(_) => hir::Visibility::Super,
        ast::Visibility::SelfValue(_) => hir::Visibility::SelfValue,
        ast::Visibility::In(ast) => {
            hir::Visibility::In(alloc!(ctx, ast; path(ctx, &ast.restriction.path)?))
        }
    })
}

/// Lower a function argument.
fn fn_arg<'hir>(ctx: &Ctx<'hir, '_>, ast: &ast::FnArg) -> Result<hir::FnArg<'hir>, HirError> {
    Ok(match ast {
        ast::FnArg::SelfValue(ast) => hir::FnArg::SelfValue(ast.span()),
        ast::FnArg::Pat(ast) => hir::FnArg::Pat(alloc!(ctx, ast; pat(ctx, ast)?)),
    })
}

/// Lower an assignment.
fn local<'hir>(ctx: &Ctx<'hir, '_>, ast: &ast::Local) -> Result<hir::Local<'hir>, HirError> {
    Ok(hir::Local {
        span: ast.span(),
        pat: alloc!(ctx, ast; pat(ctx, &ast.pat)?),
        expr: alloc!(ctx, ast; expr(ctx, &ast.expr)?),
    })
}

/// Lower a statement
fn stmt<'hir>(ctx: &Ctx<'hir, '_>, ast: &ast::Stmt) -> Result<hir::Stmt<'hir>, HirError> {
    Ok(match ast {
        ast::Stmt::Local(ast) => hir::Stmt::Local(alloc!(ctx, ast; local(ctx, ast)?)),
        ast::Stmt::Expr(ast) => hir::Stmt::Expr(alloc!(ctx, ast; expr(ctx, ast)?)),
        ast::Stmt::Semi(ast) => hir::Stmt::Semi(alloc!(ctx, ast; expr(ctx, &ast.expr)?)),
        ast::Stmt::Item(..) => hir::Stmt::Item(ast.span()),
    })
}

fn pat<'hir>(ctx: &Ctx<'hir, '_>, ast: &ast::Pat) -> Result<hir::Pat<'hir>, HirError> {
    Ok(hir::Pat {
        span: ast.span(),
        kind: match ast {
            ast::Pat::PatIgnore(..) => hir::PatKind::PatIgnore,
            ast::Pat::PatRest(..) => hir::PatKind::PatRest,
            ast::Pat::PatPath(ast) => {
                hir::PatKind::PatPath(alloc!(ctx, ast; path(ctx, &ast.path)?))
            }
            ast::Pat::PatLit(ast) => hir::PatKind::PatLit(alloc!(ctx, ast; expr(ctx, &ast.expr)?)),
            ast::Pat::PatVec(ast) => {
                let items = iter!(ctx, ast; &ast.items, |(ast, _)| pat(ctx, ast)?);
                let (is_open, count) = pat_items_count(items)?;

                hir::PatKind::PatVec(alloc!(ctx, ast; hir::PatItems {
                    path: None,
                    items,
                    is_open,
                    count,
                }))
            }
            ast::Pat::PatTuple(ast) => {
                let items = iter!(ctx, ast; &ast.items, |(ast, _)| pat(ctx, ast)?);
                let (is_open, count) = pat_items_count(items)?;

                hir::PatKind::PatTuple(alloc!(ctx, ast; hir::PatItems {
                    path: option!(ctx, ast; &ast.path, |ast| path(ctx, ast)?),
                    items,
                    is_open,
                    count,
                }))
            }
            ast::Pat::PatObject(ast) => {
                let items = iter!(ctx, ast; &ast.items, |(ast, _)| pat(ctx, ast)?);
                let (is_open, count) = pat_items_count(items)?;

                hir::PatKind::PatObject(alloc!(ctx, ast; hir::PatItems {
                    path: object_ident(ctx, &ast.ident)?,
                    items,
                    is_open,
                    count,
                }))
            }
            ast::Pat::PatBinding(ast) => {
                hir::PatKind::PatBinding(alloc!(ctx, ast; hir::PatBinding {
                    key: alloc!(ctx, ast; object_key(ctx, &ast.key)?),
                    pat: alloc!(ctx, ast; pat(ctx, &ast.pat)?),
                }))
            }
        },
    })
}

fn object_key<'hir>(
    ctx: &Ctx<'hir, '_>,
    ast: &ast::ObjectKey,
) -> Result<hir::ObjectKey<'hir>, HirError> {
    Ok(match ast {
        ast::ObjectKey::LitStr(ast) => hir::ObjectKey::LitStr(alloc!(ctx, ast; *ast)),
        ast::ObjectKey::Path(ast) => hir::ObjectKey::Path(alloc!(ctx, ast; path(ctx, ast)?)),
    })
}

/// Lower an object identifier to an optional path.
fn object_ident<'hir>(
    ctx: &Ctx<'hir, '_>,
    ast: &ast::ObjectIdent,
) -> Result<Option<&'hir hir::Path<'hir>>, HirError> {
    Ok(match ast {
        ast::ObjectIdent::Anonymous(_) => None,
        ast::ObjectIdent::Named(ast) => Some(alloc!(ctx, ast; path(ctx, ast)?)),
    })
}

/// Lower the given path.
pub fn path<'hir>(ctx: &Ctx<'hir, '_>, ast: &ast::Path) -> Result<hir::Path<'hir>, HirError> {
    Ok(hir::Path {
        id: ast.id,
        span: ast.span(),
        global: ast.global.as_ref().map(Spanned::span),
        trailing: ast.trailing.as_ref().map(Spanned::span),
        first: alloc!(ctx, ast; path_segment(ctx, &ast.first)?),
        rest: iter!(ctx, ast; &ast.rest, |(_, s)| path_segment(ctx, s)?),
    })
}

fn path_segment<'hir>(
    ctx: &Ctx<'hir, '_>,
    ast: &ast::PathSegment,
) -> Result<hir::PathSegment<'hir>, HirError> {
    let kind = match ast {
        ast::PathSegment::SelfType(..) => hir::PathSegmentKind::SelfType,
        ast::PathSegment::SelfValue(..) => hir::PathSegmentKind::SelfValue,
        ast::PathSegment::Ident(ast) => hir::PathSegmentKind::Ident(alloc!(ctx, ast; *ast)),
        ast::PathSegment::Crate(..) => hir::PathSegmentKind::Crate,
        ast::PathSegment::Super(..) => hir::PathSegmentKind::Super,
        ast::PathSegment::Generics(ast) => {
            hir::PathSegmentKind::Generics(iter!(ctx, ast; ast, |(e, _)| expr(ctx, &e.expr)?))
        }
    };

    Ok(hir::PathSegment {
        span: ast.span(),
        kind,
    })
}

fn label<'hir>(_: &Ctx<'hir, '_>, ast: &ast::Label) -> Result<ast::Label, HirError> {
    Ok(ast::Label {
        span: ast.span,
        source: ast.source,
    })
}

fn condition<'hir>(
    ctx: &Ctx<'hir, '_>,
    ast: &ast::Condition,
) -> Result<hir::Condition<'hir>, HirError> {
    Ok(match ast {
        ast::Condition::Expr(ast) => hir::Condition::Expr(alloc!(ctx, ast; expr(ctx, ast)?)),
        ast::Condition::ExprLet(ast) => hir::Condition::ExprLet(alloc!(ctx, ast; hir::ExprLet {
            pat: alloc!(ctx, ast; pat(ctx, &ast.pat)?),
            expr: alloc!(ctx, ast; expr(ctx, &ast.expr)?),
        })),
    })
}

/// Test if the given pattern is open or not.
fn pat_items_count(items: &[hir::Pat<'_>]) -> Result<(bool, usize), HirError> {
    let mut it = items.iter();

    let (is_open, mut count) = match it.next_back() {
        Some(pat) => matches!(pat.kind, hir::PatKind::PatRest)
            .then(|| (true, 0))
            .unwrap_or((false, 1)),
        None => return Ok((false, 0)),
    };

    for pat in it {
        if let hir::PatKind::PatRest = pat.kind {
            return Err(HirError::new(
                pat.span(),
                HirErrorKind::UnsupportedPatternRest,
            ));
        }

        count += 1;
    }

    Ok((is_open, count))
}
