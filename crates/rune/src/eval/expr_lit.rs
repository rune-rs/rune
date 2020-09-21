use crate::eval::prelude::*;

impl Eval<&ast::ExprLit> for ConstCompiler<'_> {
    fn eval(&mut self, expr_lit: &ast::ExprLit, used: Used) -> Result<ConstValue, EvalOutcome> {
        self.budget.take(expr_lit)?;

        match &expr_lit.lit {
            ast::Lit::Bool(b) => {
                return Ok(ConstValue::Bool(b.value));
            }
            ast::Lit::Number(n) => {
                let n = n.resolve(&self.query.storage, self.source)?;

                return Ok(match n {
                    ast::Number::Integer(n) => ConstValue::Integer(n),
                    ast::Number::Float(n) => ConstValue::Float(n),
                });
            }
            ast::Lit::Template(lit_template) => {
                return self.eval(lit_template, used);
            }
            ast::Lit::Str(s) => {
                let s = s.resolve(&self.query.storage, self.source)?;
                return Ok(ConstValue::String(s.into()));
            }
            ast::Lit::Tuple(lit_tuple) => {
                return self.eval(lit_tuple, used);
            }
            ast::Lit::Vec(lit_vec) => {
                return self.eval(lit_vec, used);
            }
            _ => (),
        }

        Err(EvalOutcome::not_const(expr_lit))
    }
}
