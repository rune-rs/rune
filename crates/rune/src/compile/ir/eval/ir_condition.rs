use crate::compile::ir::eval::prelude::*;

impl IrEval for ir::IrCondition {
    type Output = bool;

    fn eval(
        &self,
        interp: &mut IrInterpreter<'_>,
        used: Used,
    ) -> Result<Self::Output, IrEvalOutcome> {
        match self {
            ir::IrCondition::Ir(ir) => Ok(ir.as_bool(interp, used)?),
            ir::IrCondition::Let(ir_let) => {
                let value = ir_let.ir.eval(interp, used)?;
                Ok(ir_let.pat.matches(interp, value, used, self)?)
            }
        }
    }
}
