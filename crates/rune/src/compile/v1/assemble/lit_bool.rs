use crate::compile::v1::assemble::prelude::*;

/// Compile a literal boolean such as `true`.
impl Assemble for ast::LitBool {
    fn assemble(&self, c: &mut Compiler<'_, '_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("LitBool => {:?}", c.q.sources.source(c.source_id, span));

        // If the value is not needed, no need to encode it.
        if !needs.value() {
            c.diagnostics.not_used(c.source_id, span, c.context());
        } else {
            c.asm.push(Inst::bool(self.value), span);
        }

        Ok(Asm::top(span))
    }
}
