use crate::ir::eval::prelude::*;
use std::fmt::Write as _;

impl IrEval for &ir::IrTemplate {
    type Output = IrValue;

    fn eval(
        &self,
        interp: &mut IrInterpreter<'_>,
        used: Used,
    ) -> Result<Self::Output, IrEvalOutcome> {
        interp.budget.take(self)?;

        let mut buf = String::new();

        for component in &self.components {
            match component {
                ir::IrTemplateComponent::String(string) => {
                    buf.push_str(string);
                }
                ir::IrTemplateComponent::Ir(ir) => {
                    let const_value = ir.eval(interp, used)?;

                    match const_value {
                        IrValue::Integer(integer) => {
                            write!(buf, "{}", integer).unwrap();
                        }
                        IrValue::Float(float) => {
                            let mut buffer = ryu::Buffer::new();
                            buf.push_str(buffer.format(float));
                        }
                        IrValue::Bool(b) => {
                            write!(buf, "{}", b).unwrap();
                        }
                        IrValue::String(s) => {
                            let s = s.borrow_ref().map_err(IrError::access(self))?;
                            buf.push_str(&*s);
                        }
                        _ => {
                            return Err(IrEvalOutcome::not_const(self));
                        }
                    }
                }
            }
        }

        Ok(IrValue::String(Shared::new(buf)))
    }
}
