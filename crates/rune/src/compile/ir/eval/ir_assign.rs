use crate::compile::ir::eval::prelude::*;

impl IrEval for ir::IrAssign {
    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<IrValue, IrEvalOutcome> {
        interp.budget.take(self)?;
        let value = self.value.eval(interp, used)?;

        interp
            .scopes
            .mut_target(&self.target, move |t| self.op.assign(self, t, value))?;

        Ok(IrValue::Unit)
    }
}
