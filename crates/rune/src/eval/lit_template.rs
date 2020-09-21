use crate::eval::prelude::*;
use std::fmt::Write as _;

impl Eval<&ast::LitTemplate> for ConstCompiler<'_> {
    fn eval(
        &mut self,
        lit_template: &ast::LitTemplate,
        used: Used,
    ) -> Result<ConstValue, EvalOutcome> {
        let span = lit_template.span();
        self.budget.take(span)?;
        let template = self.resolve(lit_template)?;

        let mut buf = String::new();

        for component in &template.components {
            match component {
                ast::TemplateComponent::String(string) => {
                    self.budget.take(lit_template)?;
                    buf.push_str(&string);
                }
                ast::TemplateComponent::Expr(expr) => {
                    let const_value = self.eval(&**expr, used)?;

                    match const_value {
                        ConstValue::String(s) => {
                            buf.push_str(&s);
                        }
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
                        _ => {
                            return Err(EvalOutcome::not_const(lit_template));
                        }
                    }
                }
            }
        }

        Ok(ConstValue::String(buf.into_boxed_str()))
    }
}
