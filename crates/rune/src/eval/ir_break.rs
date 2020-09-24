use crate::eval::prelude::*;

impl Eval<&IrBreak> for IrInterpreter<'_> {
    fn eval(&mut self, ir_break: &IrBreak, used: Used) -> Result<IrValue, EvalOutcome> {
        let span = ir_break.span();
        self.budget.take(span)?;

        match &ir_break.kind {
            IrBreakKind::Ir(ir) => {
                let value = self.eval(&**ir, used)?;
                Err(EvalOutcome::Break(span, EvalBreak::Value(value)))
            }
            IrBreakKind::Label(label) => {
                Err(EvalOutcome::Break(span, EvalBreak::Label(label.clone())))
            }
            IrBreakKind::Inherent => Err(EvalOutcome::Break(span, EvalBreak::Inherent)),
        }
    }
}
