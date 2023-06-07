use core::mem::{replace, take};

use crate::no_std::prelude::*;

use crate::ast::{self, Span, Spanned};
use crate::compile;
use crate::compile::ir::{self, IrValue};
use crate::hir;
use crate::parse::Resolve;
use crate::query::Query;
use crate::runtime::{Bytes, Shared};
use crate::SourceId;

use rune_macros::instrument;

/// A c that compiles AST into Rune IR.
pub(crate) struct IrCompiler<'a> {
    /// The source id of the source.
    pub(crate) source_id: SourceId,
    /// Query associated with the compiler.
    pub(crate) q: Query<'a>,
}

impl IrCompiler<'_> {
    /// Resolve the given resolvable value.
    pub(crate) fn resolve<'s, T>(&'s self, value: &T) -> compile::Result<T::Output>
    where
        T: Resolve<'s>,
    {
        value.resolve(resolve_context!(self.q))
    }

    /// Resolve an ir target from an expression.
    fn ir_target(&self, expr: &hir::Expr<'_>) -> compile::Result<ir::IrTarget> {
        match expr.kind {
            hir::ExprKind::Path(path) => {
                if let Some(ident) = path.try_as_ident() {
                    let name = self.resolve(ident)?;

                    return Ok(ir::IrTarget {
                        span: expr.span(),
                        kind: ir::IrTargetKind::Name(name.into()),
                    });
                }
            }
            hir::ExprKind::FieldAccess(expr_field_access) => {
                let target = self.ir_target(expr_field_access.expr)?;

                match *expr_field_access.expr_field {
                    hir::ExprField::Path(field) => {
                        if let Some(ident) = field.try_as_ident() {
                            let name = self.resolve(ident)?;

                            return Ok(ir::IrTarget {
                                span: expr.span(),
                                kind: ir::IrTargetKind::Field(Box::new(target), name.into()),
                            });
                        }
                    }
                    hir::ExprField::LitNumber(number) => {
                        let number = self.resolve(number)?;

                        if let Some(index) = number.as_tuple_index() {
                            return Ok(ir::IrTarget {
                                span: expr.span(),
                                kind: ir::IrTargetKind::Index(Box::new(target), index),
                            });
                        }
                    }
                }
            }
            _ => (),
        }

        Err(compile::Error::msg(expr, "not supported as a target"))
    }
}

#[instrument]
pub(crate) fn expr(hir: &hir::Expr<'_>, c: &mut IrCompiler<'_>) -> compile::Result<ir::Ir> {
    let span = hir.span();

    Ok(match hir.kind {
        hir::ExprKind::Vec(hir) => ir::Ir::new(span, expr_vec(span, c, hir)?),
        hir::ExprKind::Tuple(hir) => expr_tuple(span, c, hir)?,
        hir::ExprKind::Object(hir) => ir::Ir::new(span, expr_object(span, c, hir)?),
        hir::ExprKind::Group(hir) => expr(hir, c)?,
        hir::ExprKind::Binary(hir) => expr_binary(span, c, hir)?,
        hir::ExprKind::Assign(hir) => expr_assign(span, c, hir)?,
        hir::ExprKind::Call(hir) => ir::Ir::new(span, expr_call(span, c, hir)?),
        hir::ExprKind::If(hir) => ir::Ir::new(span, expr_if(span, c, hir)?),
        hir::ExprKind::Loop(hir) => ir::Ir::new(span, expr_loop(span, c, hir)?),
        hir::ExprKind::Lit(hir) => lit(span, c, hir)?,
        hir::ExprKind::Block(hir) => expr_block(span, c, hir)?,
        hir::ExprKind::Path(hir) => path(hir, c)?,
        hir::ExprKind::FieldAccess(..) => ir::Ir::new(span, c.ir_target(hir)?),
        hir::ExprKind::Break(hir) => ir::Ir::new(span, ir::IrBreak::compile_ast(span, c, hir)?),
        hir::ExprKind::Template(template) => {
            let ir_template = builtin_template(template, c)?;
            ir::Ir::new(hir.span(), ir_template)
        }
        _ => return Err(compile::Error::msg(hir, "not supported yet")),
    })
}

