use crate::ir::eval::prelude::*;

impl Eval<&ir::IrLoop> for IrInterpreter<'_> {
    type Output = IrValue;

    fn eval(&mut self, ir_loop: &ir::IrLoop, used: Used) -> Result<IrValue, EvalOutcome> {
        let span = ir_loop.span();
        self.budget.take(span)?;

        let guard = self.scopes.push();

        loop {
            if let Some(condition) = &ir_loop.condition {
                self.scopes.clear_current(&*condition)?;

                if !self.eval(&**condition, used)? {
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

                            return Err(EvalOutcome::from(IrError::custom(
                                span,
                                "break with value is not supported for unconditional loops",
                            )));
                        }
                    },
                    outcome => return Err(outcome),
                },
            };
        }

        self.scopes.pop(ir_loop, guard)?;
        Ok(IrValue::Unit)
    }
}
