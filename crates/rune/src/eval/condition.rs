use crate::eval::prelude::*;

impl Eval<&ast::Condition> for ConstCompiler<'_> {
    fn eval(
        &mut self,
        condition: &ast::Condition,
        used: Used,
    ) -> Result<Option<ConstValue>, crate::CompileError> {
        self.budget.take(condition)?;

        match condition {
            ast::Condition::Expr(expr) => self.eval(&**expr, used),
            _ => Ok(None),
        }
    }
}
