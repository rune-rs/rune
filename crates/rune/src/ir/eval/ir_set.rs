use crate::ir::eval::prelude::*;

impl IrEval for ir::IrSet {
    type Output = IrValue;

    fn eval(
        &self,
        interp: &mut IrInterpreter<'_>,
        used: Used,
    ) -> Result<Self::Output, IrEvalOutcome> {
        interp.budget.take(self)?;
        let value = self.value.eval(interp, used)?;
        interp.scopes.set_target(&self.target, value)?;
        Ok(IrValue::Unit)
    }
}
