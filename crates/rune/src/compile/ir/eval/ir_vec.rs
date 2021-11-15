use crate::compile::ir::eval::prelude::*;

impl IrEval for ir::IrVec {
    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<IrValue, IrEvalOutcome> {
        let mut vec = Vec::with_capacity(self.items.len());

        for item in self.items.iter() {
            vec.push(item.eval(interp, used)?);
        }

        Ok(IrValue::Vec(Shared::new(vec)))
    }
}
