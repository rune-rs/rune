use crate::compiling::assemble::prelude::*;

/// Compile a unary expression.
impl Assemble for ast::ExprUnary {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprUnary => {:?}", c.source.source(span));

        // NB: special unary expressions.
        if let ast::UnOp::BorrowRef { .. } = self.op {
            return Err(CompileError::new(self, CompileErrorKind::UnsupportedRef));
        }

        if let (ast::UnOp::Neg, ast::Expr::Lit(expr_lit)) = (self.op, &self.expr) {
            if let ast::Lit::Number(n) = &expr_lit.lit {
                let n = n.resolve(&c.storage, &*c.source)?.as_i64(span, true)?;
                c.asm.push(Inst::integer(n), span);
                return Ok(());
            }
        }

        self.expr.assemble(c, Needs::Value)?;

        match self.op {
            ast::UnOp::Not { .. } => {
                c.asm.push(Inst::Not, span);
            }
            ast::UnOp::Neg { .. } => {
                c.asm.push(Inst::Neg, span);
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
            c.asm.push(Inst::Pop, span);
        }

        Ok(())
    }
}
