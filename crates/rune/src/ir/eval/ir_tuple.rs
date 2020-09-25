use crate::ir::eval::prelude::*;

impl Eval<&ir::IrTuple> for IrInterpreter<'_> {
    type Output = IrValue;

    fn eval(&mut self, ir_tuple: &ir::IrTuple, used: Used) -> Result<Self::Output, EvalOutcome> {
        let mut items = Vec::with_capacity(ir_tuple.items.len());

        for item in ir_tuple.items.iter() {
            items.push(self.eval(item, used)?);
        }

        Ok(IrValue::Tuple(Shared::new(items.into_boxed_slice())))
    }
}
