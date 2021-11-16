use crate::compile::v1::assemble::prelude::*;

/// Compile a literal character.
impl Assemble for ast::LitChar {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("LitChar => {:?}", c.q.sources.source(c.source_id, span));

        // NB: Elide the entire literal if it's not needed.
        if !needs.value() {
            c.diagnostics.not_used(c.source_id, span, c.context());
            return Ok(Asm::top(span));
        }

        let ch = self.resolve(c.q.storage(), c.q.sources)?;
        c.asm.push(Inst::char(ch), span);
        Ok(Asm::top(span))
    }
}
