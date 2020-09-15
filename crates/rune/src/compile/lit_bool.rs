use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::traits::Compile;
use crate::CompileResult;
use crate::Spanned as _;
use runestick::Inst;

/// Compile a literal boolean such as `true`.
impl Compile<(&ast::LitBool, Needs)> for Compiler<'_> {
    fn compile(&mut self, (lit_bool, needs): (&ast::LitBool, Needs)) -> CompileResult<()> {
        let span = lit_bool.span();
        log::trace!("LitBool => {:?}", self.source.source(span));

        // If the value is not needed, no need to encode it.
        if !needs.value() {
            return Ok(());
        }

        self.asm.push(
            Inst::Bool {
                value: lit_bool.value,
            },
            span,
        );

        Ok(())
    }
}
