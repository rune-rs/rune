use crate::compiling::v1::assemble::prelude::*;

/// Compile an `.await` expression.
impl Assemble for ast::ExprAwait {
    fn assemble(&self, c: &mut Compiler<'_, '_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("ExprAwait => {:?}", c.source.source(span));

        self.expr.assemble(c, Needs::Value)?.apply(c)?;
        c.asm.push(Inst::Await, span);

        if !needs.value() {
            c.asm.push(Inst::Pop, span);
        }

        Ok(Asm::top(span))
    }
}
