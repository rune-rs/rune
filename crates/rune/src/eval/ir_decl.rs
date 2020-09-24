use crate::eval::prelude::*;

impl Eval<&IrDecl> for IrInterpreter<'_> {
    fn eval(&mut self, im_decl: &IrDecl, used: Used) -> Result<IrValue, EvalOutcome> {
        self.budget.take(im_decl)?;
        let value = self.eval(&*im_decl.value, used)?;
        self.scopes.decl(&im_decl.name, value, im_decl)?;
        Ok(IrValue::Unit)
    }
}
