use crate::ir::eval::prelude::*;

/// IrEval the interior expression.
impl IrEval for ir::Ir {
    type Output = IrValue;

    fn eval(
        &self,
        interp: &mut IrInterpreter<'_, '_>,
        used: Used,
    ) -> Result<Self::Output, IrEvalOutcome> {
        interp.budget.take(self)?;

        match &self.kind {
            ir::IrKind::Scope(ir_scope) => ir_scope.eval(interp, used),
            ir::IrKind::Binary(ir_binary) => ir_binary.eval(interp, used),
            ir::IrKind::Decl(ir_decl) => ir_decl.eval(interp, used),
            ir::IrKind::Set(ir_set) => ir_set.eval(interp, used),
            ir::IrKind::Assign(ir_assign) => ir_assign.eval(interp, used),
            ir::IrKind::Template(ir_template) => ir_template.eval(interp, used),
            ir::IrKind::Name(name) => Ok(interp.resolve_var(self.span(), name.as_ref(), used)?),
            ir::IrKind::Target(ir_target) => Ok(interp.scopes.get_target(ir_target)?),
            ir::IrKind::Value(value) => Ok(IrValue::from_const(value.clone())),
            ir::IrKind::Branches(branches) => branches.eval(interp, used),
            ir::IrKind::Loop(ir_loop) => ir_loop.eval(interp, used),
            ir::IrKind::Break(ir_break) => {
                ir_break.eval(interp, used)?;
                Ok(IrValue::Unit)
            }
            ir::IrKind::Vec(ir_vec) => ir_vec.eval(interp, used),
            ir::IrKind::Tuple(ir_tuple) => ir_tuple.eval(interp, used),
            ir::IrKind::Object(ir_object) => ir_object.eval(interp, used),
            ir::IrKind::Call(ir_call) => ir_call.eval(interp, used),
        }
    }
}
