use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::traits::Compile;
use crate::CompileResult;
use crate::Spanned as _;
use runestick::Inst;

/// Compile a literal unit `()`.
impl Compile<(&ast::LitUnit, Needs)> for Compiler<'_> {
    fn compile(&mut self, (lit_unit, needs): (&ast::LitUnit, Needs)) -> CompileResult<()> {
        let span = lit_unit.span();
        log::trace!("LitUnit => {:?}", self.source.source(span));

        // If the value is not needed, no need to encode it.
        if !needs.value() {
            return Ok(());
        }

        self.asm.push(Inst::unit(), span);
        Ok(())
    }
}
