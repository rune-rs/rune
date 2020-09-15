use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::traits::Compile;
use crate::CompileResult;
use crate::{Resolve as _, Spanned as _};
use runestick::Inst;

/// Compile a literal string `b"Hello World"`.
impl Compile<(&ast::LitByteStr, Needs)> for Compiler<'_> {
    fn compile(&mut self, (lit_byte_str, needs): (&ast::LitByteStr, Needs)) -> CompileResult<()> {
        let span = lit_byte_str.span();
        log::trace!("LitByteStr => {:?}", self.source.source(span));

        // NB: Elide the entire literal if it's not needed.
        if !needs.value() {
            self.warnings.not_used(self.source_id, span, self.context());
            return Ok(());
        }

        let bytes = lit_byte_str.resolve(&self.storage, &*self.source)?;
        let slot = self.unit.borrow_mut().new_static_bytes(&*bytes)?;
        self.asm.push(Inst::Bytes { slot }, span);
        Ok(())
    }
}
