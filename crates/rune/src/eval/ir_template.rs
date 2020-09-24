use crate::eval::prelude::*;
use std::fmt::Write as _;

impl Eval<&IrTemplate> for IrInterpreter<'_> {
    fn eval(&mut self, ir_template: &IrTemplate, used: Used) -> Result<ConstValue, EvalOutcome> {
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
                        ConstValue::Integer(integer) => {
                            let mut buffer = itoa::Buffer::new();
                            buf.push_str(buffer.format(integer));
                        }
                        ConstValue::Float(float) => {
                            let mut buffer = ryu::Buffer::new();
                            buf.push_str(buffer.format(float));
                        }
                        ConstValue::Bool(b) => {
                            write!(buf, "{}", b).unwrap();
                        }
                        ConstValue::String(s) => {
                            buf.push_str(&s);
                        }
                        _ => {
                            return Err(EvalOutcome::not_const(ir_template));
                        }
                    }
                }
            }
        }

        Ok(ConstValue::String(buf))
    }
}
