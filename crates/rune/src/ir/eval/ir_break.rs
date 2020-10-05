use crate::ir::eval::prelude::*;

impl IrEval for ir::IrBreak {
    type Output = ();

    fn eval(&self, interp: &mut IrInterpreter<'_>, used: Used) -> Result<(), IrEvalOutcome> {
        let span = self.span();
        interp.budget.take(span)?;

        match &self.kind {
            ir::IrBreakKind::Ir(ir) => {
                let value = ir.eval(interp, used)?;
                Err(IrEvalOutcome::Break(span, IrEvalBreak::Value(value)))
            }
            ir::IrBreakKind::Label(label) => Err(IrEvalOutcome::Break(
                span,
                IrEvalBreak::Label(label.clone()),
            )),
            ir::IrBreakKind::Inherent => Err(IrEvalOutcome::Break(span, IrEvalBreak::Inherent)),
        }
    }
}
