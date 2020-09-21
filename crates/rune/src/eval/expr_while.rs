use crate::eval::prelude::*;

impl Eval<&ast::ExprWhile> for ConstCompiler<'_> {
    fn eval(&mut self, expr_while: &ast::ExprWhile, used: Used) -> Result<ConstValue, EvalOutcome> {
        let span = expr_while.span();
        self.budget.take(span)?;

        let loop_label = match &expr_while.label {
            Some((label, _)) => Some(<Box<str>>::from(self.resolve(label)?)),
            None => None,
        };

        while expr_while.condition.as_bool(self, used)? {
            match self.eval(&*expr_while.body, used) {
                Ok(_) => (),
                Err(outcome) => {
                    // Handle potential outcomes which apply to this loop.
                    match &outcome {
                        EvalOutcome::Break(_, EvalBreak::Empty) => break,
                        EvalOutcome::Break(_, EvalBreak::Label(label))
                            if Some(label) == loop_label.as_ref() =>
                        {
                            break
                        }
                        _ => (),
                    }

                    return Err(outcome);
                }
            }
        }

        Ok(ConstValue::Unit)
    }
}
