use crate::ast;
use crate::ast::Spanned;
use crate::compile::ir;
use crate::compile::{IrError, IrValue};
use crate::hir;
use crate::parse::Resolve;
use crate::query::Query;
use crate::runtime::{Bytes, Shared};

/// A c that compiles AST into Rune IR.
pub(crate) struct IrCompiler<'a> {
    pub(crate) q: Query<'a>,
}

impl IrCompiler<'_> {
    /// Resolve the given resolvable value.
    pub(crate) fn resolve<'s, T>(&'s self, value: &T) -> Result<T::Output, IrError>
    where
        T: Resolve<'s>,
    {
        Ok(value.resolve(resolve_context!(self.q))?)
    }

    /// Resolve an ir target from an expression.
    fn ir_target(&self, expr: &hir::Expr<'_>) -> Result<ir::IrTarget, IrError> {
        match *expr {
            hir::Expr::Path(path) => {
                if let Some(ident) = path.try_as_ident() {
                    let name = self.resolve(ident)?;

                    return Ok(ir::IrTarget {
                        span: expr.span(),
                        kind: ir::IrTargetKind::Name(name.into()),
                    });
                }
            }
            hir::Expr::FieldAccess(expr_field_access) => {
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

        Err(IrError::msg(expr, "not supported as a target"))
    }
}

pub(crate) fn expr(hir: &hir::Expr<'_>, c: &mut IrCompiler<'_>) -> Result<ir::Ir, IrError> {
    Ok(match hir {
        hir::Expr::Vec(e) => ir::Ir::new(e.span(), expr_vec(e, c)?),
        hir::Expr::Tuple(e) => expr_tuple(e, c)?,
        hir::Expr::Object(e) => ir::Ir::new(e.span(), expr_object(e, c)?),
        hir::Expr::Group(e) => expr(e, c)?,
        hir::Expr::Binary(e) => expr_binary(e, c)?,
        hir::Expr::Assign(e) => expr_assign(e, c)?,
        hir::Expr::Call(e) => ir::Ir::new(e.span(), expr_call(e, c)?),
        hir::Expr::If(e) => ir::Ir::new(e.span(), expr_if(e, c)?),
        hir::Expr::Loop(e) => ir::Ir::new(e.span(), expr_loop(e, c)?),
        hir::Expr::While(e) => ir::Ir::new(e.span(), expr_while(e, c)?),
        hir::Expr::Lit(e) => expr_lit(e, c)?,
        hir::Expr::Block(e) => expr_block(e, c)?,
        hir::Expr::Path(e) => path(e, c)?,
        hir::Expr::FieldAccess(..) => ir::Ir::new(hir.span(), c.ir_target(hir)?),
        hir::Expr::Break(e) => ir::Ir::new(e, ir::IrBreak::compile_ast(e, c)?),
        hir::Expr::MacroCall(macro_call) => match macro_call {
            hir::MacroCall::Template(template) => {
                let ir_template = builtin_template(template, c)?;
                ir::Ir::new(hir.span(), ir_template)
            }
            hir::MacroCall::File(file) => {
                let s = c.resolve(&file.value)?;
                ir::Ir::new(file.span, IrValue::String(Shared::new(s.into_owned())))
            }
            hir::MacroCall::Line(line) => {
                let n = c.resolve(&line.value)?;

                let const_value = match n {
                    ast::Number::Integer(n) => IrValue::Integer(n),
                    ast::Number::Float(n) => IrValue::Float(n),
                };

                ir::Ir::new(line.span, const_value)
            }
            _ => {
                return Err(IrError::msg(hir, "unsupported builtin macro"));
            }
        },
        _ => return Err(IrError::msg(hir, "not supported yet")),
    })
}

fn expr_assign(hir: &hir::ExprAssign<'_>, c: &mut IrCompiler<'_>) -> Result<ir::Ir, IrError> {
    let span = hir.span();
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

fn expr_call(hir: &hir::ExprCall<'_>, c: &mut IrCompiler<'_>) -> Result<ir::IrCall, IrError> {
    let span = hir.span();

    let mut args = Vec::with_capacity(hir.args.len());

    for e in hir.args {
        args.push(expr(e, c)?);
    }

    if let hir::Expr::Path(path) = &*hir.expr {
        if let Some(ident) = path.try_as_ident() {
            let target = c.resolve(ident)?;

            return Ok(ir::IrCall {
                span,
                target: target.into(),
                args,
            });
        }
    }

    Err(IrError::msg(span, "call not supported"))
}

fn expr_binary(hir: &hir::ExprBinary<'_>, c: &mut IrCompiler<'_>) -> Result<ir::Ir, IrError> {
    let span = hir.span();

    if hir.op.is_assign() {
        let op = match hir.op {
            ast::BinOp::AddAssign(..) => ir::IrAssignOp::Add,
            ast::BinOp::SubAssign(..) => ir::IrAssignOp::Sub,
            ast::BinOp::MulAssign(..) => ir::IrAssignOp::Mul,
            ast::BinOp::DivAssign(..) => ir::IrAssignOp::Div,
            ast::BinOp::ShlAssign(..) => ir::IrAssignOp::Shl,
            ast::BinOp::ShrAssign(..) => ir::IrAssignOp::Shr,
            _ => return Err(IrError::msg(hir.op, "op not supported yet")),
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
        _ => return Err(IrError::msg(hir.op, "op not supported yet")),
    };

    Ok(ir::Ir::new(
        hir.span(),
        ir::IrBinary {
            span,
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        },
    ))
}

fn expr_lit(hir: &hir::ExprLit<'_>, c: &mut IrCompiler<'_>) -> Result<ir::Ir, IrError> {
    let span = hir.span();

    Ok(match hir.lit {
        ast::Lit::Bool(b) => ir::Ir::new(span, IrValue::Bool(b.value)),
        ast::Lit::Str(s) => {
            let s = c.resolve(s)?;
            ir::Ir::new(span, IrValue::String(Shared::new(s.into_owned())))
        }
        ast::Lit::Number(n) => {
            let n = c.resolve(n)?;

            let const_value = match n {
                ast::Number::Integer(n) => IrValue::Integer(n),
                ast::Number::Float(n) => IrValue::Float(n),
            };

            ir::Ir::new(span, const_value)
        }
        ast::Lit::Byte(lit) => {
            let b = c.resolve(lit)?;
            ir::Ir::new(span, IrValue::Byte(b))
        }
        ast::Lit::ByteStr(lit) => {
            let byte_str = c.resolve(lit)?;
            let value = IrValue::Bytes(Shared::new(Bytes::from_vec(byte_str.into_owned())));
            ir::Ir::new(span, value)
        }
        ast::Lit::Char(lit) => {
            let c = c.resolve(lit)?;
            ir::Ir::new(span, IrValue::Char(c))
        }
    })
}

fn expr_tuple(hir: &hir::ExprTuple<'_>, c: &mut IrCompiler<'_>) -> Result<ir::Ir, IrError> {
    let span = hir.span();

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
            span: hir.span(),
            items: items.into_boxed_slice(),
        },
    ))
}

