use crate::eval::prelude::*;

impl Eval<&IrObject> for IrInterpreter<'_> {
    fn eval(&mut self, ir_object: &IrObject, used: Used) -> Result<ConstValue, EvalOutcome> {
        let mut keys = Vec::new();
        let mut values = Vec::new();

        for (key, value) in ir_object.assignments.iter() {
            keys.push(key.clone());
            values.push(self.eval(value, used)?);
        }

        Ok(ConstValue::Object(ConstObject {
            keys: keys.into_boxed_slice(),
            values: values.into_boxed_slice(),
        }))
    }
}
