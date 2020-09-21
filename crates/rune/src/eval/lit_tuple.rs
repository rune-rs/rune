use crate::eval::prelude::*;

impl Eval<&ast::LitTuple> for ConstCompiler<'_> {
    fn eval(&mut self, lit_tuple: &ast::LitTuple, used: Used) -> Result<ConstValue, EvalOutcome> {
        self.budget.take(lit_tuple)?;
        let mut tuple = Vec::new();

        for (expr, _) in &lit_tuple.items {
            tuple.push(self.eval(expr, used)?);
        }

        Ok(ConstValue::Tuple(tuple.into_boxed_slice()))
    }
}
