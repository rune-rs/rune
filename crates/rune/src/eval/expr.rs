use crate::eval::prelude::*;

/// Eval the interior expression.
impl Eval<&ast::Expr> for ConstCompiler<'_> {
    fn eval(&mut self, expr: &ast::Expr, used: Used) -> Result<Option<ConstValue>, CompileError> {
        match expr {
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
            _ => (),
        }

        Ok(None)
    }
}
