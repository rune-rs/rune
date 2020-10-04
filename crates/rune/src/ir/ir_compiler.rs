use crate::ir::ir;
use crate::ir::IrQuery;
use crate::parsing::Opaque;
use crate::{Resolve, Spanned, Storage};
use runestick::{ConstValue, Source};
use std::sync::Arc;

use crate::ast;
use crate::CompileError;

/// A compiler that compiles AST into Rune IR.
pub(crate) struct IrCompiler<'a> {
    pub(crate) storage: Storage,
    pub(crate) source: Arc<Source>,
    pub(crate) query: &'a mut dyn IrQuery,
}

impl<'a> IrCompiler<'a> {
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

pub(crate) trait IrCompile<T> {
    type Output;

    fn compile(&mut self, value: T) -> Result<Self::Output, CompileError>;
}

impl IrCompile<&ast::Expr> for IrCompiler<'_> {
    type Output = ir::Ir;

    fn compile(&mut self, expr: &ast::Expr) -> Result<Self::Output, CompileError> {
        Ok(match expr {
            ast::Expr::ExprGroup(expr_group) => self.compile(&*expr_group.expr)?,
            ast::Expr::ExprBinary(expr_binary) => self.compile(expr_binary)?,
            ast::Expr::ExprAssign(expr_assign) => self.compile(expr_assign)?,
            ast::Expr::ExprCall(expr_call) => ir::Ir::new(expr.span(), self.compile(expr_call)?),
            ast::Expr::ExprIf(expr_if) => ir::Ir::new(expr.span(), self.compile(expr_if)?),
            ast::Expr::ExprLoop(expr_loop) => ir::Ir::new(expr.span(), self.compile(expr_loop)?),
            ast::Expr::ExprWhile(expr_while) => ir::Ir::new(expr.span(), self.compile(expr_while)?),
            ast::Expr::ExprLit(expr_lit) => self.compile(expr_lit)?,
            ast::Expr::ExprBlock(expr_block) => {
                ir::Ir::new(expr.span(), self.compile(&expr_block.block)?)
            }
            ast::Expr::Path(path) => self.compile(path)?,
            ast::Expr::ExprFieldAccess(..) => ir::Ir::new(expr, self.ir_target(expr)?),
            ast::Expr::ExprBreak(expr_break) => ir::Ir::new(expr_break, self.compile(expr_break)?),
            ast::Expr::ExprLet(expr_let) => ir::Ir::new(expr_let, self.compile(expr_let)?),
            _ => return Err(CompileError::const_error(expr, "not supported yet")),
        })
    }
}

impl IrCompile<&ast::ItemFn> for IrCompiler<'_> {
    type Output = ir::IrFn;

    fn compile(&mut self, item_fn: &ast::ItemFn) -> Result<Self::Output, CompileError> {
        let mut args = Vec::new();

        for (arg, _) in &item_fn.args {
            match arg {
                ast::FnArg::Ident(ident) => {
                    args.push(self.resolve(ident)?.into());
                }
                _ => {
                    return Err(CompileError::const_error(
                        arg,
                        "unsupported argument in const fn",
                    ))
                }
            }
        }

        let ir_scope = self.compile(&item_fn.body)?;

        Ok(ir::IrFn {
            span: item_fn.span(),
            args,
            ir: ir::Ir::new(item_fn.span(), ir_scope),
        })
    }
}

impl IrCompile<&ast::ExprAssign> for IrCompiler<'_> {
    type Output = ir::Ir;

    fn compile(&mut self, expr_assign: &ast::ExprAssign) -> Result<Self::Output, CompileError> {
        let span = expr_assign.span();
        let target = self.ir_target(&*expr_assign.lhs)?;

        return Ok(ir::Ir::new(
            span,
            ir::IrSet {
                span,
                target,
                value: Box::new(self.compile(&*expr_assign.rhs)?),
            },
        ));
    }
}

