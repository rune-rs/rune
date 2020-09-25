use crate::ir::eval::prelude::*;

impl Eval<&ir::IrDecl> for IrInterpreter<'_> {
    type Output = IrValue;

    fn eval(&mut self, im_decl: &ir::IrDecl, used: Used) -> Result<Self::Output, EvalOutcome> {
        self.budget.take(im_decl)?;
        let value = self.eval(&*im_decl.value, used)?;
        self.scopes.decl(&im_decl.name, value, im_decl)?;
        Ok(IrValue::Unit)
    }
}
