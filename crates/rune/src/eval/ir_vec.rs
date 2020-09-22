use crate::eval::prelude::*;

impl Eval<&IrVec> for IrInterpreter<'_> {
    fn eval(&mut self, ir_vec: &IrVec, used: Used) -> Result<ConstValue, EvalOutcome> {
        let mut items = Vec::with_capacity(ir_vec.items.len());

        for item in ir_vec.items.iter() {
            items.push(self.eval(item, used)?);
        }

        Ok(ConstValue::Vec(items.into_boxed_slice()))
    }
}
