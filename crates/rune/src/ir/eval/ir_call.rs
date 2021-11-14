use crate::ir::eval::prelude::*;

impl IrEval for ir::IrCall {
    type Output = IrValue;

    fn eval(
        &self,
        interp: &mut IrInterpreter<'_>,
        used: Used,
    ) -> Result<Self::Output, IrEvalOutcome> {
        let mut args = Vec::new();

        for arg in &self.args {
            args.push(arg.eval(interp, used)?);
        }

        Ok(interp.call_const_fn(self, &self.target, args, used)?)
    }
}
