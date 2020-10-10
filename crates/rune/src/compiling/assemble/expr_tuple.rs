use crate::compiling::assemble::prelude::*;

/// Compile a literal tuple.
impl Assemble for ast::ExprTuple {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprTuple => {:?}", c.source.source(span));

        if self.items.is_empty() {
            c.asm.push(Inst::unit(), span);
        } else {
            for (expr, _) in &self.items {
                expr.assemble(c, Needs::Value)?;
                c.scopes.decl_anon(expr.span())?;
            }

            c.asm.push(
                Inst::Tuple {
                    count: self.items.len(),
                },
                span,
            );

            c.scopes.undecl_anon(span, self.items.len())?;
        }

        if !needs.value() {
            c.warnings.not_used(c.source_id, span, c.context());
            c.asm.push(Inst::Pop, span);
            return Ok(());
        }

        Ok(())
    }
}
