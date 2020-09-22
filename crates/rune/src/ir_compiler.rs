use crate::{Resolve, Spanned, Storage};
use rune_ir::{
    Ir, IrBinary, IrBinaryOp, IrBranches, IrBreak, IrBreakKind, IrDecl, IrLoop, IrScope, IrSet,
    IrTemplate, IrTemplateComponent, IrTuple, IrVec,
};
use runestick::{ConstValue, Source, Span};

use crate::ast;
use crate::CompileError;

/// A compiler that compiles AST into Rune IR.
pub(crate) struct IrCompiler<'a> {
    pub(crate) storage: &'a Storage,
    pub(crate) source: &'a Source,
}

impl<'a> IrCompiler<'a> {
    /// Resolve the given resolvable value.
    pub(crate) fn resolve<T>(&self, value: &T) -> Result<T::Output, CompileError>
    where
        T: Resolve<'a>,
    {
        Ok(value.resolve(&self.storage, self.source)?)
    }
}

pub(crate) trait Compile<T> {
    type Output;

    fn compile(&mut self, value: T) -> Result<Self::Output, CompileError>;
}

impl Compile<&ast::Expr> for IrCompiler<'_> {
    type Output = Ir;

    fn compile(&mut self, expr: &ast::Expr) -> Result<Self::Output, CompileError> {
        Ok(match expr {
            ast::Expr::ExprBinary(expr_binary) => self.compile(expr_binary)?,
            ast::Expr::ExprIf(expr_if) => Ir::new(expr.span(), self.compile(expr_if)?),
            ast::Expr::ExprLoop(expr_loop) => Ir::new(expr.span(), self.compile(expr_loop)?),
            ast::Expr::ExprWhile(expr_while) => Ir::new(expr.span(), self.compile(expr_while)?),
            ast::Expr::ExprLit(expr_lit) => self.compile(expr_lit)?,
            ast::Expr::ExprBlock(expr_block) => {
                Ir::new(expr.span(), self.compile(&expr_block.block)?)
            }
            ast::Expr::Path(path) => self.compile(path)?,
            ast::Expr::ExprBreak(expr_break) => Ir::new(expr.span(), self.compile(expr_break)?),
            ast::Expr::ExprLet(expr_let) => {
                let decl = match self.compile(expr_let)? {
                    Some(decl) => decl,
                    None => return Ok(Ir::new(expr_let.span(), ConstValue::Unit)),
                };

                Ir::new(expr.span(), decl)
            }
            _ => return Err(CompileError::const_error(expr, "not supported yet")),
        })
    }
}

impl Compile<&ast::ExprBinary> for IrCompiler<'_> {
    type Output = Ir;

    fn compile(&mut self, expr_binary: &ast::ExprBinary) -> Result<Self::Output, CompileError> {
        let span = expr_binary.span();

        if expr_binary.op.is_assign() {
            match expr_binary.op {
                ast::BinOp::Assign => match &*expr_binary.lhs {
                    ast::Expr::Path(path) => {
                        if let Some(ident) = path.try_as_ident() {
                            let name = self.resolve(ident)?;
                            let value = self.compile(&*expr_binary.rhs)?;

                            return Ok(Ir::new(
                                span,
                                IrSet {
                                    span,
                                    name: name.into(),
                                    value: Box::new(value),
                                },
                            ));
                        }
                    }
                    _ => (),
                },
                _ => (),
            }

            return Err(CompileError::const_error(
                expr_binary.op_span(),
                "op not supported yet",
            ));
        }

        let lhs = self.compile(&*expr_binary.lhs)?;
        let rhs = self.compile(&*expr_binary.rhs)?;

        let op = match expr_binary.op {
            ast::BinOp::Add => IrBinaryOp::Add,
            ast::BinOp::Sub => IrBinaryOp::Sub,
            ast::BinOp::Mul => IrBinaryOp::Mul,
            ast::BinOp::Div => IrBinaryOp::Div,
            ast::BinOp::Shl => IrBinaryOp::Shl,
            ast::BinOp::Shr => IrBinaryOp::Shr,
            ast::BinOp::Lt => IrBinaryOp::Lt,
            ast::BinOp::Lte => IrBinaryOp::Lte,
            ast::BinOp::Eq => IrBinaryOp::Eq,
            ast::BinOp::Gt => IrBinaryOp::Gt,
            ast::BinOp::Gte => IrBinaryOp::Gte,
            _ => {
                return Err(CompileError::const_error(
                    expr_binary.op_span(),
                    "op not supported yet",
                ))
            }
        };

        Ok(Ir::new(
            expr_binary.span(),
            IrBinary {
                span,
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            },
        ))
    }
}

