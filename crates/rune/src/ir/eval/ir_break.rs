use crate::ir::eval::prelude::*;

impl Eval<&ir::IrBreak> for IrInterpreter<'_> {
    type Output = ();

    fn eval(&mut self, ir_break: &ir::IrBreak, used: Used) -> Result<(), EvalOutcome> {
        let span = ir_break.span();
        self.budget.take(span)?;

        match &ir_break.kind {
            ir::IrBreakKind::Ir(ir) => {
                let value = self.eval(&**ir, used)?;
                Err(EvalOutcome::Break(span, EvalBreak::Value(value)))
            }
            ir::IrBreakKind::Label(label) => {
                Err(EvalOutcome::Break(span, EvalBreak::Label(label.clone())))
            }
            ir::IrBreakKind::Inherent => Err(EvalOutcome::Break(span, EvalBreak::Inherent)),
        }
    }
}
