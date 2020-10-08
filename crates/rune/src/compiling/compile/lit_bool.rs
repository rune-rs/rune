use crate::compiling::compile::prelude::*;

/// Compile a literal boolean such as `true`.
impl Compile2 for ast::LitBool {
    fn compile2(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("LitBool => {:?}", c.source.source(span));

        // If the value is not needed, no need to encode it.
        if needs.value() {
            c.asm.push(Inst::bool(self.value), span);
        }

        Ok(())
    }
}
