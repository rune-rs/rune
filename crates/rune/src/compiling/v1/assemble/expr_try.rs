use crate::compiling::v1::assemble::prelude::*;

/// Compile a try expression.
impl Assemble for ast::ExprTry {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("ExprTry => {:?}", c.source.source(span));

        let clean = c.scopes.total_var_count(span)?;
        let address = self.expr.assemble(c, Needs::Value)?.apply_targeted(c)?;
        c.asm.push(
            Inst::Try {
                address,
                clean,
                preserve: needs.value(),
            },
            span,
        );

        Ok(Asm::top(span))
    }
}