impl IrCompile<&ast::ExprCall> for IrCompiler<'_> {
    type Output = ir::IrCall;

    fn compile(&mut self, expr_call: &ast::ExprCall) -> Result<Self::Output, CompileError> {
        let span = expr_call.span();

        let mut args = Vec::new();

        for (expr, _) in &expr_call.args {
            args.push(self.compile(expr)?);
        }

        match &*expr_call.expr {
            ast::Expr::Path(path) => {
                if let Some(ident) = path.try_as_ident() {
                    let target = self.resolve(ident)?;

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

impl IrCompile<&ast::ExprBinary> for IrCompiler<'_> {
    type Output = ir::Ir;

    fn compile(&mut self, expr_binary: &ast::ExprBinary) -> Result<Self::Output, CompileError> {
        let span = expr_binary.span();

        if expr_binary.op.is_assign() {
            let op = match expr_binary.op {
                ast::BinOp::AddAssign => ir::IrAssignOp::Add,
                ast::BinOp::SubAssign => ir::IrAssignOp::Sub,
                ast::BinOp::MulAssign => ir::IrAssignOp::Mul,
                ast::BinOp::DivAssign => ir::IrAssignOp::Div,
                ast::BinOp::ShlAssign => ir::IrAssignOp::Shl,
                ast::BinOp::ShrAssign => ir::IrAssignOp::Shr,
                _ => {
                    return Err(CompileError::const_error(
                        expr_binary.op_span(),
                        "op not supported yet",
                    ))
                }
            };

            let target = self.ir_target(&*expr_binary.lhs)?;

            return Ok(ir::Ir::new(
                span,
                ir::IrAssign {
                    span,
                    target,
                    value: Box::new(self.compile(&*expr_binary.rhs)?),
                    op,
                },
            ));
        }

        let lhs = self.compile(&*expr_binary.lhs)?;
        let rhs = self.compile(&*expr_binary.rhs)?;

        let op = match expr_binary.op {
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
                    expr_binary.op_span(),
                    "op not supported yet",
                ))
            }
        };

        Ok(ir::Ir::new(
            expr_binary.span(),
            ir::IrBinary {
                span,
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            },
        ))
    }
}

impl IrCompile<&ast::ExprLit> for IrCompiler<'_> {
    type Output = ir::Ir;

    fn compile(&mut self, expr_lit: &ast::ExprLit) -> Result<Self::Output, CompileError> {
        let span = expr_lit.span();

        Ok(match &expr_lit.lit {
            ast::Lit::Unit(..) => ir::Ir::new(span, ConstValue::Unit),
            ast::Lit::Bool(b) => ir::Ir::new(span, ConstValue::Bool(b.value)),
            ast::Lit::Str(s) => {
                let s = self.resolve(s)?;
                ir::Ir::new(span, ConstValue::String(s.as_ref().to_owned()))
            }
            ast::Lit::Number(n) => {
                let n = self.resolve(n)?;

                let const_value = match n {
                    ast::Number::Integer(n) => ConstValue::Integer(n),
                    ast::Number::Float(n) => ConstValue::Float(n),
                };

                ir::Ir::new(span, const_value)
            }
            ast::Lit::Template(lit_template) => ir::Ir::new(span, self.compile(lit_template)?),
            ast::Lit::Vec(lit_vec) => ir::Ir::new(span, self.compile(lit_vec)?),
            ast::Lit::Tuple(lit_tuple) => ir::Ir::new(span, self.compile(lit_tuple)?),
            ast::Lit::Byte(lit_byte) => ir::Ir::new(span, self.compile(lit_byte)?),
            ast::Lit::ByteStr(lit_byte_str) => ir::Ir::new(span, self.compile(lit_byte_str)?),
            ast::Lit::Char(lit_char) => ir::Ir::new(span, self.compile(lit_char)?),
            ast::Lit::Object(lit_object) => ir::Ir::new(span, self.compile(lit_object)?),
        })
    }
}

impl IrCompile<&ast::LitTuple> for IrCompiler<'_> {
    type Output = ir::IrTuple;

    fn compile(&mut self, lit_tuple: &ast::LitTuple) -> Result<Self::Output, CompileError> {
        let mut items = Vec::new();

        for (expr, _) in &lit_tuple.items {
            items.push(self.compile(expr)?);
        }

        Ok(ir::IrTuple {
            span: lit_tuple.span(),
            items: items.into_boxed_slice(),
        })
    }
}

impl IrCompile<&ast::LitVec> for IrCompiler<'_> {
    type Output = ir::IrVec;

    fn compile(&mut self, lit_vec: &ast::LitVec) -> Result<Self::Output, CompileError> {
        let mut items = Vec::new();

        for (expr, _) in &lit_vec.items {
            items.push(self.compile(expr)?);
        }

        Ok(ir::IrVec {
            span: lit_vec.span(),
            items: items.into_boxed_slice(),
        })
    }
}

