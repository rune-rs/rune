use crate::compile::ir::eval::prelude::*;

impl IrEval for ir::IrSet {
    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<IrValue, IrEvalOutcome> {
        interp.budget.take(self)?;
        let value = self.value.eval(interp, used)?;
        interp.scopes.set_target(&self.target, value)?;
        Ok(IrValue::Unit)
    }
}
