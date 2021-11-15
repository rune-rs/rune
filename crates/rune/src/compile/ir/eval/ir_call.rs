use crate::compile::ir::eval::prelude::*;

impl IrEval for ir::IrCall {
    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<IrValue, IrEvalOutcome> {
        let mut args = Vec::new();

        for arg in &self.args {
            args.push(arg.eval(interp, used)?);
        }

        Ok(interp.call_const_fn(self, &self.target, args, used)?)
    }
}
