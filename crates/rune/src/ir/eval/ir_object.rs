use crate::collections::HashMap;
use crate::ir::eval::prelude::*;

impl IrEval for ir::IrObject {
    type Output = IrValue;

    fn eval(
        &self,
        interp: &mut IrInterpreter<'_>,
        used: Used,
    ) -> Result<Self::Output, IrEvalOutcome> {
        let mut object = HashMap::with_capacity(self.assignments.len());

        for (key, value) in self.assignments.iter() {
            object.insert(key.as_ref().to_owned(), value.eval(interp, used)?);
        }

        Ok(IrValue::Object(Shared::new(object)))
    }
}
