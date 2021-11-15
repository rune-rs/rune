use crate::compile::ir::eval::prelude::*;

impl IrEval for ir::IrCondition {
    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<IrValue, IrEvalOutcome> {
        Ok(IrValue::Bool(match self {
            ir::IrCondition::Ir(ir) => ir.eval_bool(interp, used)?,
            ir::IrCondition::Let(ir_let) => {
                let value = ir_let.ir.eval(interp, used)?;
                ir_let.pat.matches(interp, value, self)?
            }
        }))
    }
}
