use crate::compile::ir::eval::prelude::*;

impl IrEval for ir::IrScope {
    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<IrValue, IrEvalOutcome> {
        interp.budget.take(self)?;
        let guard = interp.scopes.push();

        for ir in &self.instructions {
            let _ = ir.eval(interp, used)?;
        }

        let value = if let Some(last) = &self.last {
            last.eval(interp, used)?
        } else {
            IrValue::Unit
        };

        interp.scopes.pop(self, guard)?;
        Ok(value)
    }
}