impl Compile<&ast::ExprLit> for IrCompiler<'_> {
    type Output = Ir;

    fn compile(&mut self, expr_lit: &ast::ExprLit) -> Result<Self::Output, CompileError> {
        let span = expr_lit.span();

        match &expr_lit.lit {
            ast::Lit::Unit(..) => Ok(Ir::new(span, ConstValue::Unit)),
            ast::Lit::Bool(b) => Ok(Ir::new(span, ConstValue::Bool(b.value))),
            ast::Lit::Str(s) => {
                let s = self.resolve(s)?;
                Ok(Ir::new(span, ConstValue::String(s.into())))
            }
            ast::Lit::Number(n) => {
                let n = self.resolve(n)?;

                let const_value = match n {
                    ast::Number::Integer(n) => ConstValue::Integer(n),
                    ast::Number::Float(n) => ConstValue::Float(n),
                };

                Ok(Ir::new(span, const_value))
            }
            ast::Lit::Template(lit_template) => Ok(Ir::new(span, self.compile(lit_template)?)),
            ast::Lit::Vec(lit_vec) => Ok(Ir::new(span, self.compile(lit_vec)?)),
            ast::Lit::Tuple(lit_tuple) => Ok(Ir::new(span, self.compile(lit_tuple)?)),
            _ => Err(CompileError::const_error(span, "not supported yet")),
        }
    }
}

impl Compile<&ast::LitTuple> for IrCompiler<'_> {
    type Output = IrTuple;

    fn compile(&mut self, lit_tuple: &ast::LitTuple) -> Result<Self::Output, CompileError> {
        let mut items = Vec::new();

        for (expr, _) in &lit_tuple.items {
            items.push(self.compile(expr)?);
        }

        Ok(IrTuple {
            span: lit_tuple.span(),
            items: items.into_boxed_slice(),
        })
    }
}

impl Compile<&ast::LitVec> for IrCompiler<'_> {
    type Output = IrVec;

    fn compile(&mut self, lit_vec: &ast::LitVec) -> Result<Self::Output, CompileError> {
        let mut items = Vec::new();

        for (expr, _) in &lit_vec.items {
            items.push(self.compile(expr)?);
        }

        Ok(IrVec {
            span: lit_vec.span(),
            items: items.into_boxed_slice(),
        })
    }
}

impl Compile<&ast::ExprBlock> for IrCompiler<'_> {
    type Output = IrScope;

    fn compile(&mut self, expr_block: &ast::ExprBlock) -> Result<Self::Output, CompileError> {
        self.compile(&expr_block.block)
    }
}

impl Compile<&ast::Block> for IrCompiler<'_> {
    type Output = IrScope;

    fn compile(&mut self, block: &ast::Block) -> Result<Self::Output, CompileError> {
        let span = block.span();

        let mut last = None::<(&ast::Expr, bool)>;
        let mut instructions = Vec::new();

        for stmt in &block.statements {
            let (expr, term) = match stmt {
                ast::Stmt::Expr(expr) => (expr, false),
                ast::Stmt::Semi(expr, _) => (expr, true),
                _ => continue,
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

        Ok(IrScope {
            span,
            instructions,
            last,
        })
    }
}

impl Compile<&ast::LitTemplate> for IrCompiler<'_> {
    type Output = IrTemplate;

    fn compile(&mut self, lit_template: &ast::LitTemplate) -> Result<Self::Output, CompileError> {
        let span = lit_template.span();
        let mut components = Vec::new();

        let template = self.resolve(lit_template)?;

        for c in template.components {
            match c {
                ast::TemplateComponent::String(string) => {
                    components.push(IrTemplateComponent::String(string.into()));
                }
                ast::TemplateComponent::Expr(expr) => {
                    let ir = self.compile(&*expr)?;
                    components.push(IrTemplateComponent::Ir(ir));
                }
            }
        }

        Ok(IrTemplate { span, components })
    }
}

