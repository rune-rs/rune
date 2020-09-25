use crate::ir::eval::prelude::*;

impl Eval<&ir::IrCondition> for IrInterpreter<'_> {
    type Output = bool;

    fn eval(
        &mut self,
        ir_condition: &ir::IrCondition,
        used: Used,
    ) -> Result<Self::Output, EvalOutcome> {
        match ir_condition {
            ir::IrCondition::Ir(ir) => Ok(ir.as_bool(self, used)?),
            ir::IrCondition::Let(ir_let) => {
                let value = self.eval(&ir_let.ir, used)?;
                Ok(ir_let.pat.matches(self, value, used, ir_condition)?)
            }
        }
    }
}
