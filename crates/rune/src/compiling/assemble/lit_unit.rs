use crate::compiling::assemble::prelude::*;

/// Compile a literal unit `()`.
impl Assemble for ast::LitUnit {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("LitUnit => {:?}", c.source.source(span));

        // If the value is not needed, no need to encode it.
        if !needs.value() {
            c.warnings.not_used(c.source_id, span, c.context());
            return Ok(());
        }

        c.asm.push(Inst::unit(), span);
        Ok(())
    }
}
