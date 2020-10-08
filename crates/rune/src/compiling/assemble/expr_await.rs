use crate::compiling::assemble::prelude::*;

/// Compile an `.await` expression.
impl Assemble for ast::ExprAwait {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprAwait => {:?}", c.source.source(span));

        self.expr.assemble(c, Needs::Value)?;
        c.asm.push(Inst::Await, span);

        if !needs.value() {
            c.asm.push(Inst::Pop, span);
        }

        Ok(())
    }
}
