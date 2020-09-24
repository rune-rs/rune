use crate::collections::HashMap;
use crate::eval::prelude::*;

impl Eval<&IrObject> for IrInterpreter<'_> {
    fn eval(&mut self, ir_object: &IrObject, used: Used) -> Result<IrValue, EvalOutcome> {
        let mut object = HashMap::with_capacity(ir_object.assignments.len());

        for (key, value) in ir_object.assignments.iter() {
            object.insert(key.as_ref().to_owned(), self.eval(value, used)?);
        }

        Ok(IrValue::Object(Shared::new(object)))
    }
}
