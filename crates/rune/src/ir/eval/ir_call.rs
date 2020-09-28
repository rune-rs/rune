use crate::ir::eval::prelude::*;

impl Eval<&ir::IrCall> for IrInterpreter<'_> {
    type Output = IrValue;

    fn eval(&mut self, ir_call: &ir::IrCall, used: Used) -> Result<Self::Output, EvalOutcome> {
        let mut args = Vec::new();

        for arg in &ir_call.args {
            args.push(self.eval(arg, used)?);
        }

        Ok(self.call_const_fn(ir_call, &*ir_call.target, args, used)?)
    }
}
