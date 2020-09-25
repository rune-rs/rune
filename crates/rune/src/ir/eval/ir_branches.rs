use crate::ir::eval::prelude::*;

impl Eval<&ir::IrBranches> for IrInterpreter<'_> {
    type Output = IrValue;

    fn eval(
        &mut self,
        ir_branches: &ir::IrBranches,
        used: Used,
    ) -> Result<Self::Output, EvalOutcome> {
        for (ir_condition, branch) in &ir_branches.branches {
            let guard = self.scopes.push();

            let output = if self.eval(ir_condition, used)? {
                Some(self.eval(branch, used)?)
            } else {
                None
            };

            self.scopes.pop(branch, guard)?;

            if let Some(output) = output {
                return Ok(output);
            }
        }

        if let Some(branch) = &ir_branches.default_branch {
            return self.eval(branch, used);
        }

        Ok(IrValue::Unit)
    }
}
