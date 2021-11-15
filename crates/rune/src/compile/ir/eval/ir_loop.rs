use crate::compile::ir::eval::prelude::*;

impl IrEval for ir::IrLoop {
    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<IrValue, IrEvalOutcome> {
        let span = self.span();
        interp.budget.take(span)?;

        let guard = interp.scopes.push();

        loop {
            if let Some(condition) = &self.condition {
                interp.scopes.clear_current(&*condition)?;

                if !condition.eval_bool(interp, used)? {
                    break;
                }
            }

            match self.body.eval(interp, used) {
                Ok(..) => (),
                Err(outcome) => match outcome {
                    IrEvalOutcome::Break(span, b) => match b {
                        IrEvalBreak::Inherent => break,
                        IrEvalBreak::Label(l) => {
                            if self.label.as_ref() == Some(&l) {
                                break;
                            }

                            return Err(IrEvalOutcome::Break(span, IrEvalBreak::Label(l)));
                        }
                        IrEvalBreak::Value(value) => {
                            if self.condition.is_none() {
                                return Ok(value);
                            }

                            return Err(IrEvalOutcome::from(IrError::msg(
                                span,
                                "break with value is not supported for unconditional loops",
                            )));
                        }
                    },
                    outcome => return Err(outcome),
                },
            };
        }

        interp.scopes.pop(self, guard)?;
        Ok(IrValue::Unit)
    }
}
