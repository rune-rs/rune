use crate::compiling::compile::prelude::*;

/// Compile a literal string `b"Hello World"`.
impl Compile2 for ast::LitByteStr {
    fn compile2(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("LitByteStr => {:?}", c.source.source(span));

        // NB: Elide the entire literal if it's not needed.
        if !needs.value() {
            c.warnings.not_used(c.source_id, span, c.context());
            return Ok(());
        }

        let bytes = self.resolve(&c.storage, &*c.source)?;
        let slot = c.unit.new_static_bytes(span, &*bytes)?;
        c.asm.push(Inst::Bytes { slot }, span);
        Ok(())
    }
}
