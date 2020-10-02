use crate::ir::eval::prelude::*;
use std::fmt::Write as _;

impl Eval<&ir::IrTemplate> for IrInterpreter<'_> {
    type Output = IrValue;

    fn eval(
        &mut self,
        ir_template: &ir::IrTemplate,
        used: Used,
    ) -> Result<Self::Output, EvalOutcome> {
        self.budget.take(ir_template)?;

        let mut buf = String::new();

        for component in &ir_template.components {
            match component {
                ir::IrTemplateComponent::String(string) => {
                    buf.push_str(&string);
                }
                ir::IrTemplateComponent::Ir(ir) => {
                    let const_value = self.eval(ir, used)?;

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
                            let s = s.borrow_ref().map_err(IrError::access(ir_template))?;
                            buf.push_str(&*s);
                        }
                        _ => {
                            return Err(EvalOutcome::not_const(ir_template));
                        }
                    }
                }
            }
        }

        Ok(IrValue::String(Shared::new(buf)))
    }
}