impl IrCompile<&ast::LitObject> for IrCompiler<'_> {
    type Output = ir::IrObject;

    fn compile(&mut self, lit_object: &ast::LitObject) -> Result<Self::Output, CompileError> {
        let mut assignments = Vec::new();

        for (assign, _) in &lit_object.assignments {
            let key = self.resolve(&assign.key)?.into_owned();

            let ir = if let Some((_, expr)) = &assign.assign {
                self.compile(expr)?
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
            span: lit_object.span(),
            assignments: assignments.into_boxed_slice(),
        })
    }
}

impl IrCompile<&ast::LitByteStr> for IrCompiler<'_> {
    type Output = ConstValue;

    fn compile(&mut self, lit_byte_str: &ast::LitByteStr) -> Result<Self::Output, CompileError> {
        let byte_str = self.resolve(lit_byte_str)?;
        Ok(ConstValue::Bytes(byte_str.as_ref().to_vec()))
    }
}

impl IrCompile<&ast::LitByte> for IrCompiler<'_> {
    type Output = ConstValue;

    fn compile(&mut self, lit_byte: &ast::LitByte) -> Result<Self::Output, CompileError> {
        let b = self.resolve(lit_byte)?;
        Ok(ConstValue::Byte(b))
    }
}

impl IrCompile<&ast::LitChar> for IrCompiler<'_> {
    type Output = ConstValue;

    fn compile(&mut self, lit_char: &ast::LitChar) -> Result<Self::Output, CompileError> {
        let c = self.resolve(lit_char)?;
        Ok(ConstValue::Char(c))
    }
}

impl IrCompile<&ast::ExprBlock> for IrCompiler<'_> {
    type Output = ir::IrScope;

    fn compile(&mut self, expr_block: &ast::ExprBlock) -> Result<Self::Output, CompileError> {
        self.compile(&expr_block.block)
    }
}

impl IrCompile<&ast::Block> for IrCompiler<'_> {
    type Output = ir::IrScope;

    fn compile(&mut self, block: &ast::Block) -> Result<Self::Output, CompileError> {
        let span = block.span();

        let mut last = None::<(&ast::Expr, bool)>;
        let mut instructions = Vec::new();

        for stmt in &block.statements {
            let (expr, term) = match stmt {
                ast::Stmt::Local(local) => {
                    instructions.push(self.compile(local)?);
                    continue;
                }
                ast::Stmt::Expr(expr) => (expr, false),
                ast::Stmt::Semi(expr, _) => (expr, true),
                ast::Stmt::Item(..) => continue,
            };

            if let Some((expr, _)) = std::mem::replace(&mut last, Some((expr, term))) {
                instructions.push(self.compile(expr)?);
            }
        }

        let last = if let Some((expr, term)) = last {
            if term {
                instructions.push(self.compile(expr)?);
                None
            } else {
                Some(Box::new(self.compile(expr)?))
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

impl IrCompile<&ast::LitTemplate> for IrCompiler<'_> {
    type Output = ir::IrTemplate;

    fn compile(&mut self, lit_template: &ast::LitTemplate) -> Result<Self::Output, CompileError> {
        let span = lit_template.span();
        let mut components = Vec::new();

        let template = self
            .query
            .template_for(lit_template.span(), lit_template.id())?;

        for c in &template.components {
            match c {
                ast::TemplateComponent::String(string) => {
                    components.push(ir::IrTemplateComponent::String(string.clone().into()));
                }
                ast::TemplateComponent::Expr(expr) => {
                    let ir = self.compile(&**expr)?;
                    components.push(ir::IrTemplateComponent::Ir(ir));
                }
            }
        }

        Ok(ir::IrTemplate { span, components })
    }
}

impl IrCompile<&ast::Path> for IrCompiler<'_> {
    type Output = ir::Ir;

    fn compile(&mut self, path: &ast::Path) -> Result<Self::Output, CompileError> {
        let span = path.span();

        if let Some(name) = path.try_as_ident() {
            let name = self.resolve(name)?;
            return Ok(ir::Ir::new(span, <Box<str>>::from(name)));
        }

        Err(CompileError::const_error(span, "not supported yet"))
    }
}

impl IrCompile<&ast::ExprBreak> for IrCompiler<'_> {
    type Output = ir::IrBreak;

    fn compile(&mut self, expr_break: &ast::ExprBreak) -> Result<Self::Output, CompileError> {
        let span = expr_break.span();

        let kind = match &expr_break.expr {
            Some(expr) => match expr {
                ast::ExprBreakValue::Expr(expr) => {
                    ir::IrBreakKind::Ir(Box::new(self.compile(&**expr)?))
                }
                ast::ExprBreakValue::Label(label) => {
                    ir::IrBreakKind::Label(self.resolve(label)?.into())
                }
            },
            None => ir::IrBreakKind::Inherent,
        };

        Ok(ir::IrBreak { span, kind })
    }
}

