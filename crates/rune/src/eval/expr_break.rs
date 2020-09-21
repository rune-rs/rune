use crate::eval::prelude::*;

impl Eval<&ast::ExprBreak> for ConstCompiler<'_> {
    fn eval(&mut self, expr_break: &ast::ExprBreak, used: Used) -> Result<ConstValue, EvalOutcome> {
        let span = expr_break.span();
        self.budget.take(span)?;

        match &expr_break.expr {
            Some(expr_break_value) => match expr_break_value {
                ast::ExprBreakValue::Label(label) => {
                    let label = self.resolve(label)?;
                    Err(EvalOutcome::Break(span, EvalBreak::Label(label.into())))
                }
                ast::ExprBreakValue::Expr(expr) => {
                    let value = self.eval(&**expr, used)?;
                    Err(EvalOutcome::Break(span, EvalBreak::Value(value)))
                }
            },
            None => Err(EvalOutcome::Break(span, EvalBreak::Empty)),
        }
    }
}