#[instrument]
fn expr_assign(
    span: Span,
    c: &mut IrCompiler<'_>,
    hir: &hir::ExprAssign<'_>,
) -> compile::Result<ir::Ir> {
    let target = c.ir_target(hir.lhs)?;

    Ok(ir::Ir::new(
        span,
        ir::IrSet {
            span,
            target,
            value: Box::new(expr(hir.rhs, c)?),
        },
    ))
}

#[instrument]
fn expr_call(
    span: Span,
    c: &mut IrCompiler<'_>,
    hir: &hir::ExprCall<'_>,
) -> compile::Result<ir::IrCall> {
    let mut args = Vec::with_capacity(hir.args.len());

    for e in hir.args {
        args.push(expr(e, c)?);
    }

    if let hir::ExprKind::Path(path) = hir.expr.kind {
        if let Some(ident) = path.try_as_ident() {
            let target = c.resolve(ident)?;

            return Ok(ir::IrCall {
                span,
                target: target.into(),
                args,
            });
        }
    }

    Err(compile::Error::msg(span, "call not supported"))
}

#[instrument]
fn expr_binary(
    span: Span,
    c: &mut IrCompiler<'_>,
    hir: &hir::ExprBinary<'_>,
) -> compile::Result<ir::Ir> {
    if hir.op.is_assign() {
        let op = match hir.op {
            ast::BinOp::AddAssign(..) => ir::IrAssignOp::Add,
            ast::BinOp::SubAssign(..) => ir::IrAssignOp::Sub,
            ast::BinOp::MulAssign(..) => ir::IrAssignOp::Mul,
            ast::BinOp::DivAssign(..) => ir::IrAssignOp::Div,
            ast::BinOp::ShlAssign(..) => ir::IrAssignOp::Shl,
            ast::BinOp::ShrAssign(..) => ir::IrAssignOp::Shr,
            _ => return Err(compile::Error::msg(hir.op, "op not supported yet")),
        };

        let target = c.ir_target(hir.lhs)?;

        return Ok(ir::Ir::new(
            span,
            ir::IrAssign {
                span,
                target,
                value: Box::new(expr(hir.rhs, c)?),
                op,
            },
        ));
    }

    let lhs = expr(hir.lhs, c)?;
    let rhs = expr(hir.rhs, c)?;

    let op = match hir.op {
        ast::BinOp::Add(..) => ir::IrBinaryOp::Add,
        ast::BinOp::Sub(..) => ir::IrBinaryOp::Sub,
        ast::BinOp::Mul(..) => ir::IrBinaryOp::Mul,
        ast::BinOp::Div(..) => ir::IrBinaryOp::Div,
        ast::BinOp::Shl(..) => ir::IrBinaryOp::Shl,
        ast::BinOp::Shr(..) => ir::IrBinaryOp::Shr,
        ast::BinOp::Lt(..) => ir::IrBinaryOp::Lt,
        ast::BinOp::Lte(..) => ir::IrBinaryOp::Lte,
        ast::BinOp::Eq(..) => ir::IrBinaryOp::Eq,
        ast::BinOp::Gt(..) => ir::IrBinaryOp::Gt,
        ast::BinOp::Gte(..) => ir::IrBinaryOp::Gte,
        _ => return Err(compile::Error::msg(hir.op, "op not supported yet")),
    };

    Ok(ir::Ir::new(
        span,
        ir::IrBinary {
            span,
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        },
    ))
}

