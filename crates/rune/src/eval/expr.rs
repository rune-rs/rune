use crate::eval::prelude::*;

/// Eval the interior expression.
impl Eval<&ast::Expr> for ConstCompiler<'_> {
    fn eval(&mut self, expr: &ast::Expr, used: Used) -> Result<Option<ConstValue>, CompileError> {
        self.budget.take(expr.span())?;

        match expr {
            ast::Expr::ExprLet(expr_let) => {
                return self.eval(expr_let, used);
            }
            ast::Expr::ExprBinary(binary) => {
                return self.eval(binary, used);
            }
            ast::Expr::ExprLit(expr_lit) => {
                return self.eval(expr_lit, used);
            }
            ast::Expr::Path(path) => {
                if let Some(ident) = path.try_as_ident() {
                    let ident = ident.resolve(&self.query.storage, self.source)?;
                    let const_value = self.resolve_var(ident.as_ref(), path.span(), used)?;
                    return Ok(Some(const_value));
                }
            }
            ast::Expr::ExprBlock(expr_block) => {
                return self.eval(expr_block, used);
            }
            ast::Expr::ExprIf(expr_if) => {
                return self.eval(expr_if, used);
            }
            ast::Expr::ExprWhile(expr_while) => {
                return self.eval(expr_while, used);
            }
            _ => (),
        }

        Ok(None)
    }
}
