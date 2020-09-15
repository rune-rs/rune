use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::traits::Compile;
use crate::CompileResult;
use crate::{Resolve as _, Spanned as _};
use runestick::Inst;

/// Compile a literal number.
impl Compile<(&ast::LitNumber, Needs)> for Compiler<'_> {
    fn compile(&mut self, (lit_number, needs): (&ast::LitNumber, Needs)) -> CompileResult<()> {
        let span = lit_number.span();
        log::trace!("LitNumber => {:?}", self.source.source(span));

        // NB: don't encode unecessary literal.
        if !needs.value() {
            self.warnings.not_used(self.source_id, span, self.context());
            return Ok(());
        }

        let lit_number = lit_number.resolve(&self.storage, &*self.source)?;

        match lit_number {
            ast::Number::Float(number) => {
                self.asm.push(Inst::float(number), span);
            }
            ast::Number::Integer(number) => {
                self.asm.push(Inst::integer(number), span);
            }
        }

        Ok(())
    }
}