#[instrument]
fn lit(span: Span, c: &mut IrCompiler<'_>, hir: hir::Lit<'_>) -> compile::Result<ir::Ir> {
    Ok(match hir {
        hir::Lit::Bool(boolean) => ir::Ir::new(span, IrValue::Bool(boolean)),
        hir::Lit::Str(string) => ir::Ir::new(span, IrValue::String(Shared::new(string.to_owned()))),
        hir::Lit::Integer(n) => ir::Ir::new(span, IrValue::Integer(n)),
        hir::Lit::Float(n) => ir::Ir::new(span, IrValue::Float(n)),
        hir::Lit::Byte(b) => ir::Ir::new(span, IrValue::Byte(b)),
        hir::Lit::ByteStr(byte_str) => {
            let value = IrValue::Bytes(Shared::new(Bytes::from_vec(byte_str.to_vec())));
            ir::Ir::new(span, value)
        }
        hir::Lit::Char(c) => ir::Ir::new(span, IrValue::Char(c)),
    })
}

#[instrument]
fn expr_tuple(
    span: Span,
    c: &mut IrCompiler<'_>,
    hir: &hir::ExprSeq<'_>,
) -> compile::Result<ir::Ir> {
    if hir.items.is_empty() {
        return Ok(ir::Ir::new(span, IrValue::Unit));
    }

    let mut items = Vec::new();

    for e in hir.items {
        items.push(expr(e, c)?);
    }

    Ok(ir::Ir::new(
        span,
        ir::IrTuple {
            span,
            items: items.into_boxed_slice(),
        },
    ))
}

#[instrument]
fn expr_vec(
    span: Span,
    c: &mut IrCompiler<'_>,
    hir: &hir::ExprSeq<'_>,
) -> compile::Result<ir::IrVec> {
    let mut items = Vec::new();

    for e in hir.items {
        items.push(expr(e, c)?);
    }

    Ok(ir::IrVec {
        span,
        items: items.into_boxed_slice(),
    })
}

#[instrument]
fn expr_object(
    span: Span,
    c: &mut IrCompiler<'_>,
    hir: &hir::ExprObject<'_>,
) -> compile::Result<ir::IrObject> {
    let mut assignments = Vec::new();

    for assign in hir.assignments {
        let (span, key) = assign.key;

        let ir = if let Some(e) = assign.assign {
            expr(e, c)?
        } else {
            ir::Ir::new(
                span,
                ir::IrKind::Target(ir::IrTarget {
                    span,
                    kind: ir::IrTargetKind::Name(key.into()),
                }),
            )
        };

        assignments.push((key.into(), ir))
    }

    Ok(ir::IrObject {
        span,
        assignments: assignments.into_boxed_slice(),
    })
}

#[instrument]
pub(crate) fn expr_block(
    span: Span,
    c: &mut IrCompiler<'_>,
    hir: &hir::ExprBlock<'_>,
) -> compile::Result<ir::Ir> {
    Ok(ir::Ir::new(span, block(hir.block, c)?))
}

