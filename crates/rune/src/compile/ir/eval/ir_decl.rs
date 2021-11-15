use crate::compile::ir::eval::prelude::*;

impl IrEval for ir::IrDecl {
    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<IrValue, IrEvalOutcome> {
        interp.budget.take(self)?;
        let value = self.value.eval(interp, used)?;
        interp.scopes.decl(&self.name, value, self)?;
        Ok(IrValue::Unit)
    }
}
