use crate::compiling::compile::prelude::*;

/// Compile a literal byte such as `b'a'`.
impl Compile<(&ast::LitByte, Needs)> for Compiler<'_> {
    fn compile(&mut self, (lit_byte, needs): (&ast::LitByte, Needs)) -> CompileResult<()> {
        let span = lit_byte.span();
        log::trace!("LitByte => {:?}", self.source.source(span));

        // NB: Elide the entire literal if it's not needed.
        if !needs.value() {
            self.warnings.not_used(self.source_id, span, self.context());
            return Ok(());
        }

        let b = lit_byte.resolve(&self.storage, &*self.source)?;
        self.asm.push(Inst::byte(b), span);
        Ok(())
    }
}
