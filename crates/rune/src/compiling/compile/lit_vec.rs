use crate::compiling::compile::prelude::*;

/// Compile a literal vector.
impl Compile<(&ast::LitVec, Needs)> for Compiler<'_> {
    fn compile(&mut self, (lit_vec, needs): (&ast::LitVec, Needs)) -> CompileResult<()> {
        let span = lit_vec.span();
        log::trace!("LitVec => {:?}", self.source.source(span));

        let count = lit_vec.items.len();

        for (expr, _) in &lit_vec.items {
            self.compile((expr, Needs::Value))?;
            self.scopes.decl_anon(expr.span())?;
        }

        self.asm.push(Inst::Vec { count }, span);
        self.scopes.undecl_anon(span, lit_vec.items.len())?;

        // Evaluate the expressions one by one, then pop them to cause any
        // side effects (without creating an object).
        if !needs.value() {
            self.asm.push(Inst::Pop, span);
        }

        Ok(())
    }
}
