use crate::compiling::compile::prelude::*;

/// Compile a `yield` expression.
impl Compile2 for ast::ExprYield {
    fn compile2(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprYield => {:?}", c.source.source(span));

        if let Some(expr) = &self.expr {
            expr.compile2(c, Needs::Value)?;
            c.asm.push(Inst::Yield, span);
        } else {
            c.asm.push(Inst::YieldUnit, span);
        }

        if !needs.value() {
            c.asm.push(Inst::Pop, span);
        }

        Ok(())
    }
}
