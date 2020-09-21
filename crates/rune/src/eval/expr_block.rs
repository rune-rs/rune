use crate::eval::prelude::*;

impl Eval<&ast::ExprBlock> for ConstCompiler<'_> {
    fn eval(&mut self, expr_block: &ast::ExprBlock, used: Used) -> Result<ConstValue, EvalOutcome> {
        self.eval(&expr_block.block, used)
    }
}
