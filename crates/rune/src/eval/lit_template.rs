use crate::eval::prelude::*;
use std::fmt::Write as _;

impl Eval<&ast::LitTemplate> for ConstCompiler<'_> {
    fn eval(
        &mut self,
        lit_template: &ast::LitTemplate,
        used: Used,
    ) -> Result<Option<ConstValue>, crate::CompileError> {
        self.budget.take(lit_template)?;

        let template = self.resolve(lit_template)?;

        let mut buf = String::new();

        for component in template.components {
            match component {
                ast::TemplateComponent::String(string) => {
                    buf.push_str(&string);
                }
                ast::TemplateComponent::Expr(expr) => {
                    let span = expr.span();
                    let const_value = self
                        .eval(&*expr, used)?
                        .ok_or_else(|| CompileError::not_const(span))?;

                    match const_value {
                        ConstValue::Unit => {}
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
                    }
                }
            }
        }

        Ok(Some(ConstValue::String(buf.into_boxed_str())))
    }
}
