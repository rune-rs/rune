use crate::ir::eval::prelude::*;

impl IrEval for ir::IrBranches {
    type Output = IrValue;

    fn eval(
        &self,
        interp: &mut IrInterpreter<'_, '_>,
        used: Used,
    ) -> Result<Self::Output, IrEvalOutcome> {
        for (ir_condition, branch) in &self.branches {
            let guard = interp.scopes.push();

            let output = if ir_condition.eval(interp, used)? {
                Some(branch.eval(interp, used)?)
            } else {
                None
            };

            interp.scopes.pop(branch, guard)?;

            if let Some(output) = output {
                return Ok(output);
            }
        }

        if let Some(branch) = &self.default_branch {
            return interp.eval(branch, used);
        }

        Ok(IrValue::Unit)
    }
}
