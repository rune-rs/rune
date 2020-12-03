use crate::compiling::assemble::prelude::*;

/// Compile a literal vector.
impl Assemble for ast::ExprVec {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("ExprVec => {:?}", c.source.source(span));

        let count = self.items.len();

        for (expr, _) in &self.items {
            expr.assemble(c, Needs::Value)?.apply(c)?;
            c.scopes.decl_anon(expr.span())?;
        }

        c.asm.push(Inst::Vec { count }, span);
        c.scopes.undecl_anon(span, self.items.len())?;

        // Evaluate the expressions one by one, then pop them to cause any
        // side effects (without creating an object).
        if !needs.value() {
            c.warnings.not_used(c.source_id, span, c.context());
            c.asm.push(Inst::Pop, span);
        }

        Ok(Asm::top(span))
    }
}
