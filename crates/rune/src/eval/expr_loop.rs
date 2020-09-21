use crate::eval::prelude::*;

impl Eval<&ast::ExprLoop> for ConstCompiler<'_> {
    fn eval(&mut self, expr_loop: &ast::ExprLoop, used: Used) -> Result<ConstValue, EvalOutcome> {
        let span = expr_loop.span();
        self.budget.take(span)?;

        let loop_label = match &expr_loop.label {
            Some((label, _)) => Some(<Box<str>>::from(self.resolve(label)?)),
            None => None,
        };

        loop {
            match self.eval(&*expr_loop.body, used) {
                Ok(_) => (),
                Err(outcome) => {
                    // Handle potential outcomes which apply to this loop.
                    match outcome {
                        EvalOutcome::Break(_, EvalBreak::Empty) => break,
                        EvalOutcome::Break(span, EvalBreak::Label(label)) => {
                            if let Some(loop_label) = &loop_label {
                                if label == *loop_label {
                                    break;
                                }
                            }

                            return Err(EvalOutcome::Break(span, EvalBreak::Label(label)));
                        }
                        EvalOutcome::Break(_, EvalBreak::Value(value)) => return Ok(value),
                        outcome => return Err(outcome),
                    }
                }
            }
        }

        Ok(ConstValue::Unit)
    }
}
