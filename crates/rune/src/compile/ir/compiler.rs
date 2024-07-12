use core::mem::take;

use crate::alloc::prelude::*;
use crate::alloc::{try_format, Box, Vec};
use crate::ast::{self, Span, Spanned};
use crate::compile::ir;
use crate::compile::{self, ErrorKind, WithSpan};
use crate::hir;
use crate::query::Query;
use crate::runtime::{Bytes, Value};
use crate::SourceId;

use rune_macros::instrument;

/// A c that compiles AST into Rune IR.
pub(crate) struct Ctxt<'a, 'arena> {
    /// The source id of the source.
    pub(crate) source_id: SourceId,
    /// Query associated with the compiler.
    pub(crate) q: Query<'a, 'arena>,
}

#[instrument]
pub(crate) fn expr(hir: &hir::Expr<'_>, c: &mut Ctxt<'_, '_>) -> compile::Result<ir::Ir> {
    let span = hir.span();

    Ok(match hir.kind {
        hir::ExprKind::Vec(hir) => ir::Ir::new(span, expr_vec(span, c, hir)?),
        hir::ExprKind::Tuple(hir) => expr_tuple(c, span, hir)?,
        hir::ExprKind::Object(hir) => ir::Ir::new(span, expr_object(span, c, hir)?),
        hir::ExprKind::Group(hir) => expr(hir, c)?,
        hir::ExprKind::Binary(hir) => expr_binary(span, c, hir)?,
        hir::ExprKind::Assign(hir) => expr_assign(span, c, hir)?,
        hir::ExprKind::Call(hir) => ir::Ir::new(span, expr_call(span, c, hir)?),
        hir::ExprKind::If(hir) => ir::Ir::new(span, expr_if(span, c, hir)?),
        hir::ExprKind::Loop(hir) => ir::Ir::new(span, expr_loop(span, c, hir)?),
        hir::ExprKind::Lit(hir) => lit(c, span, hir)?,
        hir::ExprKind::Block(hir) => ir::Ir::new(span, block(hir, c)?),
        hir::ExprKind::FieldAccess(..) => ir::Ir::new(span, ir_target(hir)?),
        hir::ExprKind::Break(hir) => ir::Ir::new(span, ir::IrBreak::compile_ast(span, c, hir)?),
        hir::ExprKind::Template(template) => {
            let ir_template = builtin_template(template, c)?;
            ir::Ir::new(hir.span(), ir_template)
        }
        hir::ExprKind::Const(hash) => {
            let Some(value) = c.q.get_const_value(hash) else {
                return Err(compile::Error::msg(
                    hir,
                    try_format!("Missing constant for hash {hash}"),
                ));
            };

            let value = value.as_value().with_span(span)?;
            ir::Ir::new(span, value)
        }
        hir::ExprKind::Variable(name) => {
            return Ok(ir::Ir::new(span, name.into_owned()?));
        }
        _ => {
            return Err(compile::Error::msg(
                hir,
                "Expression kind not supported yet in constant contexts",
            ))
        }
    })
}

/// Resolve an ir target from an expression.
fn ir_target(expr: &hir::Expr<'_>) -> compile::Result<ir::IrTarget> {
    match expr.kind {
        hir::ExprKind::Variable(name) => {
            return Ok(ir::IrTarget {
                span: expr.span(),
                kind: ir::IrTargetKind::Name(name.into_owned()?),
            });
        }
        hir::ExprKind::FieldAccess(expr_field_access) => {
            let target = ir_target(&expr_field_access.expr)?;

            match expr_field_access.expr_field {
                hir::ExprField::Ident(name) => {
                    return Ok(ir::IrTarget {
                        span: expr.span(),
                        kind: ir::IrTargetKind::Field(Box::try_new(target)?, name.try_into()?),
                    });
                }
                hir::ExprField::Index(index) => {
                    return Ok(ir::IrTarget {
                        span: expr.span(),
                        kind: ir::IrTargetKind::Index(Box::try_new(target)?, index),
                    });
                }
                _ => {
                    return Err(compile::Error::new(expr, ErrorKind::BadFieldAccess));
                }
            }
        }
        _ => (),
    }

    Err(compile::Error::msg(expr, "Not supported as a target"))
}

