use crate::compiling::compile::prelude::*;

/// Compile a literal vector.
impl Compile<(&ast::LitVec, Needs)> for Compiler<'_> {
    fn compile(&mut self, (lit_vec, needs): (&ast::LitVec, Needs)) -> CompileResult<()> {
        let span = lit_vec.span();
        log::trace!("LitVec => {:?}", self.source.source(span));

        let count = lit_vec.items.len();

        for (expr, _) in &lit_vec.items {
            self.compile((expr, Needs::Value))?;

            // Evaluate the expressions one by one, then pop them to cause any
            // side effects (without creating an object).
            if !needs.value() {
                self.asm.push(Inst::Pop, span);
            }
        }

        // No need to create a vector if it's not needed.
        if !needs.value() {
            self.warnings.not_used(self.source_id, span, self.context());
            return Ok(());
        }

        self.asm.push(Inst::Vec { count }, span);
        Ok(())
    }
}
