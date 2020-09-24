use crate::eval::prelude::*;

impl Eval<&IrBranches> for IrInterpreter<'_> {
    fn eval(&mut self, ir_branches: &IrBranches, used: Used) -> Result<IrValue, EvalOutcome> {
        for (ir, branch) in &ir_branches.branches {
            if ir.as_bool(self, used)? {
                return self.eval(branch, used);
            }
        }

        if let Some(branch) = &ir_branches.default_branch {
            return self.eval(branch, used);
        }

        Ok(IrValue::Unit)
    }
}
