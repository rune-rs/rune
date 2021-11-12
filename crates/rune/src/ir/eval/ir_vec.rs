use crate::ir::eval::prelude::*;

impl IrEval for ir::IrVec {
    type Output = IrValue;

    fn eval(
        &self,
        interp: &mut IrInterpreter<'_, '_>,
        used: Used,
    ) -> Result<Self::Output, IrEvalOutcome> {
        let mut vec = Vec::with_capacity(self.items.len());

        for item in self.items.iter() {
            vec.push(item.eval(interp, used)?);
        }

        Ok(IrValue::Vec(Shared::new(vec)))
    }
}
