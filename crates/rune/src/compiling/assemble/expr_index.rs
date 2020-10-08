use crate::compiling::assemble::prelude::*;

/// Compile an expression.
impl Assemble for ast::ExprIndex {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprIndex => {:?}", c.source.source(span));

        let guard = c.scopes.push_child(span)?;

        self.target.assemble(c, Needs::Value)?;
        c.scopes.decl_anon(span)?;

        self.index.assemble(c, Needs::Value)?;
        c.scopes.decl_anon(span)?;

        c.asm.push(Inst::IndexGet, span);
        c.scopes.undecl_anon(span, 2)?;

        // NB: we still need to perform the operation since it might have side
        // effects, but pop the result in case a value is not needed.
        if !needs.value() {
            c.asm.push(Inst::Pop, span);
        }

        c.scopes.pop(guard, span)?;
        Ok(())
    }
}