impl IrCompile<&ast::ExprLet> for IrCompiler<'_> {
    type Output = ir::IrDecl;

    fn compile(&mut self, expr_let: &ast::ExprLet) -> Result<Self::Output, CompileError> {
        Err(CompileError::const_error(expr_let, "not supported yet"))
    }
}

impl IrCompile<&ast::Local> for IrCompiler<'_> {
    type Output = ir::Ir;

    fn compile(&mut self, local: &ast::Local) -> Result<Self::Output, CompileError> {
        let span = local.span();

        let name = loop {
            match &local.pat {
                ast::Pat::PatIgnore(_) => {
                    return self.compile(&*local.expr);
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
                name: self.resolve(name)?.into(),
                value: Box::new(self.compile(&*local.expr)?),
            },
        ))
    }
}

impl IrCompile<&ast::Condition> for IrCompiler<'_> {
    type Output = ir::IrCondition;

    fn compile(&mut self, condition: &ast::Condition) -> Result<Self::Output, CompileError> {
        match condition {
            ast::Condition::Expr(expr) => Ok(ir::IrCondition::Ir(self.compile(&**expr)?)),
            ast::Condition::ExprLet(expr_let) => {
                let pat = self.compile(&expr_let.pat)?;
                let ir = self.compile(&*expr_let.expr)?;

                Ok(ir::IrCondition::Let(ir::IrLet {
                    span: expr_let.span(),
                    pat,
                    ir,
                }))
            }
        }
    }
}

impl IrCompile<&ast::Pat> for IrCompiler<'_> {
    type Output = ir::IrPat;

    fn compile(&mut self, pat: &ast::Pat) -> Result<Self::Output, CompileError> {
        match pat {
            ast::Pat::PatIgnore(..) => return Ok(ir::IrPat::Ignore),
            ast::Pat::PatPath(path) => {
                if let Some(ident) = path.path.try_as_ident() {
                    let name = self.resolve(ident)?;
                    return Ok(ir::IrPat::Binding(name.into()));
                }
            }
            _ => (),
        }

        Err(CompileError::const_error(pat, "pattern not supported yet"))
    }
}

impl IrCompile<&ast::ExprIf> for IrCompiler<'_> {
    type Output = ir::IrBranches;

    fn compile(&mut self, expr_if: &ast::ExprIf) -> Result<Self::Output, CompileError> {
        let mut branches = Vec::new();
        let mut default_branch = None;

        let condition = self.compile(&expr_if.condition)?;
        let ir = self.compile(&*expr_if.block)?;
        branches.push((condition, ir));

        for expr_else_if in &expr_if.expr_else_ifs {
            let condition = self.compile(&expr_else_if.condition)?;
            let ir = self.compile(&*expr_else_if.block)?;
            branches.push((condition, ir));
        }

        if let Some(expr_else) = &expr_if.expr_else {
            let ir = self.compile(&*expr_else.block)?;
            default_branch = Some(ir);
        }

        Ok(ir::IrBranches {
            branches,
            default_branch,
        })
    }
}

impl IrCompile<&ast::ExprWhile> for IrCompiler<'_> {
    type Output = ir::IrLoop;

    fn compile(&mut self, expr_while: &ast::ExprWhile) -> Result<Self::Output, CompileError> {
        Ok(ir::IrLoop {
            span: expr_while.span(),
            label: match &expr_while.label {
                Some((label, _)) => Some(self.resolve(label)?.into()),
                None => None,
            },
            condition: Some(Box::new(self.compile(&expr_while.condition)?)),
            body: self.compile(&*expr_while.body)?,
        })
    }
}

impl IrCompile<&ast::ExprLoop> for IrCompiler<'_> {
    type Output = ir::IrLoop;

    fn compile(&mut self, expr_loop: &ast::ExprLoop) -> Result<Self::Output, CompileError> {
        Ok(ir::IrLoop {
            span: expr_loop.span(),
            label: match &expr_loop.label {
                Some((label, _)) => Some(self.resolve(label)?.into()),
                None => None,
            },
            condition: None,
            body: self.compile(&*expr_loop.body)?,
        })
    }
}
