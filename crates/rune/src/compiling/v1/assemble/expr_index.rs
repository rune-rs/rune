use crate::compiling::v1::assemble::prelude::*;

/// Compile an expression.
impl Assemble for ast::ExprIndex {
    fn assemble(&self, c: &mut Compiler<'_, '_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("ExprIndex => {:?}", c.q.sources.source(c.source_id, span));

        let guard = c.scopes.push_child(span)?;

        let target = self.target.assemble(c, Needs::Value)?.apply_targeted(c)?;
        let index = self.index.assemble(c, Needs::Value)?.apply_targeted(c)?;

        c.asm.push(Inst::IndexGet { index, target }, span);

        // NB: we still need to perform the operation since it might have side
        // effects, but pop the result in case a value is not needed.
        if !needs.value() {
            c.asm.push(Inst::Pop, span);
        }

        c.scopes.pop(guard, span)?;
        Ok(Asm::top(span))
    }
}
