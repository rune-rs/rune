use crate::eval::prelude::*;

impl Eval<&ast::ExprLet> for ConstCompiler<'_> {
    fn eval(
        &mut self,
        expr_let: &ast::ExprLet,
        used: Used,
    ) -> Result<Option<ConstValue>, crate::CompileError> {
        match &expr_let.pat {
            ast::Pat::PatPath(path) => {
                if let Some(ident) = path.path.try_as_ident() {
                    let name = self.resolve(ident)?;
                    let value = self
                        .eval(&*expr_let.expr, used)?
                        .ok_or_else(|| CompileError::not_const(&*expr_let.expr))?;
                    self.scopes.decl(name.as_ref(), value, ident.span())?;
                    return Ok(Some(ConstValue::Unit));
                }
            }
            _ => (),
        }

        Ok(None)
    }
}
