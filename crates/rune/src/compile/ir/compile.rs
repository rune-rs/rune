use crate::ast;
use crate::ast::Spanned;
use crate::compile::ir;
use crate::compile::{IrError, IrValue};
use crate::parse::Resolve;
use crate::query::{BuiltInMacro, BuiltInTemplate, Query};
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
    fn ir_target(&self, expr: &ast::Expr) -> Result<ir::IrTarget, IrError> {
        match expr {
            ast::Expr::Path(path) => {
                if let Some(ident) = path.try_as_ident() {
                    let name = self.resolve(ident)?;

                    return Ok(ir::IrTarget {
                        span: expr.span(),
                        kind: ir::IrTargetKind::Name(name.into()),
                    });
                }
            }
            ast::Expr::FieldAccess(expr_field_access) => {
                let target = self.ir_target(&expr_field_access.expr)?;

                match &expr_field_access.expr_field {
                    ast::ExprField::Path(field) => {
                        let field = self.resolve(field)?;

                        return Ok(ir::IrTarget {
                            span: expr.span(),
                            kind: ir::IrTargetKind::Field(Box::new(target), field),
                        });
                    }
                    ast::ExprField::LitNumber(number) => {
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

pub(crate) fn expr(ast: &ast::Expr, c: &mut IrCompiler<'_>) -> Result<ir::Ir, IrError> {
    Ok(match ast {
        ast::Expr::Vec(e) => ir::Ir::new(e.span(), expr_vec(e, c)?),
        ast::Expr::Tuple(e) => expr_tuple(e, c)?,
        ast::Expr::Object(e) => ir::Ir::new(e.span(), expr_object(e, c)?),
        ast::Expr::Group(e) => expr(&e.expr, c)?,
        ast::Expr::Empty(e) => expr(&e.expr, c)?,
        ast::Expr::Binary(e) => expr_binary(e, c)?,
        ast::Expr::Assign(e) => expr_assign(e, c)?,
        ast::Expr::Call(e) => ir::Ir::new(e.span(), expr_call(e, c)?),
        ast::Expr::If(e) => ir::Ir::new(e.span(), expr_if(e, c)?),
        ast::Expr::Loop(e) => ir::Ir::new(e.span(), expr_loop(e, c)?),
        ast::Expr::While(e) => ir::Ir::new(e.span(), expr_while(e, c)?),
        ast::Expr::Lit(e) => expr_lit(e, c)?,
        ast::Expr::Block(e) => expr_block(e, c)?,
        ast::Expr::Path(e) => path(e, c)?,
        ast::Expr::FieldAccess(..) => ir::Ir::new(ast.span(), c.ir_target(ast)?),
        ast::Expr::Break(expr_break) => {
            ir::Ir::new(expr_break, ir::IrBreak::compile_ast(expr_break, c)?)
        }
        ast::Expr::MacroCall(macro_call) => {
            let internal_macro = c.q.builtin_macro_for(&*macro_call)?;

            match &*internal_macro {
                BuiltInMacro::Template(template) => {
                    let ir_template = built_in_template(template, c)?;
                    ir::Ir::new(ast.span(), ir_template)
                }
                BuiltInMacro::File(file) => {
                    let s = c.resolve(&file.value)?;
                    ir::Ir::new(file.span, IrValue::String(Shared::new(s.into_owned())))
                }
                BuiltInMacro::Line(line) => {
                    let n = c.resolve(&line.value)?;

                    let const_value = match n {
                        ast::Number::Integer(n) => IrValue::Integer(n),
                        ast::Number::Float(n) => IrValue::Float(n),
                    };

                    ir::Ir::new(line.span, const_value)
                }
                _ => {
                    return Err(IrError::msg(ast, "unsupported builtin macro"));
                }
            }
        }
        _ => return Err(IrError::msg(ast, "not supported yet")),
    })
}

fn expr_assign(ast: &ast::ExprAssign, c: &mut IrCompiler<'_>) -> Result<ir::Ir, IrError> {
    let span = ast.span();
    let target = c.ir_target(&ast.lhs)?;

    Ok(ir::Ir::new(
        span,
        ir::IrSet {
            span,
            target,
            value: Box::new(expr(&ast.rhs, c)?),
        },
    ))
}

fn expr_call(ast: &ast::ExprCall, c: &mut IrCompiler<'_>) -> Result<ir::IrCall, IrError> {
    let span = ast.span();

    let mut args = Vec::with_capacity(ast.args.len());

    for (e, _) in &ast.args {
        args.push(expr(e, c)?);
    }

    if let ast::Expr::Path(path) = &*ast.expr {
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

fn expr_binary(ast: &ast::ExprBinary, c: &mut IrCompiler<'_>) -> Result<ir::Ir, IrError> {
    let span = ast.span();

    if ast.op.is_assign() {
        let op = match &ast.op {
            ast::BinOp::AddAssign(..) => ir::IrAssignOp::Add,
            ast::BinOp::SubAssign(..) => ir::IrAssignOp::Sub,
            ast::BinOp::MulAssign(..) => ir::IrAssignOp::Mul,
            ast::BinOp::DivAssign(..) => ir::IrAssignOp::Div,
            ast::BinOp::ShlAssign(..) => ir::IrAssignOp::Shl,
            ast::BinOp::ShrAssign(..) => ir::IrAssignOp::Shr,
            _ => return Err(IrError::msg(&ast.op, "op not supported yet")),
        };

        let target = c.ir_target(&ast.lhs)?;

        return Ok(ir::Ir::new(
            span,
            ir::IrAssign {
                span,
                target,
                value: Box::new(expr(&ast.rhs, c)?),
                op,
            },
        ));
    }

    let lhs = expr(&ast.lhs, c)?;
    let rhs = expr(&ast.rhs, c)?;

    let op = match &ast.op {
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
        _ => return Err(IrError::msg(&ast.op, "op not supported yet")),
    };

    Ok(ir::Ir::new(
        ast.span(),
        ir::IrBinary {
            span,
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        },
    ))
}

fn expr_lit(ast: &ast::ExprLit, c: &mut IrCompiler<'_>) -> Result<ir::Ir, IrError> {
    let span = ast.span();

    Ok(match &ast.lit {
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

fn expr_tuple(ast: &ast::ExprTuple, c: &mut IrCompiler<'_>) -> Result<ir::Ir, IrError> {
    let span = ast.span();

    if ast.items.is_empty() {
        return Ok(ir::Ir::new(span, IrValue::Unit));
    }

    let mut items = Vec::new();

    for (e, _) in &ast.items {
        items.push(expr(e, c)?);
    }

    Ok(ir::Ir::new(
        span,
        ir::IrTuple {
            span: ast.span(),
            items: items.into_boxed_slice(),
        },
    ))
}

fn expr_vec(ast: &ast::ExprVec, c: &mut IrCompiler<'_>) -> Result<ir::IrVec, IrError> {
    let mut items = Vec::new();

    for (e, _) in &ast.items {
        items.push(expr(e, c)?);
    }

    Ok(ir::IrVec {
        span: ast.span(),
        items: items.into_boxed_slice(),
    })
}

fn expr_object(ast: &ast::ExprObject, c: &mut IrCompiler<'_>) -> Result<ir::IrObject, IrError> {
    let mut assignments = Vec::new();

    for (assign, _) in &ast.assignments {
        let key = c.resolve(&assign.key)?.into_owned().into_boxed_str();

        let ir = if let Some((_, e)) = &assign.assign {
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
        span: ast.span(),
        assignments: assignments.into_boxed_slice(),
    })
}

pub(crate) fn expr_block(ast: &ast::ExprBlock, c: &mut IrCompiler<'_>) -> Result<ir::Ir, IrError> {
    Ok(ir::Ir::new(ast.span(), block(&ast.block, c)?))
}

pub(crate) fn block(ast: &ast::Block, c: &mut IrCompiler<'_>) -> Result<ir::IrScope, IrError> {
    let span = ast.span();

    let mut last = None::<(&ast::Expr, bool)>;
    let mut instructions = Vec::new();

    for stmt in &ast.statements {
        let (e, term) = match stmt {
            ast::Stmt::Local(l) => {
                if let Some((e, _)) = std::mem::take(&mut last) {
                    instructions.push(expr(e, c)?);
                }

                instructions.push(local(l, c)?);
                continue;
            }
            ast::Stmt::Expr(e, semi) => (e, semi.is_some()),
            ast::Stmt::Item(..) => continue,
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

fn built_in_template(
    template: &BuiltInTemplate,
    c: &mut IrCompiler<'_>,
) -> Result<ir::IrTemplate, IrError> {
    let span = template.span;
    let mut components = Vec::new();

    for e in &template.exprs {
        if let ast::Expr::Lit(ast::ExprLit {
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

fn path(ast: &ast::Path, c: &mut IrCompiler<'_>) -> Result<ir::Ir, IrError> {
    let span = ast.span();

    if let Some(name) = ast.try_as_ident() {
        let name = c.resolve(name)?;
        return Ok(ir::Ir::new(span, <Box<str>>::from(name)));
    }

    Err(IrError::msg(span, "not supported yet"))
}

fn local(ast: &ast::Local, c: &mut IrCompiler<'_>) -> Result<ir::Ir, IrError> {
    let span = ast.span();

    let name = loop {
        match &ast.pat {
            ast::Pat::PatIgnore(_) => {
                return expr(&ast.expr, c);
            }
            ast::Pat::PatPath(path) => {
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
            value: Box::new(expr(&ast.expr, c)?),
        },
    ))
}

fn condition(ast: &ast::Condition, c: &mut IrCompiler<'_>) -> Result<ir::IrCondition, IrError> {
    match ast {
        ast::Condition::Expr(e) => Ok(ir::IrCondition::Ir(expr(e, c)?)),
        ast::Condition::ExprLet(expr_let) => {
            let pat = ir::IrPat::compile_ast(&expr_let.pat, c)?;
            let ir = expr(&expr_let.expr, c)?;

            Ok(ir::IrCondition::Let(ir::IrLet {
                span: expr_let.span(),
                pat,
                ir,
            }))
        }
    }
}

fn expr_if(ast: &ast::ExprIf, c: &mut IrCompiler<'_>) -> Result<ir::IrBranches, IrError> {
    let mut branches = Vec::new();
    let mut default_branch = None;

    let cond = condition(&ast.condition, c)?;
    let ir = block(&ast.block, c)?;
    branches.push((cond, ir));

    for expr_else_if in &ast.expr_else_ifs {
        let cond = condition(&expr_else_if.condition, c)?;
        let ir = block(&expr_else_if.block, c)?;
        branches.push((cond, ir));
    }

    if let Some(expr_else) = &ast.expr_else {
        let ir = block(&expr_else.block, c)?;
        default_branch = Some(ir);
    }

    Ok(ir::IrBranches {
        branches,
        default_branch,
    })
}

fn expr_while(ast: &ast::ExprWhile, c: &mut IrCompiler<'_>) -> Result<ir::IrLoop, IrError> {
    Ok(ir::IrLoop {
        span: ast.span(),
        label: match &ast.label {
            Some((label, _)) => Some(c.resolve(label)?.into()),
            None => None,
        },
        condition: Some(Box::new(condition(&ast.condition, c)?)),
        body: block(&ast.body, c)?,
    })
}

fn expr_loop(ast: &ast::ExprLoop, c: &mut IrCompiler<'_>) -> Result<ir::IrLoop, IrError> {
    Ok(ir::IrLoop {
        span: ast.span(),
        label: match &ast.label {
            Some((label, _)) => Some(c.resolve(label)?.into()),
            None => None,
        },
        condition: None,
        body: block(&ast.body, c)?,
    })
}
