use crate::compiling::compile::prelude::*;

/// Compile an expression.
impl Compile2 for ast::ExprIndex {
    fn compile2(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprIndex => {:?}", c.source.source(span));

        let guard = c.scopes.push_child(span)?;

        self.target.compile2(c, Needs::Value)?;
        c.scopes.decl_anon(span)?;

        self.index.compile2(c, Needs::Value)?;
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
