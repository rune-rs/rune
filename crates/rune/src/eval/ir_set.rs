use crate::eval::prelude::*;

impl Eval<&IrSet> for IrInterpreter<'_> {
    fn eval(&mut self, im_set: &IrSet, used: Used) -> Result<ConstValue, EvalOutcome> {
        self.budget.take(im_set)?;
        let value = self.eval(&*im_set.value, used)?;
        self.scopes.replace(&im_set.name, value, im_set)?;
        Ok(ConstValue::Unit)
    }
}
