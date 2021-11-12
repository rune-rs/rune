use crate::compiling::v1::assemble::prelude::*;

/// Compile a literal string `"Hello World"`.
impl Assemble for ast::LitStr {
    fn assemble(&self, c: &mut Compiler<'_, '_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("LitStr => {:?}", c.source.source(span));

        // NB: Elide the entire literal if it's not needed.
        if !needs.value() {
            c.diagnostics.not_used(c.source_id, span, c.context());
            return Ok(Asm::top(span));
        }

        let string = self.resolve(c.query.storage(), c.sources)?;
        let slot = c.query.unit_mut().new_static_string(span, &*string)?;
        c.asm.push(Inst::String { slot }, span);
        Ok(Asm::top(span))
    }
}
