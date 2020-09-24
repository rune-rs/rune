use crate::eval::prelude::*;

impl Eval<&IrLoop> for IrInterpreter<'_> {
    fn eval(&mut self, ir_loop: &IrLoop, used: Used) -> Result<IrValue, EvalOutcome> {
        let span = ir_loop.span();
        self.budget.take(span)?;

        loop {
            if let Some(condition) = &ir_loop.condition {
                if !condition.as_bool(self, used)? {
                    break;
                }
            }

            match self.eval(&ir_loop.body, used) {
                Ok(..) => (),
                Err(outcome) => match outcome {
                    EvalOutcome::Break(span, b) => match b {
                        EvalBreak::Inherent => break,
                        EvalBreak::Label(l) => {
                            if ir_loop.label.as_ref() == Some(&l) {
                                break;
                            }

                            return Err(EvalOutcome::Break(span, EvalBreak::Label(l)));
                        }
                        EvalBreak::Value(value) => {
                            if ir_loop.condition.is_none() {
                                return Ok(value);
                            }

                            return Err(EvalOutcome::from(CompileError::const_error(
                                span,
                                "break with value is not supported for unconditional loops",
                            )));
                        }
                    },
                    outcome => return Err(outcome),
                },
            };
        }

        Ok(IrValue::Unit)
    }
}
