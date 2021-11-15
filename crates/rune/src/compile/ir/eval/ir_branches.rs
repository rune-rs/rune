use crate::compile::ir::eval::prelude::*;

impl IrEval for ir::IrBranches {
    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<IrValue, IrEvalOutcome> {
        for (ir_condition, branch) in &self.branches {
            let guard = interp.scopes.push();

            let output = if ir_condition.eval_bool(interp, used)? {
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
            return branch.eval(interp, used);
        }

        Ok(IrValue::Unit)
    }
}
