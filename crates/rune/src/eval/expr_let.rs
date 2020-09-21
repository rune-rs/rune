use crate::eval::prelude::*;

impl Eval<&ast::ExprLet> for ConstCompiler<'_> {
    fn eval(&mut self, expr_let: &ast::ExprLet, used: Used) -> Result<ConstValue, EvalOutcome> {
        match &expr_let.pat {
            ast::Pat::PatPath(path) => {
                if let Some(ident) = path.path.try_as_ident() {
                    let name = self.resolve(ident)?;
                    let value = self.eval(&*expr_let.expr, used)?;
                    self.scopes.decl(name.as_ref(), value, ident.span())?;
                    return Ok(ConstValue::Unit);
                }
            }
            _ => (),
        }

        Err(EvalOutcome::not_const(expr_let))
    }
}
