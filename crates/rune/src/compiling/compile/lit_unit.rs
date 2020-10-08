use crate::compiling::compile::prelude::*;

/// Compile a literal unit `()`.
impl Compile2 for ast::LitUnit {
    fn compile2(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("LitUnit => {:?}", c.source.source(span));

        // If the value is not needed, no need to encode it.
        if !needs.value() {
            return Ok(());
        }

        c.asm.push(Inst::unit(), span);
        Ok(())
    }
}
