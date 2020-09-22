use crate::eval::prelude::*;

impl Eval<&IrScope> for IrInterpreter<'_> {
    fn eval(&mut self, im_scope: &IrScope, used: Used) -> Result<ConstValue, EvalOutcome> {
        self.budget.take(im_scope)?;
        let _guard = self.scopes.push();

        for im in &im_scope.instructions {
            let _ = self.eval(im, used)?;
        }

        if let Some(last) = &im_scope.last {
            self.eval(&**last, used)
        } else {
            Ok(ConstValue::Unit)
        }
    }
}
