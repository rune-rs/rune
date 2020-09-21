use crate::eval::prelude::*;

impl Eval<&ast::ExprLit> for ConstCompiler<'_> {
    fn eval(
        &mut self,
        expr_lit: &ast::ExprLit,
        used: Used,
    ) -> Result<Option<ConstValue>, crate::CompileError> {
        match &expr_lit.lit {
            ast::Lit::Bool(b) => {
                return Ok(Some(ConstValue::Bool(b.value)));
            }
            ast::Lit::Number(n) => {
                let n = n.resolve(&self.query.storage, self.source)?;

                return Ok(Some(match n {
                    ast::Number::Integer(n) => ConstValue::Integer(n),
                    ast::Number::Float(n) => ConstValue::Float(n),
                }));
            }
            ast::Lit::Template(lit_template) => {
                return self.eval(lit_template, used);
            }
            ast::Lit::Str(s) => {
                let s = s.resolve(&self.query.storage, self.source)?;
                return Ok(Some(ConstValue::String(s.into())));
            }
            _ => (),
        }

        Ok(None)
    }
}