fn expr_vec(hir: &hir::ExprVec<'_>, c: &mut IrCompiler<'_>) -> Result<ir::IrVec, IrError> {
    let mut items = Vec::new();

    for e in hir.items {
        items.push(expr(e, c)?);
    }

    Ok(ir::IrVec {
        span: hir.span(),
        items: items.into_boxed_slice(),
    })
}

fn expr_object(hir: &hir::ExprObject<'_>, c: &mut IrCompiler<'_>) -> Result<ir::IrObject, IrError> {
    let mut assignments = Vec::new();

    for assign in hir.assignments {
        let key = c.resolve(assign.key)?.into_owned().into_boxed_str();

        let ir = if let Some(e) = assign.assign {
            expr(e, c)?
        } else {
            ir::Ir::new(
                assign,
                ir::IrKind::Target(ir::IrTarget {
                    span: assign.span(),
                    kind: ir::IrTargetKind::Name(key.clone()),
                }),
            )
        };

        assignments.push((key, ir))
    }

    Ok(ir::IrObject {
        span: hir.span(),
        assignments: assignments.into_boxed_slice(),
    })
}

pub(crate) fn expr_block(
    hir: &hir::ExprBlock<'_>,
    c: &mut IrCompiler<'_>,
) -> Result<ir::Ir, IrError> {
    Ok(ir::Ir::new(hir.span(), block(hir.block, c)?))
}

