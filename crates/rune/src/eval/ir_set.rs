use crate::eval::prelude::*;

impl Eval<&IrSet> for IrInterpreter<'_> {
    type Output = IrValue;

    fn eval(&mut self, ir_set: &IrSet, used: Used) -> Result<Self::Output, EvalOutcome> {
        self.budget.take(ir_set)?;
        let value = self.eval(&*ir_set.value, used)?;
        self.scopes.set_target(&ir_set.target, value)?;
        Ok(IrValue::Unit)
    }
}
