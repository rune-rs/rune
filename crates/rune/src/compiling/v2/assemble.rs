use crate::ast;
use crate::compiling::v2::{Branches, Compiler};
use crate::compiling::{CompileError, CompileErrorKind, CompileResult};
use crate::parsing::ParseErrorKind;
use crate::shared::ResultExt as _;
use crate::spanned::Spanned as _;
use rune_ssa::{Block, Constant, Var};

/// Assemble a variable.
pub(crate) trait Assemble {
    /// Walk the current type with the given item.
    fn assemble(&self, c: &mut Compiler<'_>, block: Block) -> CompileResult<(Block, Var)>;
}

/// Assemble a function.
pub(crate) trait AssembleFn {
    /// Assemble a function.
    fn assemble_fn(&self, c: &mut Compiler<'_>, instance_fn: bool) -> CompileResult<()>;
}

impl AssembleFn for ast::ItemFn {
    fn assemble_fn(&self, c: &mut Compiler<'_>, instance_fn: bool) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ItemFn => {:?}", c.source.source(span));

        let name = c.resolve(&self.name)?;
        let block = c.program.named(&name);
        let mut first = true;

        for (arg, _) in &self.args {
            let span = arg.span();
            let value = block.input().with_span(span)?;
            let first = std::mem::take(&mut first);

            match arg {
                ast::FnArg::SelfValue(s) => {
                    let span = s.span();

                    if !instance_fn || !first {
                        return Err(CompileError::new(span, CompileErrorKind::UnsupportedSelf));
                    }

                    c.scope.declare(span, "self", value)?;
                    continue;
                }
                ast::FnArg::Pat(ast::Pat::PatPath(path)) => {
                    if let Some(ident) = path.path.try_as_ident() {
                        let name = c.resolve(ident)?;
                        c.scope.declare(span, &name, value)?;
                        continue;
                    } else {
                        return Err(CompileError::msg(span, "path not supported yet"));
                    }
                }
                _ => {
                    return Err(CompileError::msg(span, "argument not supported yet"));
                }
            }
        }

        let (value_block, value) = self.body.assemble(c, block)?;
        value_block.return_(value).with_span(span)?;
        value_block.seal().with_span(span)?;
        Ok(())
    }
}

/// Assembler for a block.
impl Assemble for ast::Block {
    fn assemble(&self, c: &mut Compiler<'_>, mut block: Block) -> CompileResult<(Block, Var)> {
        let span = self.span();
        log::trace!("Block => {:?}", c.source.source(span));

        c.contexts.push(span);
        c.scope.push();

        let mut last = None;

        for stmt in &self.statements {
            let (next, semi) = match stmt {
                ast::Stmt::Local(local) => {
                    let (next, _) = local.assemble(c, block)?;
                    block = next;
                    continue;
                }
                ast::Stmt::Expr(expr, semi) => {
                    let (next, value) = expr.assemble(c, block)?;
                    block = next;
                    (value, semi)
                }
                ast::Stmt::Item(..) => continue,
            };

            // NB: semi-colons were checked during parsing.
            if semi.is_none() {
                last = Some(next);
            }
        }

        c.scope.pop(span)?;

        let value = match last {
            Some(last) => last,
            None => block.unit().with_span(span)?,
        };

        Ok((block, value))
    }
}

impl Assemble for ast::Local {
    fn assemble(&self, c: &mut Compiler<'_>, block: Block) -> CompileResult<(Block, Var)> {
        let span = self.span();
        log::trace!("Local => {:?}", c.source.source(span));

        match &self.pat {
            ast::Pat::PatPath(path) => {
                if let Some(name) = path.path.try_as_ident() {
                    let name = c.resolve(name)?;
                    let (block, value) = self.expr.assemble(c, block)?;
                    c.scope.declare(span, &name, value)?;
                    let value = block.unit().with_span(span)?;
                    return Ok((block, value));
                }
            }
            _ => (),
        }

        Err(CompileError::msg(span, "unsupported assignment"))
    }
}

/// Assembler for a block.
impl Assemble for ast::Expr {
    fn assemble(&self, c: &mut Compiler<'_>, block: Block) -> CompileResult<(Block, Var)> {
        let span = self.span();
        log::trace!("Expr => {:?}", c.source.source(span));

        match self {
            ast::Expr::Lit(expr) => expr.assemble(c, block),
            ast::Expr::Unary(expr) => expr.assemble(c, block),
            ast::Expr::Binary(expr) => expr.assemble(c, block),
            ast::Expr::Path(expr) => expr.assemble(c, block),
            ast::Expr::If(expr) => expr.assemble(c, block),
            ast::Expr::Assign(expr) => expr.assemble(c, block),
            _ => Err(CompileError::msg(span, "unsupported expr")),
        }
    }
}

impl Assemble for ast::ExprLit {
    fn assemble(&self, c: &mut Compiler<'_>, block: Block) -> CompileResult<(Block, Var)> {
        use num::ToPrimitive as _;

        let span = self.span();
        log::trace!("ExprLit => {:?}", c.source.source(span));

        let value = match &self.lit {
            ast::Lit::Bool(b) => block.constant(Constant::Bool(b.value)),
            ast::Lit::Byte(b) => {
                let b = c.resolve(b)?;
                block.constant(Constant::Byte(b))
            }
            ast::Lit::Str(s) => {
                let s = c.resolve(s)?;
                block.constant(Constant::String(s.into()))
            }
            ast::Lit::ByteStr(b) => {
                let b = c.resolve(b)?;
                block.constant(Constant::Bytes(b.into()))
            }
            ast::Lit::Char(ch) => {
                let ch = c.resolve(ch)?;
                block.constant(Constant::Char(ch))
            }
            ast::Lit::Number(n) => match c.resolve(n)? {
                ast::Number::Float(n) => block.constant(Constant::float(n).with_span(span)?),
                ast::Number::Integer(n) => {
                    let n = match n.to_i64() {
                        Some(n) => n,
                        None => {
                            return Err(CompileError::new(
                                span,
                                ParseErrorKind::BadNumberOutOfBounds,
                            ));
                        }
                    };

                    block.constant(Constant::Integer(n))
                }
            },
        };

        let value = value.with_span(span)?;
        Ok((block, value))
    }
}

impl Assemble for ast::ExprBinary {
    fn assemble(&self, c: &mut Compiler<'_>, mut block: Block) -> CompileResult<(Block, Var)> {
        let span = self.span();
        log::trace!("ExprBinary => {:?}", c.source.source(span));

        let (next, lhs) = self.lhs.assemble(c, block)?;
        block = next;
        let (next, rhs) = self.rhs.assemble(c, block)?;
        block = next;

        let value = match self.op {
            ast::BinOp::Add => block.add(lhs, rhs),
            ast::BinOp::Sub => block.sub(lhs, rhs),
            ast::BinOp::Div => block.div(lhs, rhs),
            ast::BinOp::Mul => block.mul(lhs, rhs),
            _ => return Err(CompileError::msg(self.op_span(), "unsupported binary op")),
        };

        let value = value.with_span(span)?;
        Ok((block, value))
    }
}

impl Assemble for ast::ExprUnary {
    fn assemble(&self, c: &mut Compiler<'_>, block: Block) -> CompileResult<(Block, Var)> {
        let span = self.span();
        log::trace!("ExprUnary => {:?}", c.source.source(span));

        let (block, expr) = self.expr.assemble(c, block)?;

        let value = match self.op {
            ast::UnOp::Not => block.not(expr),
            _ => return Err(CompileError::msg(self.op_span(), "unsupported unary op")),
        };

        let value = value.with_span(span)?;
        Ok((block, value))
    }
}

impl Assemble for ast::Path {
    fn assemble(&self, c: &mut Compiler<'_>, block: Block) -> CompileResult<(Block, Var)> {
        let span = self.span();
        log::trace!("Path => {:?}", c.source.source(span));

        if let Some(ident) = self.try_as_ident() {
            let name = c.resolve(ident)?;
            let var = c.scope.get(span, &name)?;
            return Ok((block, var));
        }

        Err(CompileError::msg(span, "unsupported path"))
    }
}

impl Assemble for ast::ExprIf {
    fn assemble(&self, c: &mut Compiler<'_>, block: Block) -> CompileResult<(Block, Var)> {
        let span = self.span();
        log::trace!("ExprIf => {:?}", c.source.source(span));

        let mut branches = Branches::new();
        branches.conditional(&self.block, &self.condition);

        for else_if in &self.expr_else_ifs {
            branches.conditional(&else_if.block, &else_if.condition);
        }

        if let Some(else_) = &self.expr_else {
            branches.fallback(&else_.block);
        }

        branches.assemble(span, c, block)
    }
}

impl Assemble for ast::ExprAssign {
    fn assemble(&self, c: &mut Compiler<'_>, block: Block) -> CompileResult<(Block, Var)> {
        let span = self.span();
        log::trace!("ExprAssign => {:?}", c.source.source(span));

        match &self.lhs {
            // <var> <op> <expr>
            ast::Expr::Path(path) if path.rest.is_empty() => {
                let ident = path
                    .first
                    .try_as_ident()
                    .ok_or_else(|| CompileError::msg(path, "unsupported path segment"))?;

                let name = c.resolve(ident)?;
                let id = c.scope.get(span, &name)?;
                let (block, value) = self.rhs.assemble(c, block)?;
                block.assign(id, value).with_span(span)?;
                let value = block.unit().with_span(span)?;
                return Ok((block, value));
            }
            _ => return Err(CompileError::msg(span, "unsupported op")),
        }
    }
}

impl Assemble for ast::Condition {
    fn assemble(&self, c: &mut Compiler<'_>, block: Block) -> CompileResult<(Block, Var)> {
        let span = self.span();
        log::trace!("Condition => {:?}", c.source.source(span));

        match self {
            ast::Condition::Expr(expr) => expr.assemble(c, block),
            ast::Condition::ExprLet(_) => Err(CompileError::msg(span, "unsupported condition")),
        }
    }
}