impl Compile<&ast::Path> for IrCompiler<'_> {
    type Output = Ir;

    fn compile(&mut self, path: &ast::Path) -> Result<Self::Output, CompileError> {
        let span = path.span();

        if let Some(name) = path.try_as_ident() {
            let name = self.resolve(name)?;
            return Ok(Ir::new(span, <Box<str>>::from(name)));
        }

        Err(CompileError::const_error(span, "not supported yet"))
    }
}

impl Compile<&ast::ExprBreak> for IrCompiler<'_> {
    type Output = IrBreak;

    fn compile(&mut self, expr_break: &ast::ExprBreak) -> Result<Self::Output, CompileError> {
        let span = expr_break.span();

        let kind = match &expr_break.expr {
            Some(expr) => match expr {
                ast::ExprBreakValue::Expr(expr) => {
                    IrBreakKind::Ir(Box::new(self.compile(&**expr)?))
                }
                ast::ExprBreakValue::Label(label) => {
                    IrBreakKind::Label(self.resolve(label)?.into())
                }
            },
            None => IrBreakKind::Inherent,
        };

        Ok(IrBreak { span, kind })
    }
}

impl Compile<&ast::ExprLet> for IrCompiler<'_> {
    type Output = Option<IrDecl>;

    fn compile(&mut self, expr_let: &ast::ExprLet) -> Result<Self::Output, CompileError> {
        let span = expr_let.span();

        let name = loop {
            match &expr_let.pat {
                ast::Pat::PatIgnore(_) => {
                    return Ok(None);
                }
                ast::Pat::PatPath(path) => match path.path.try_as_ident() {
                    Some(ident) => break ident,
                    None => (),
                },
                _ => (),
            }

            return Err(CompileError::const_error(span, "not supported yet"));
        };

        Ok(Some(IrDecl {
            span,
            name: self.resolve(name)?.into(),
            value: Box::new(self.compile(&*expr_let.expr)?),
        }))
    }
}

impl Compile<&ast::Condition> for IrCompiler<'_> {
    type Output = Ir;

    fn compile(&mut self, condition: &ast::Condition) -> Result<Self::Output, CompileError> {
        match condition {
            ast::Condition::Expr(expr) => {
                return self.compile(&**expr);
            }
            _ => (),
        }

        Err(CompileError::const_error(condition, "not supported yet"))
    }
}

impl Compile<&ast::ExprIf> for IrCompiler<'_> {
    type Output = IrBranches;

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

        Ok(IrBranches {
            branches,
            default_branch,
        })
    }
}

impl Compile<&ast::ExprWhile> for IrCompiler<'_> {
    type Output = IrLoop;

    fn compile(&mut self, expr_while: &ast::ExprWhile) -> Result<Self::Output, CompileError> {
        Ok(IrLoop {
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

impl Compile<&ast::ExprLoop> for IrCompiler<'_> {
    type Output = IrLoop;

    fn compile(&mut self, expr_loop: &ast::ExprLoop) -> Result<Self::Output, CompileError> {
        Ok(IrLoop {
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

macro_rules! impl_spanned {
    ($($ty:ty),* $(,)?) => {
        $(impl Spanned for $ty {
            fn span(&self) -> Span {
                self.span
            }
        })*
    };
}

impl_spanned! {
    rune_ir::Ir,
    rune_ir::IrScope,
    rune_ir::IrSet,
    rune_ir::IrDecl,
    rune_ir::IrBinary,
    rune_ir::IrTemplate,
    rune_ir::IrBreak,
    rune_ir::IrLoop,
}
