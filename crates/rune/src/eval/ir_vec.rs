use crate::eval::prelude::*;

impl Eval<&IrVec> for IrInterpreter<'_> {
    type Output = IrValue;

    fn eval(&mut self, ir_vec: &IrVec, used: Used) -> Result<Self::Output, EvalOutcome> {
        let mut vec = Vec::with_capacity(ir_vec.items.len());

        for item in ir_vec.items.iter() {
            vec.push(self.eval(item, used)?);
        }

        Ok(IrValue::Vec(Shared::new(vec)))
    }
}
