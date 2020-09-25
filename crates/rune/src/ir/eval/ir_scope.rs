use crate::ir::eval::prelude::*;

impl Eval<&ir::IrScope> for IrInterpreter<'_> {
    type Output = IrValue;

    fn eval(&mut self, ir_scope: &ir::IrScope, used: Used) -> Result<Self::Output, EvalOutcome> {
        self.budget.take(ir_scope)?;
        let guard = self.scopes.push();

        for im in &ir_scope.instructions {
            let _ = self.eval(im, used)?;
        }

        let value = if let Some(last) = &ir_scope.last {
            self.eval(&**last, used)?
        } else {
            IrValue::Unit
        };

        self.scopes.pop(ir_scope, guard)?;
        Ok(value)
    }
}
