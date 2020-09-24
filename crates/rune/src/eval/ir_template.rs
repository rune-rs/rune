use crate::eval::prelude::*;
use std::fmt::Write as _;

impl Eval<&IrTemplate> for IrInterpreter<'_> {
    fn eval(&mut self, ir_template: &IrTemplate, used: Used) -> Result<IrValue, EvalOutcome> {
        self.budget.take(ir_template)?;

        let mut buf = String::new();

        for component in &ir_template.components {
            match component {
                IrTemplateComponent::String(string) => {
                    buf.push_str(&string);
                }
                IrTemplateComponent::Ir(ir) => {
                    let const_value = self.eval(ir, used)?;

                    match const_value {
                        IrValue::Integer(integer) => {
                            let mut buffer = itoa::Buffer::new();
                            buf.push_str(buffer.format(integer));
                        }
                        IrValue::Float(float) => {
                            let mut buffer = ryu::Buffer::new();
                            buf.push_str(buffer.format(float));
                        }
                        IrValue::Bool(b) => {
                            write!(buf, "{}", b).unwrap();
                        }
                        IrValue::String(s) => {
                            let s = s
                                .borrow_ref()
                                .map_err(|e| CompileError::access(ir_template, e))?;
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
