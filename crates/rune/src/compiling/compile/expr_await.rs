use crate::compiling::compile::prelude::*;

/// Compile an `.await` expression.
impl Compile2 for ast::ExprAwait {
    fn compile2(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprAwait => {:?}", c.source.source(span));

        self.expr.compile2(c, Needs::Value)?;
        c.asm.push(Inst::Await, span);

        if !needs.value() {
            c.asm.push(Inst::Pop, span);
        }

        Ok(())
    }
}
