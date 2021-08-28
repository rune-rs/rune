use crate::compiling::v1::assemble::prelude::*;

/// Compile a literal byte such as `b'a'`.
impl Assemble for ast::LitByte {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("LitByte => {:?}", c.source.source(span));

        // NB: Elide the entire literal if it's not needed.
        if !needs.value() {
            c.diagnostics.not_used(c.source_id, span, c.context());
            return Ok(Asm::top(span));
        }

        let b = self.resolve(c.storage, &*c.source)?;
        c.asm.push(Inst::byte(b), span);
        Ok(Asm::top(span))
    }
}
