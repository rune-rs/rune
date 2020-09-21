use crate::eval::prelude::*;

impl Eval<&ast::LitVec> for ConstCompiler<'_> {
    fn eval(&mut self, lit_vec: &ast::LitVec, used: Used) -> Result<ConstValue, EvalOutcome> {
        self.budget.take(lit_vec)?;

        let mut tuple = Vec::new();

        for (expr, _) in &lit_vec.items {
            tuple.push(self.eval(expr, used)?);
        }

        Ok(ConstValue::Tuple(tuple.into_boxed_slice()))
    }
}
