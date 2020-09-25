use crate::eval::prelude::*;

impl Eval<&IrCondition> for IrInterpreter<'_> {
    type Output = bool;

    fn eval(
        &mut self,
        ir_condition: &IrCondition,
        used: Used,
    ) -> Result<Self::Output, EvalOutcome> {
        match ir_condition {
            IrCondition::Ir(ir) => Ok(ir.as_bool(self, used)?),
            IrCondition::Let(ir_let) => {
                let value = self.eval(&ir_let.ir, used)?;
                Ok(ir_let.pat.matches(self, value, used, ir_condition)?)
            }
        }
    }
}
