use crate::eval::prelude::*;

impl Eval<&ast::ExprIf> for ConstCompiler<'_> {
    fn eval(&mut self, expr_if: &ast::ExprIf, used: Used) -> Result<ConstValue, EvalOutcome> {
        self.budget.take(expr_if)?;
        let value = expr_if.condition.as_bool(self, used)?;

        if value {
            return self.eval(&*expr_if.block, used);
        }

        for else_if in &expr_if.expr_else_ifs {
            let value = else_if.condition.as_bool(self, used)?;

            if value {
                return self.eval(&*else_if.block, used);
            }
        }

        if let Some(expr_else) = &expr_if.expr_else {
            return self.eval(&*expr_else.block, used);
        }

        Ok(ConstValue::Unit)
    }
}
