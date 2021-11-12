use crate::ir::eval::prelude::*;

impl IrEval for &ir::IrTuple {
    type Output = IrValue;

    fn eval(
        &self,
        interp: &mut IrInterpreter<'_, '_>,
        used: Used,
    ) -> Result<Self::Output, IrEvalOutcome> {
        let mut items = Vec::with_capacity(self.items.len());

        for item in self.items.iter() {
            items.push(item.eval(interp, used)?);
        }

        Ok(IrValue::Tuple(Shared::new(items.into_boxed_slice())))
    }
}