#[instrument]
fn expr_assign(
    span: Span,
    c: &mut Ctxt<'_, '_>,
    hir: &hir::ExprAssign<'_>,
) -> compile::Result<ir::Ir> {
    let target = ir_target(&hir.lhs)?;

    Ok(ir::Ir::new(
        span,
        ir::IrSet {
            span,
            target,
            value: Box::try_new(expr(&hir.rhs, c)?)?,
        },
    ))
}

#[instrument]
fn expr_call(
    span: Span,
    c: &mut Ctxt<'_, '_>,
    hir: &hir::ExprCall<'_>,
) -> compile::Result<ir::IrCall> {
    let mut args = Vec::try_with_capacity(hir.args.len())?;

    for e in hir.args {
        args.try_push(expr(e, c)?)?;
    }

    if let hir::Call::ConstFn { id, .. } = hir.call {
        return Ok(ir::IrCall { span, id, args });
    }

    Err(compile::Error::msg(
        span,
        "Call not supported in constant contexts",
    ))
}

#[instrument]
fn expr_binary(
    span: Span,
    c: &mut Ctxt<'_, '_>,
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

        let target = ir_target(&hir.lhs)?;

        return Ok(ir::Ir::new(
            span,
            ir::IrAssign {
                span,
                target,
                value: Box::try_new(expr(&hir.rhs, c)?)?,
                op,
            },
        ));
    }

    let lhs = expr(&hir.lhs, c)?;
    let rhs = expr(&hir.rhs, c)?;

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
            lhs: Box::try_new(lhs)?,
            rhs: Box::try_new(rhs)?,
        },
    ))
}

#[instrument(span = span)]
fn lit(c: &mut Ctxt<'_, '_>, span: Span, hir: hir::Lit<'_>) -> compile::Result<ir::Ir> {
    Ok(match hir {
        hir::Lit::Bool(boolean) => {
            let value = Value::try_from(boolean).with_span(span)?;
            ir::Ir::new(span, value)
        }
        hir::Lit::Str(string) => {
            let string = string.try_to_owned().with_span(span)?;
            let value = Value::try_from(string).with_span(span)?;
            ir::Ir::new(span, value)
        }
        hir::Lit::Integer(n) => {
            let value = Value::try_from(n).with_span(span)?;
            ir::Ir::new(span, value)
        }
        hir::Lit::Float(n) => {
            let value = Value::try_from(n).with_span(span)?;
            ir::Ir::new(span, value)
        }
        hir::Lit::Byte(b) => {
            let value = Value::try_from(b).with_span(span)?;
            ir::Ir::new(span, value)
        }
        hir::Lit::ByteStr(byte_str) => {
            let value = Bytes::from_vec(Vec::try_from(byte_str).with_span(span)?);
            let value = Value::try_from(value).with_span(span)?;
            ir::Ir::new(span, value)
        }
        hir::Lit::Char(c) => {
            let value = Value::try_from(c).with_span(span)?;
            ir::Ir::new(span, value)
        }
    })
}

#[instrument(span = span)]
fn expr_tuple(c: &mut Ctxt<'_, '_>, span: Span, hir: &hir::ExprSeq<'_>) -> compile::Result<ir::Ir> {
    if hir.items.is_empty() {
        let value = Value::empty().with_span(span)?;
        return Ok(ir::Ir::new(span, value));
    }

    let mut items = Vec::new();

    for e in hir.items {
        items.try_push(expr(e, c)?)?;
    }

    Ok(ir::Ir::new(
        span,
        ir::Tuple {
            span,
            items: items.try_into_boxed_slice()?,
        },
    ))
}

#[instrument]
fn expr_vec(
    span: Span,
    c: &mut Ctxt<'_, '_>,
    hir: &hir::ExprSeq<'_>,
) -> compile::Result<ir::IrVec> {
    let mut items = Vec::new();

    for e in hir.items {
        items.try_push(expr(e, c)?)?;
    }

    Ok(ir::IrVec {
        span,
        items: items.try_into_boxed_slice()?,
    })
}

