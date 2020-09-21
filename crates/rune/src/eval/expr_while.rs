use crate::eval::prelude::*;

impl Eval<&ast::ExprWhile> for ConstCompiler<'_> {
    fn eval(
        &mut self,
        expr_while: &ast::ExprWhile,
        used: Used,
    ) -> Result<Option<ConstValue>, crate::CompileError> {
        let span = expr_while.span();

        while expr_while.condition.as_bool(self, used)? {
            // NB: use up one budget on each loop, in case the condition is
            // constant folded.
            self.budget.take(span)?;

            self.eval(&*expr_while.body, used)?
                .ok_or_else(|| CompileError::not_const(&*expr_while.body))?;
        }

        Ok(Some(ConstValue::Unit))
    }
}
