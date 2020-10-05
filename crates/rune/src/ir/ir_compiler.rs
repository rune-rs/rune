use crate::ir::ir;
use crate::ir::IrQuery;
use crate::parsing::Opaque;
use crate::{Resolve, Spanned, Storage};
use runestick::{ConstValue, Source};
use std::rc::Rc;
use std::sync::Arc;

use crate::ast;
use crate::CompileError;

/// A compiler that compiles AST into Rune IR.
pub struct IrCompiler<'a> {
    pub(crate) storage: Storage,
    pub(crate) source: Arc<Source>,
    pub(crate) query: &'a mut dyn IrQuery,
}

impl<'a> IrCompiler<'a> {
    /// Compile the given target.
    pub(crate) fn compile<T>(&mut self, target: &T) -> Result<T::Output, CompileError>
    where
        T: IrCompile,
    {
        target.compile(self)
    }

    /// Resolve the given resolvable value.
    pub(crate) fn resolve<'s, T>(&'s self, value: &T) -> Result<T::Output, CompileError>
    where
        T: Resolve<'s>,
    {
        Ok(value.resolve(&self.storage, &*self.source)?)
    }

    /// Resolve an ir target from an expression.
    fn ir_target(&self, expr: &ast::Expr) -> Result<ir::IrTarget, CompileError> {
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
            ast::Expr::ExprFieldAccess(expr_field_access) => {
                let target = self.ir_target(&expr_field_access.expr)?;

                match &expr_field_access.expr_field {
                    ast::ExprField::Ident(field) => {
                        let field = self.resolve(field)?;

                        return Ok(ir::IrTarget {
                            span: expr.span(),
                            kind: ir::IrTargetKind::Field(Box::new(target), field.into()),
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

        Err(CompileError::const_error(expr, "not supported as a target"))
    }
}

/// The trait for a type that can be compiled into intermediate representation.
pub trait IrCompile {
    type Output;

    fn compile(&self, compiler: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError>;
}

impl IrCompile for ast::Expr {
    type Output = ir::Ir;

    fn compile(&self, compiler: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError> {
        Ok(match self {
            ast::Expr::ExprGroup(expr_group) => expr_group.expr.compile(compiler)?,
            ast::Expr::ExprBinary(expr_binary) => expr_binary.compile(compiler)?,
            ast::Expr::ExprAssign(expr_assign) => expr_assign.compile(compiler)?,
            ast::Expr::ExprCall(expr_call) => {
                ir::Ir::new(self.span(), expr_call.compile(compiler)?)
            }
            ast::Expr::ExprIf(expr_if) => ir::Ir::new(self.span(), expr_if.compile(compiler)?),
            ast::Expr::ExprLoop(expr_loop) => {
                ir::Ir::new(self.span(), expr_loop.compile(compiler)?)
            }
            ast::Expr::ExprWhile(expr_while) => {
                ir::Ir::new(self.span(), expr_while.compile(compiler)?)
            }
            ast::Expr::ExprLit(expr_lit) => expr_lit.compile(compiler)?,
            ast::Expr::ExprBlock(expr_block) => {
                ir::Ir::new(self.span(), expr_block.block.compile(compiler)?)
            }
            ast::Expr::Path(path) => path.compile(compiler)?,
            ast::Expr::ExprFieldAccess(..) => ir::Ir::new(self.span(), compiler.ir_target(self)?),
            ast::Expr::ExprBreak(expr_break) => {
                ir::Ir::new(expr_break, expr_break.compile(compiler)?)
            }
            ast::Expr::ExprLet(expr_let) => ir::Ir::new(expr_let, expr_let.compile(compiler)?),
            _ => return Err(CompileError::const_error(self, "not supported yet")),
        })
    }
}

impl IrCompile for ast::ItemFn {
    type Output = ir::IrFn;

    fn compile(&self, compiler: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError> {
        let mut args = Vec::new();

        for (arg, _) in &self.args {
            match arg {
                ast::FnArg::Ident(ident) => {
                    args.push(compiler.resolve(ident)?.into());
                }
                _ => {
                    return Err(CompileError::const_error(
                        arg,
                        "unsupported argument in const fn",
                    ))
                }
            }
        }

        let ir_scope = self.body.compile(compiler)?;

        Ok(ir::IrFn {
            span: self.span(),
            args,
            ir: ir::Ir::new(self.span(), ir_scope),
        })
    }
}

impl IrCompile for ast::ExprAssign {
    type Output = ir::Ir;

    fn compile(&self, compiler: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError> {
        let span = self.span();
        let target = compiler.ir_target(&*self.lhs)?;

        return Ok(ir::Ir::new(
            span,
            ir::IrSet {
                span,
                target,
                value: Box::new(self.rhs.compile(compiler)?),
            },
        ));
    }
}

impl IrCompile for ast::ExprCall {
    type Output = ir::IrCall;

    fn compile(&self, compiler: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError> {
        let span = self.span();

        let mut args = Vec::new();

        for (expr, _) in &self.args {
            args.push(expr.compile(compiler)?);
        }

        match &*self.expr {
            ast::Expr::Path(path) => {
                if let Some(ident) = path.try_as_ident() {
                    let target = compiler.resolve(ident)?;

                    return Ok(ir::IrCall {
                        span,
                        target: target.into(),
                        args,
                    });
                }
            }
            _ => (),
        }

        Err(CompileError::const_error(span, "call not supported"))
    }
}

impl IrCompile for ast::ExprBinary {
    type Output = ir::Ir;

    fn compile(&self, compiler: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError> {
        let span = self.span();

        if self.op.is_assign() {
            let op = match self.op {
                ast::BinOp::AddAssign => ir::IrAssignOp::Add,
                ast::BinOp::SubAssign => ir::IrAssignOp::Sub,
                ast::BinOp::MulAssign => ir::IrAssignOp::Mul,
                ast::BinOp::DivAssign => ir::IrAssignOp::Div,
                ast::BinOp::ShlAssign => ir::IrAssignOp::Shl,
                ast::BinOp::ShrAssign => ir::IrAssignOp::Shr,
                _ => {
                    return Err(CompileError::const_error(
                        self.op_span(),
                        "op not supported yet",
                    ))
                }
            };

            let target = compiler.ir_target(&*self.lhs)?;

            return Ok(ir::Ir::new(
                span,
                ir::IrAssign {
                    span,
                    target,
                    value: Box::new(self.rhs.compile(compiler)?),
                    op,
                },
            ));
        }

        let lhs = self.lhs.compile(compiler)?;
        let rhs = self.rhs.compile(compiler)?;

        let op = match self.op {
            ast::BinOp::Add => ir::IrBinaryOp::Add,
            ast::BinOp::Sub => ir::IrBinaryOp::Sub,
            ast::BinOp::Mul => ir::IrBinaryOp::Mul,
            ast::BinOp::Div => ir::IrBinaryOp::Div,
            ast::BinOp::Shl => ir::IrBinaryOp::Shl,
            ast::BinOp::Shr => ir::IrBinaryOp::Shr,
            ast::BinOp::Lt => ir::IrBinaryOp::Lt,
            ast::BinOp::Lte => ir::IrBinaryOp::Lte,
            ast::BinOp::Eq => ir::IrBinaryOp::Eq,
            ast::BinOp::Gt => ir::IrBinaryOp::Gt,
            ast::BinOp::Gte => ir::IrBinaryOp::Gte,
            _ => {
                return Err(CompileError::const_error(
                    self.op_span(),
                    "op not supported yet",
                ))
            }
        };

        Ok(ir::Ir::new(
            self.span(),
            ir::IrBinary {
                span,
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            },
        ))
    }
}

impl IrCompile for ast::ExprLit {
    type Output = ir::Ir;

    fn compile(&self, compiler: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError> {
        let span = self.span();

        Ok(match &self.lit {
            ast::Lit::Unit(..) => ir::Ir::new(span, ConstValue::Unit),
            ast::Lit::Bool(b) => ir::Ir::new(span, ConstValue::Bool(b.value)),
            ast::Lit::Str(s) => {
                let s = compiler.resolve(s)?;
                ir::Ir::new(span, ConstValue::String(s.as_ref().to_owned()))
            }
            ast::Lit::Number(n) => {
                let n = compiler.resolve(n)?;

                let const_value = match n {
                    ast::Number::Integer(n) => ConstValue::Integer(n),
                    ast::Number::Float(n) => ConstValue::Float(n),
                };

                ir::Ir::new(span, const_value)
            }
            ast::Lit::Template(lit_template) => ir::Ir::new(span, lit_template.compile(compiler)?),
            ast::Lit::Vec(lit_vec) => ir::Ir::new(span, lit_vec.compile(compiler)?),
            ast::Lit::Tuple(lit_tuple) => ir::Ir::new(span, lit_tuple.compile(compiler)?),
            ast::Lit::Byte(lit_byte) => ir::Ir::new(span, lit_byte.compile(compiler)?),
            ast::Lit::ByteStr(lit_byte_str) => ir::Ir::new(span, lit_byte_str.compile(compiler)?),
            ast::Lit::Char(lit_char) => ir::Ir::new(span, lit_char.compile(compiler)?),
            ast::Lit::Object(lit_object) => ir::Ir::new(span, lit_object.compile(compiler)?),
        })
    }
}

impl IrCompile for ast::LitTuple {
    type Output = ir::IrTuple;

    fn compile(&self, compiler: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError> {
        let mut items = Vec::new();

        for (expr, _) in &self.items {
            items.push(expr.compile(compiler)?);
        }

        Ok(ir::IrTuple {
            span: self.span(),
            items: items.into_boxed_slice(),
        })
    }
}

impl IrCompile for ast::LitVec {
    type Output = ir::IrVec;

    fn compile(&self, compiler: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError> {
        let mut items = Vec::new();

        for (expr, _) in &self.items {
            items.push(expr.compile(compiler)?);
        }

        Ok(ir::IrVec {
            span: self.span(),
            items: items.into_boxed_slice(),
        })
    }
}

impl IrCompile for ast::LitObject {
    type Output = ir::IrObject;

    fn compile(&self, compiler: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError> {
        let mut assignments = Vec::new();

        for (assign, _) in &self.assignments {
            let key = compiler.resolve(&assign.key)?.into_owned();

            let ir = if let Some((_, expr)) = &assign.assign {
                expr.compile(compiler)?
            } else {
                ir::Ir::new(
                    assign,
                    ir::IrKind::Target(ir::IrTarget {
                        span: assign.span(),
                        kind: ir::IrTargetKind::Name(key.clone().into()),
                    }),
                )
            };

            assignments.push((key.into(), ir))
        }

        Ok(ir::IrObject {
            span: self.span(),
            assignments: assignments.into_boxed_slice(),
        })
    }
}

impl IrCompile for ast::LitByteStr {
    type Output = ConstValue;

    fn compile(&self, compiler: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError> {
        let byte_str = compiler.resolve(self)?;
        Ok(ConstValue::Bytes(byte_str.as_ref().to_vec()))
    }
}

impl IrCompile for ast::LitByte {
    type Output = ConstValue;

    fn compile(&self, compiler: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError> {
        let b = compiler.resolve(self)?;
        Ok(ConstValue::Byte(b))
    }
}

impl IrCompile for ast::LitChar {
    type Output = ConstValue;

    fn compile(&self, compiler: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError> {
        let c = compiler.resolve(self)?;
        Ok(ConstValue::Char(c))
    }
}

impl IrCompile for ast::ExprBlock {
    type Output = ir::IrScope;

    fn compile(&self, compiler: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError> {
        self.block.compile(compiler)
    }
}

impl IrCompile for ast::Block {
    type Output = ir::IrScope;

    fn compile(&self, compiler: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError> {
        let span = self.span();

        let mut last = None::<(&ast::Expr, bool)>;
        let mut instructions = Vec::new();

        for stmt in &self.statements {
            let (expr, term) = match stmt {
                ast::Stmt::Local(local) => {
                    instructions.push(local.compile(compiler)?);
                    continue;
                }
                ast::Stmt::Expr(expr) => (expr, false),
                ast::Stmt::Semi(expr, _) => (expr, true),
                ast::Stmt::Item(..) => continue,
            };

            if let Some((expr, _)) = std::mem::replace(&mut last, Some((expr, term))) {
                instructions.push(expr.compile(compiler)?);
            }
        }

        let last = if let Some((expr, term)) = last {
            if term {
                instructions.push(expr.compile(compiler)?);
                None
            } else {
                Some(Box::new(expr.compile(compiler)?))
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
}

impl IrCompile for ast::LitTemplate {
    type Output = ir::IrTemplate;

    fn compile(&self, compiler: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError> {
        let span = self.span();
        let mut components = Vec::new();

        let template = match self.id() {
            Some(id) => compiler.query.template_for(self.span(), Some(id))?,
            None => Rc::new(compiler.resolve(self)?),
        };

        for c in &template.components {
            match c {
                ast::TemplateComponent::String(string) => {
                    components.push(ir::IrTemplateComponent::String(string.clone().into()));
                }
                ast::TemplateComponent::Expr(expr) => {
                    let ir = expr.compile(compiler)?;
                    components.push(ir::IrTemplateComponent::Ir(ir));
                }
            }
        }

        Ok(ir::IrTemplate { span, components })
    }
}

impl IrCompile for ast::Path {
    type Output = ir::Ir;

    fn compile(&self, compiler: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError> {
        let span = self.span();

        if let Some(name) = self.try_as_ident() {
            let name = compiler.resolve(name)?;
            return Ok(ir::Ir::new(span, <Box<str>>::from(name)));
        }

        Err(CompileError::const_error(span, "not supported yet"))
    }
}

impl IrCompile for ast::ExprBreak {
    type Output = ir::IrBreak;

    fn compile(&self, compiler: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError> {
        let span = self.span();

        let kind = match &self.expr {
            Some(expr) => match expr {
                ast::ExprBreakValue::Expr(expr) => {
                    ir::IrBreakKind::Ir(Box::new(expr.compile(compiler)?))
                }
                ast::ExprBreakValue::Label(label) => {
                    ir::IrBreakKind::Label(compiler.resolve(label)?.into())
                }
            },
            None => ir::IrBreakKind::Inherent,
        };

        Ok(ir::IrBreak { span, kind })
    }
}

impl IrCompile for ast::ExprLet {
    type Output = ir::IrDecl;

    fn compile(&self, _: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError> {
        Err(CompileError::const_error(self, "not supported yet"))
    }
}

impl IrCompile for ast::Local {
    type Output = ir::Ir;

    fn compile(&self, compiler: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError> {
        let span = self.span();

        let name = loop {
            match &self.pat {
                ast::Pat::PatIgnore(_) => {
                    return self.expr.compile(compiler);
                }
                ast::Pat::PatPath(path) => match path.path.try_as_ident() {
                    Some(ident) => break ident,
                    None => (),
                },
                _ => (),
            }

            return Err(CompileError::const_error(span, "not supported yet"));
        };

        Ok(ir::Ir::new(
            span,
            ir::IrDecl {
                span,
                name: compiler.resolve(name)?.into(),
                value: Box::new(self.expr.compile(compiler)?),
            },
        ))
    }
}

impl IrCompile for ast::Condition {
    type Output = ir::IrCondition;

    fn compile(&self, compiler: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError> {
        match self {
            ast::Condition::Expr(expr) => Ok(ir::IrCondition::Ir(expr.compile(compiler)?)),
            ast::Condition::ExprLet(expr_let) => {
                let pat = expr_let.pat.compile(compiler)?;
                let ir = expr_let.expr.compile(compiler)?;

                Ok(ir::IrCondition::Let(ir::IrLet {
                    span: expr_let.span(),
                    pat,
                    ir,
                }))
            }
        }
    }
}

impl IrCompile for ast::Pat {
    type Output = ir::IrPat;

    fn compile(&self, compiler: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError> {
        match self {
            ast::Pat::PatIgnore(..) => return Ok(ir::IrPat::Ignore),
            ast::Pat::PatPath(path) => {
                if let Some(ident) = path.path.try_as_ident() {
                    let name = compiler.resolve(ident)?;
                    return Ok(ir::IrPat::Binding(name.into()));
                }
            }
            _ => (),
        }

        Err(CompileError::const_error(self, "pattern not supported yet"))
    }
}

impl IrCompile for ast::ExprIf {
    type Output = ir::IrBranches;

    fn compile(&self, compiler: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError> {
        let mut branches = Vec::new();
        let mut default_branch = None;

        let condition = self.condition.compile(compiler)?;
        let ir = self.block.compile(compiler)?;
        branches.push((condition, ir));

        for expr_else_if in &self.expr_else_ifs {
            let condition = expr_else_if.condition.compile(compiler)?;
            let ir = expr_else_if.block.compile(compiler)?;
            branches.push((condition, ir));
        }

        if let Some(expr_else) = &self.expr_else {
            let ir = expr_else.block.compile(compiler)?;
            default_branch = Some(ir);
        }

        Ok(ir::IrBranches {
            branches,
            default_branch,
        })
    }
}

impl IrCompile for ast::ExprWhile {
    type Output = ir::IrLoop;

    fn compile(&self, compiler: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError> {
        Ok(ir::IrLoop {
            span: self.span(),
            label: match &self.label {
                Some((label, _)) => Some(compiler.resolve(label)?.into()),
                None => None,
            },
            condition: Some(Box::new(self.condition.compile(compiler)?)),
            body: self.body.compile(compiler)?,
        })
    }
}

impl IrCompile for &ast::ExprLoop {
    type Output = ir::IrLoop;

    fn compile(&self, compiler: &mut IrCompiler<'_>) -> Result<Self::Output, CompileError> {
        Ok(ir::IrLoop {
            span: self.span(),
            label: match &self.label {
                Some((label, _)) => Some(compiler.resolve(label)?.into()),
                None => None,
            },
            condition: None,
            body: self.body.compile(compiler)?,
        })
    }
}