#[instrument]
pub(crate) fn block(hir: &hir::Block<'_>, c: &mut IrCompiler<'_>) -> compile::Result<ir::IrScope> {
    let span = hir.span();

    let mut last = None::<(&hir::Expr<'_>, bool)>;
    let mut instructions = Vec::new();

    for stmt in hir.statements {
        let (e, term) = match stmt {
            hir::Stmt::Local(l) => {
                if let Some((e, _)) = take(&mut last) {
                    instructions.push(expr(e, c)?);
                }

                instructions.push(local(l, c)?);
                continue;
            }
            hir::Stmt::Expr(e) => (e, false),
            hir::Stmt::Semi(e) => (e, true),
            hir::Stmt::Item(..) => continue,
        };

        if let Some((e, _)) = replace(&mut last, Some((e, term))) {
            instructions.push(expr(e, c)?);
        }
    }

    let last = if let Some((e, term)) = last {
        if term {
            instructions.push(expr(e, c)?);
            None
        } else {
            Some(Box::new(expr(e, c)?))
        }
    } else {
        None
    };

    Ok(ir::IrScope {
        span,
        instructions,
        last,
    })
}

#[instrument]
fn builtin_template(
    template: &hir::BuiltInTemplate,
    c: &mut IrCompiler<'_>,
) -> compile::Result<ir::IrTemplate> {
    let span = template.span;
    let mut components = Vec::new();

    for e in template.exprs {
        if let hir::ExprKind::Lit(hir::Lit::Str(s)) = e.kind {
            components.push(ir::IrTemplateComponent::String(s.into()));
            continue;
        }

        let ir = expr(e, c)?;
        components.push(ir::IrTemplateComponent::Ir(ir));
    }

    Ok(ir::IrTemplate { span, components })
}

#[instrument]
fn path(hir: &hir::Path<'_>, c: &mut IrCompiler<'_>) -> compile::Result<ir::Ir> {
    let span = hir.span();

    if let Some(name) = hir.try_as_ident() {
        let name = c.resolve(name)?;
        return Ok(ir::Ir::new(span, <Box<str>>::from(name)));
    }

    Err(compile::Error::msg(span, "not supported yet"))
}

#[instrument]
fn local(hir: &hir::Local<'_>, c: &mut IrCompiler<'_>) -> compile::Result<ir::Ir> {
    let span = hir.span();

    let name = 'ok: {
        match hir.pat.kind {
            hir::PatKind::Ignore => {
                return expr(hir.expr, c);
            }
            hir::PatKind::Path(&hir::PatPathKind::Ident(ident)) => {
                break 'ok ident;
            }
            _ => (),
        }

        return Err(compile::Error::msg(span, "not supported yet"));
    };

    Ok(ir::Ir::new(
        span,
        ir::IrDecl {
            span,
            name: name.into(),
            value: Box::new(expr(hir.expr, c)?),
        },
    ))
}

#[instrument]
fn condition(hir: &hir::Condition<'_>, c: &mut IrCompiler<'_>) -> compile::Result<ir::IrCondition> {
    match hir {
        hir::Condition::Expr(e) => Ok(ir::IrCondition::Ir(expr(e, c)?)),
        hir::Condition::ExprLet(expr_let) => {
            let pat = ir::IrPat::compile_ast(expr_let.pat)?;
            let ir = expr(expr_let.expr, c)?;

            Ok(ir::IrCondition::Let(ir::IrLet {
                span: expr_let.span(),
                pat,
                ir,
            }))
        }
    }
}

#[instrument]
fn expr_if(
    span: Span,
    c: &mut IrCompiler<'_>,
    hir: &hir::ExprIf<'_>,
) -> compile::Result<ir::IrBranches> {
    let mut branches = Vec::new();
    let mut default_branch = None;

    let cond = condition(hir.condition, c)?;
    let ir = block(hir.block, c)?;
    branches.push((cond, ir));

    for expr_else_if in hir.expr_else_ifs {
        let cond = condition(expr_else_if.condition, c)?;
        let ir = block(expr_else_if.block, c)?;
        branches.push((cond, ir));
    }

    if let Some(expr_else) = hir.expr_else {
        let ir = block(expr_else.block, c)?;
        default_branch = Some(ir);
    }

    Ok(ir::IrBranches {
        span,
        branches,
        default_branch,
    })
}

#[instrument]
fn expr_loop(
    span: Span,
    c: &mut IrCompiler<'_>,
    hir: &hir::ExprLoop<'_>,
) -> compile::Result<ir::IrLoop> {
    Ok(ir::IrLoop {
        span,
        label: match hir.label {
            Some(label) => Some(c.resolve(label)?.into()),
            None => None,
        },
        condition: match hir.condition {
            Some(hir) => Some(Box::new(condition(hir, c)?)),
            None => None,
        },
        body: block(hir.body, c)?,
    })
}
