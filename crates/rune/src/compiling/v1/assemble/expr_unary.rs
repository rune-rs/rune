use crate::compiling::v1::assemble::prelude::*;

/// Compile a unary expression.
impl Assemble for ast::ExprUnary {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("ExprUnary => {:?}", c.source.source(span));

        // NB: special unary expressions.
        if let ast::UnOp::BorrowRef { .. } = self.op {
            return Err(CompileError::new(self, CompileErrorKind::UnsupportedRef));
        }

        if let (ast::UnOp::Neg, ast::Expr::Lit(expr_lit)) = (self.op, &self.expr) {
            if let ast::Lit::Number(n) = &expr_lit.lit {
                match n.resolve(c.storage, &*c.source)? {
                    ast::Number::Float(n) => {
                        c.asm.push(Inst::float(-n), span);
                    }
                    ast::Number::Integer(int) => {
                        use num::ToPrimitive as _;
                        use std::ops::Neg as _;

                        let n = match int.neg().to_i64() {
                            Some(n) => n,
                            None => {
                                return Err(CompileError::new(
                                    span,
                                    ParseErrorKind::BadNumberOutOfBounds,
                                ));
                            }
                        };

                        c.asm.push(Inst::integer(n), span);
                    }
                }

                return Ok(Asm::top(span));
            }
        }

        self.expr.assemble(c, Needs::Value)?.apply(c)?;

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

        Ok(Asm::top(span))
    }
}