pub(crate) fn block(hir: &hir::Block<'_>, c: &mut IrCompiler<'_>) -> Result<ir::IrScope, IrError> {
    let span = hir.span();

    let mut last = None::<(&hir::Expr<'_>, bool)>;
    let mut instructions = Vec::new();

    for stmt in hir.statements {
        let (e, term) = match stmt {
            hir::Stmt::Local(l) => {
                if let Some((e, _)) = std::mem::take(&mut last) {
                    instructions.push(expr(e, c)?);
                }

                instructions.push(local(l, c)?);
                continue;
            }
            hir::Stmt::Expr(e) => (e, false),
            hir::Stmt::Semi(e) => (e, true),
            hir::Stmt::Item(..) => continue,
        };

        if let Some((e, _)) = std::mem::replace(&mut last, Some((e, term))) {
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

fn builtin_template(
    template: &hir::BuiltInTemplate,
    c: &mut IrCompiler<'_>,
) -> Result<ir::IrTemplate, IrError> {
    let span = template.span;
    let mut components = Vec::new();

    for e in template.exprs {
        if let hir::Expr::Lit(hir::ExprLit {
            lit: ast::Lit::Str(s),
            ..
        }) = e
        {
            let s = s.resolve_template_string(resolve_context!(c.q))?;

            components.push(ir::IrTemplateComponent::String(
                s.into_owned().into_boxed_str(),
            ));

            continue;
        }

        let ir = expr(e, c)?;
        components.push(ir::IrTemplateComponent::Ir(ir));
    }

    Ok(ir::IrTemplate { span, components })
}

fn path(hir: &hir::Path<'_>, c: &mut IrCompiler<'_>) -> Result<ir::Ir, IrError> {
    let span = hir.span();

    if let Some(name) = hir.try_as_ident() {
        let name = c.resolve(name)?;
        return Ok(ir::Ir::new(span, <Box<str>>::from(name)));
    }

    Err(IrError::msg(span, "not supported yet"))
}

fn local(hir: &hir::Local<'_>, c: &mut IrCompiler<'_>) -> Result<ir::Ir, IrError> {
    let span = hir.span();

    let name = loop {
        match hir.pat {
            hir::Pat::PatIgnore(_) => {
                return expr(hir.expr, c);
            }
            hir::Pat::PatPath(path) => {
                if let Some(ident) = path.path.try_as_ident() {
                    break ident;
                }
            }
            _ => (),
        }

        return Err(IrError::msg(span, "not supported yet"));
    };

    Ok(ir::Ir::new(
        span,
        ir::IrDecl {
            span,
            name: c.resolve(name)?.into(),
            value: Box::new(expr(hir.expr, c)?),
        },
    ))
}

fn condition(hir: &hir::Condition<'_>, c: &mut IrCompiler<'_>) -> Result<ir::IrCondition, IrError> {
    match hir {
        hir::Condition::Expr(e) => Ok(ir::IrCondition::Ir(expr(e, c)?)),
        hir::Condition::ExprLet(expr_let) => {
            let pat = ir::IrPat::compile_ast(expr_let.pat, c)?;
            let ir = expr(expr_let.expr, c)?;

            Ok(ir::IrCondition::Let(ir::IrLet {
                span: expr_let.span(),
                pat,
                ir,
            }))
        }
    }
}

fn expr_if(hir: &hir::ExprIf<'_>, c: &mut IrCompiler<'_>) -> Result<ir::IrBranches, IrError> {
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
        branches,
        default_branch,
    })
}

fn expr_while(hir: &hir::ExprWhile<'_>, c: &mut IrCompiler<'_>) -> Result<ir::IrLoop, IrError> {
    Ok(ir::IrLoop {
        span: hir.span(),
        label: match hir.label {
            Some(label) => Some(c.resolve(label)?.into()),
            None => None,
        },
        condition: Some(Box::new(condition(hir.condition, c)?)),
        body: block(hir.body, c)?,
    })
}

fn expr_loop(hir: &hir::ExprLoop<'_>, c: &mut IrCompiler<'_>) -> Result<ir::IrLoop, IrError> {
    Ok(ir::IrLoop {
        span: hir.span(),
        label: match hir.label {
            Some(label) => Some(c.resolve(label)?.into()),
            None => None,
        },
        condition: None,
        body: block(hir.body, c)?,
    })
}
