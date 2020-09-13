use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::error::CompileResult;
use crate::traits::Compile;

/// Compile `self`.
impl Compile<(&ast::Self_, Needs)> for Compiler<'_> {
    fn compile(&mut self, (self_, needs): (&ast::Self_, Needs)) -> CompileResult<()> {
        let span = self_.span();
        log::trace!("Self_ => {:?}", self.source.source(span));
        let var = self.scopes.get_var("self", self.visitor, span)?;

        if !needs.value() {
            return Ok(());
        }

        var.copy(&mut self.asm, span, "self");
        Ok(())
    }
}
