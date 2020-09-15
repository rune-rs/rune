use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::traits::Compile;
use crate::CompileResult;
use crate::Spanned as _;
use runestick::Inst;

/// Compile a literal vector.
impl Compile<(&ast::LitVec, Needs)> for Compiler<'_> {
    fn compile(&mut self, (lit_vec, needs): (&ast::LitVec, Needs)) -> CompileResult<()> {
        let span = lit_vec.span();
        log::trace!("LitVec => {:?}", self.source.source(span));

        if !needs.value() && lit_vec.is_const() {
            // Don't encode unecessary literals.
            return Ok(());
        }

        let count = lit_vec.items.len();

        for expr in lit_vec.items.iter() {
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
