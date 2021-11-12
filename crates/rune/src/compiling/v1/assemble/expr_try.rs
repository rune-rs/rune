use crate::compiling::v1::assemble::prelude::*;

/// Compile a try expression.
impl Assemble for ast::ExprTry {
    fn assemble(&self, c: &mut Compiler<'_, '_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("ExprTry => {:?}", c.q.sources.source(c.source_id, span));

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

        if let InstAddress::Top = address {
            c.scopes.undecl_anon(span, 1)?;
        }

        // Why no needs.value() check here to declare another anonymous
        // variable? Because when these assembling functions were initially
        // implemented it was decided that the caller that indicates
        // Needs::Value is responsible for declaring any anonymous variables.
        //
        // TODO: This should probably change!

        Ok(Asm::top(span))
    }
}
