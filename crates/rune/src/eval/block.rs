use crate::eval::prelude::*;

impl Eval<&ast::Block> for ConstCompiler<'_> {
    fn eval(&mut self, block: &ast::Block, used: Used) -> Result<ConstValue, EvalOutcome> {
        self.budget.take(block)?;
        let _guard = self.scopes.push();
        let mut last = None::<(&ast::Expr, bool)>;

        for stmt in &block.statements {
            let (expr, term) = match stmt {
                ast::Stmt::Expr(expr) => (expr, false),
                ast::Stmt::Semi(expr, _) => (expr, true),
                _ => continue,
            };

            if let Some((expr, _)) = std::mem::replace(&mut last, Some((expr, term))) {
                let _ = self.eval(expr, used)?;
            }
        }

        if let Some((expr, term)) = last {
            let const_value = self.eval(expr, used)?;

            if !term {
                return Ok(const_value);
            }
        }

        Ok(ConstValue::Unit)
    }
}
