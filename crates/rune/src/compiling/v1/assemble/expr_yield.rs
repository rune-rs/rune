use crate::compiling::v1::assemble::prelude::*;

/// Compile a `yield` expression.
impl Assemble for ast::ExprYield {
    fn assemble(&self, c: &mut Compiler<'_, '_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("ExprYield => {:?}", c.source.source(span));

        if let Some(expr) = &self.expr {
            expr.assemble(c, Needs::Value)?.apply(c)?;
            c.asm.push(Inst::Yield, span);
        } else {
            c.asm.push(Inst::YieldUnit, span);
        }

        if !needs.value() {
            c.asm.push(Inst::Pop, span);
        }

        Ok(Asm::top(span))
    }
}
