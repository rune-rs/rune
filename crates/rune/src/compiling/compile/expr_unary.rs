use crate::compiling::compile::prelude::*;

/// Compile a unary expression.
impl Compile<(&ast::ExprUnary, Needs)> for Compiler<'_> {
    fn compile(&mut self, (expr_unary, needs): (&ast::ExprUnary, Needs)) -> CompileResult<()> {
        let span = expr_unary.span();
        log::trace!("ExprUnary => {:?}", self.source.source(span));

        // NB: special unary expressions.
        if let ast::UnOp::BorrowRef { .. } = expr_unary.op {
            return Err(CompileError::new(
                expr_unary,
                CompileErrorKind::UnsupportedRef,
            ));
        }

        if let (ast::UnOp::Neg, ast::Expr::ExprLit(expr_lit)) = (expr_unary.op, &*expr_unary.expr) {
            if let ast::Lit::Number(n) = &expr_lit.lit {
                let n = n
                    .resolve(&self.storage, &*self.source)?
                    .as_i64(span, true)?;
                self.asm.push(Inst::integer(n), span);
                return Ok(());
            }
        }

        self.compile((&*expr_unary.expr, Needs::Value))?;

        match expr_unary.op {
            ast::UnOp::Not { .. } => {
                self.asm.push(Inst::Not, span);
            }
            ast::UnOp::Neg { .. } => {
                self.asm.push(Inst::Neg, span);
            }
            op => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::UnsupportedUnaryOp { op },
                ));
            }
        }

        // NB: we put it here to preserve the call in case it has side effects.
        // But if we don't need the value, then pop it from the stack.
        if !needs.value() {
            self.asm.push(Inst::Pop, span);
        }

        Ok(())
    }
}
