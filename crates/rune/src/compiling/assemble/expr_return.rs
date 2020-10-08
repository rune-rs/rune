use crate::compiling::assemble::prelude::*;

/// Compile a return.
impl Assemble for ast::ExprReturn {
    fn assemble(&self, c: &mut Compiler<'_>, _: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprReturn => {:?}", c.source.source(span));

        // NB: drop any loop temporaries.
        for l in c.loops.iter() {
            if let Some(offset) = l.drop {
                c.asm.push(Inst::Drop { offset }, span);
            }
        }

        // NB: we actually want total_var_count here since we need to clean up
        // _every_ variable declared until we reached the current return.
        let total_var_count = c.scopes.total_var_count(span)?;

        if let Some(expr) = &self.expr {
            expr.assemble(c, Needs::Value)?;
            c.locals_clean(total_var_count, span);
            c.asm.push(Inst::Return, span);
        } else {
            c.locals_pop(total_var_count, span);
            c.asm.push(Inst::ReturnUnit, span);
        }

        Ok(())
    }
}
