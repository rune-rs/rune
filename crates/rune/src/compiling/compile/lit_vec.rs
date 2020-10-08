use crate::compiling::compile::prelude::*;

/// Compile a literal vector.
impl Compile2 for ast::LitVec {
    fn compile2(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("LitVec => {:?}", c.source.source(span));

        let count = self.items.len();

        for (expr, _) in &self.items {
            expr.compile2(c, Needs::Value)?;
            c.scopes.decl_anon(expr.span())?;
        }

        c.asm.push(Inst::Vec { count }, span);
        c.scopes.undecl_anon(span, self.items.len())?;

        // Evaluate the expressions one by one, then pop them to cause any
        // side effects (without creating an object).
        if !needs.value() {
            c.asm.push(Inst::Pop, span);
        }

        Ok(())
    }
}
