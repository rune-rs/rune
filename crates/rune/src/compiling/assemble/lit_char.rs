use crate::compiling::assemble::prelude::*;

/// Compile a literal character.
impl Assemble for ast::LitChar {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("LitChar => {:?}", c.source.source(span));

        // NB: Elide the entire literal if it's not needed.
        if !needs.value() {
            c.warnings.not_used(c.source_id, span, c.context());
            return Ok(());
        }

        let ch = self.resolve(&c.storage, &*c.source)?;
        c.asm.push(Inst::char(ch), span);
        Ok(())
    }
}
