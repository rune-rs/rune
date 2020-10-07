use crate::compiling::compile::prelude::*;

/// Compile a literal tuple.
impl Compile<(&ast::LitTuple, Needs)> for Compiler<'_> {
    fn compile(&mut self, (lit_tuple, needs): (&ast::LitTuple, Needs)) -> CompileResult<()> {
        let span = lit_tuple.span();
        log::trace!("LitTuple => {:?}", self.source.source(span));

        for (expr, _) in &lit_tuple.items {
            self.compile((expr, Needs::Value))?;
            self.scopes.decl_anon(expr.span())?;
        }

        self.asm.push(
            Inst::Tuple {
                count: lit_tuple.items.len(),
            },
            span,
        );

        self.scopes.undecl_anon(span, lit_tuple.items.len())?;

        if !needs.value() {
            self.warnings.not_used(self.source_id, span, self.context());
            self.asm.push(Inst::Pop, span);
            return Ok(());
        }

        Ok(())
    }
}
