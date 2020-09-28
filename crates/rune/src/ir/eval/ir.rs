use crate::ir::eval::prelude::*;

/// Eval the interior expression.
impl Eval<&ir::Ir> for IrInterpreter<'_> {
    type Output = IrValue;

    fn eval(&mut self, ir: &ir::Ir, used: Used) -> Result<Self::Output, EvalOutcome> {
        self.budget.take(ir)?;

        match &ir.kind {
            ir::IrKind::Scope(ir_scope) => self.eval(ir_scope, used),
            ir::IrKind::Binary(ir_binary) => self.eval(ir_binary, used),
            ir::IrKind::Decl(ir_decl) => self.eval(ir_decl, used),
            ir::IrKind::Set(ir_set) => self.eval(ir_set, used),
            ir::IrKind::Assign(ir_assign) => self.eval(ir_assign, used),
            ir::IrKind::Template(ir_template) => self.eval(ir_template, used),
            ir::IrKind::Name(name) => Ok(self.resolve_var(name.as_ref(), ir.span(), used)?),
            ir::IrKind::Target(ir_target) => Ok(self.scopes.get_target(ir_target)?),
            ir::IrKind::Value(const_value) => Ok(IrValue::from_const(const_value.clone())),
            ir::IrKind::Branches(branches) => self.eval(branches, used),
            ir::IrKind::Loop(ir_loop) => self.eval(ir_loop, used),
            ir::IrKind::Break(ir_break) => {
                self.eval(ir_break, used)?;
                Ok(IrValue::Unit)
            }
            ir::IrKind::Vec(ir_vec) => self.eval(ir_vec, used),
            ir::IrKind::Tuple(ir_tuple) => self.eval(ir_tuple, used),
            ir::IrKind::Object(ir_object) => self.eval(ir_object, used),
            ir::IrKind::Call(ir_call) => self.eval(ir_call, used),
        }
    }
}