#[instrument]
fn expr_object(
    span: Span,
    c: &mut Ctxt<'_, '_>,
    hir: &hir::ExprObject<'_>,
) -> compile::Result<ir::IrObject> {
    let mut assignments = Vec::new();

    for assign in hir.assignments {
        let (_, key) = assign.key;
        let ir = expr(&assign.assign, c)?;
        assignments.try_push((key.try_into()?, ir))?
    }

    Ok(ir::IrObject {
        span,
        assignments: assignments.try_into_boxed_slice()?,
    })
}

#[instrument]
pub(crate) fn block(hir: &hir::Block<'_>, c: &mut Ctxt<'_, '_>) -> compile::Result<ir::IrScope> {
    let span = hir.span();

    let mut last = None::<(&hir::Expr<'_>, bool)>;
    let mut instructions = Vec::new();

    for stmt in hir.statements {
        match stmt {
            hir::Stmt::Local(l) => {
                if let Some((e, _)) = take(&mut last) {
                    instructions.try_push(expr(e, c)?)?;
                }

                instructions.try_push(local(l, c)?)?;
            }
            hir::Stmt::Expr(e) => {
                instructions.try_push(expr(e, c)?)?;
            }
        };
    }

    let last = if let Some(e) = hir.value {
        Some(Box::try_new(expr(e, c)?)?)
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
    c: &mut Ctxt<'_, '_>,
) -> compile::Result<ir::IrTemplate> {
    let span = template.span;
    let mut components = Vec::new();

    for e in template.exprs {
        if let hir::ExprKind::Lit(hir::Lit::Str(s)) = e.kind {
            components.try_push(ir::IrTemplateComponent::String(s.try_into()?))?;
            continue;
        }

        let ir = expr(e, c)?;
        components.try_push(ir::IrTemplateComponent::Ir(ir))?;
    }

    Ok(ir::IrTemplate { span, components })
}

#[instrument]
fn local(hir: &hir::Local<'_>, c: &mut Ctxt<'_, '_>) -> compile::Result<ir::Ir> {
    let span = hir.span();

    let name = match hir.pat.kind {
        hir::PatKind::Ignore => {
            return expr(&hir.expr, c);
        }
        hir::PatKind::Path(&hir::PatPathKind::Ident(name)) => name,
        _ => {
            return Err(compile::Error::msg(span, "not supported yet"));
        }
    };

    Ok(ir::Ir::new(
        span,
        ir::IrDecl {
            span,
            name: hir::Name::Str(name).into_owned()?,
            value: Box::try_new(expr(&hir.expr, c)?)?,
        },
    ))
}

#[instrument]
fn condition(hir: &hir::Condition<'_>, c: &mut Ctxt<'_, '_>) -> compile::Result<ir::IrCondition> {
    match hir {
        hir::Condition::Expr(e) => Ok(ir::IrCondition::Ir(expr(e, c)?)),
        hir::Condition::ExprLet(hir) => {
            let pat = ir::IrPat::compile_ast(&hir.pat)?;
            let ir = expr(&hir.expr, c)?;

            Ok(ir::IrCondition::Let(ir::IrLet {
                span: hir.span(),
                pat,
                ir,
            }))
        }
    }
}

#[instrument]
fn expr_if(
    span: Span,
    c: &mut Ctxt<'_, '_>,
    hir: &hir::Conditional<'_>,
) -> compile::Result<ir::IrBranches> {
    let mut branches = Vec::new();

    for hir in hir.branches {
        let cond = condition(hir.condition, c)?;
        let ir = block(&hir.block, c)?;
        branches.try_push((cond, ir))?;
    }

    let default_branch = match hir.fallback {
        Some(hir) => Some(block(hir, c)?),
        None => None,
    };

    Ok(ir::IrBranches {
        span,
        branches,
        default_branch,
    })
}

#[instrument]
fn expr_loop(
    span: Span,
    c: &mut Ctxt<'_, '_>,
    hir: &hir::ExprLoop<'_>,
) -> compile::Result<ir::IrLoop> {
    Ok(ir::IrLoop {
        span,
        label: hir.label.map(TryInto::try_into).transpose()?,
        condition: match hir.condition {
            Some(hir) => Some(Box::try_new(condition(hir, c)?)?),
            None => None,
        },
        body: block(&hir.body, c)?,
    })
}
