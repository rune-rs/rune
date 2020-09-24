use crate::eval::prelude::*;

/// Eval the interior expression.
impl Eval<&Ir> for IrInterpreter<'_> {
    fn eval(&mut self, ir: &Ir, used: Used) -> Result<IrValue, EvalOutcome> {
        self.budget.take(ir)?;

        match &ir.kind {
            IrKind::Scope(ir_scope) => self.eval(ir_scope, used),
            IrKind::Binary(ir_binary) => self.eval(ir_binary, used),
            IrKind::Decl(ir_decl) => self.eval(ir_decl, used),
            IrKind::Set(ir_set) => self.eval(ir_set, used),
            IrKind::Template(ir_template) => self.eval(ir_template, used),
            IrKind::Name(name) => Ok(self.resolve_var(name.as_ref(), ir.span(), used)?),
            IrKind::Target(ir_target) => Ok(self.scopes.get_target(ir_target)?),
            IrKind::Value(const_value) => Ok(IrValue::from_const(const_value.clone())),
            IrKind::Branches(branches) => self.eval(branches, used),
            IrKind::Loop(ir_loop) => self.eval(ir_loop, used),
            IrKind::Break(ir_break) => self.eval(ir_break, used),
            IrKind::Vec(ir_vec) => self.eval(ir_vec, used),
            IrKind::Tuple(ir_tuple) => self.eval(ir_tuple, used),
            IrKind::Object(ir_object) => self.eval(ir_object, used),
        }
    }
}
