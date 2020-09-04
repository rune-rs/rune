use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::error::CompileResult;
use crate::traits::{Compile, Resolve as _};
use runestick::Inst;

/// Compile a literal character.
impl Compile<(&ast::LitChar, Needs)> for Compiler<'_, '_> {
    fn compile(&mut self, (lit_char, needs): (&ast::LitChar, Needs)) -> CompileResult<()> {
        let span = lit_char.span();
        log::trace!("LitChar => {:?}", self.source.source(span)?);

        // NB: Elide the entire literal if it's not needed.
        if !needs.value() {
            self.warnings.not_used(span, self.context());
            return Ok(());
        }

        let resolved_char = lit_char.resolve(self.source)?;
        self.asm.push(Inst::Char { c: resolved_char }, span);
        Ok(())
    }
}
