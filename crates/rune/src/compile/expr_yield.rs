use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::traits::Compile;
use crate::CompileResult;
use crate::Spanned as _;
use runestick::Inst;

/// Compile a `yield` expression.
impl Compile<(&ast::ExprYield, Needs)> for Compiler<'_> {
    fn compile(&mut self, (expr_yield, needs): (&ast::ExprYield, Needs)) -> CompileResult<()> {
        let span = expr_yield.span();
        log::trace!("ExprYield => {:?}", self.source.source(span));

        if let Some(expr) = &expr_yield.expr {
            self.compile((&**expr, Needs::Value))?;
            self.asm.push(Inst::Yield, span);
        } else {
            self.asm.push(Inst::YieldUnit, span);
        }

        if !needs.value() {
            self.asm.push(Inst::Pop, span);
        }

        Ok(())
    }
}
