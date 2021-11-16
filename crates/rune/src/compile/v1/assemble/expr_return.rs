use crate::compile::v1::assemble::prelude::*;

/// Compile a return.
impl Assemble for ast::ExprReturn {
    fn assemble(&self, c: &mut Compiler<'_>, _: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("ExprReturn => {:?}", c.q.sources.source(c.source_id, span));

        // NB: drop any loop temporaries.
        for l in c.loops.iter() {
            if let Some(offset) = l.drop {
                c.asm.push(Inst::Drop { offset }, span);
            }
        }

        if let Some(expr) = &self.expr {
            c.return_(span, expr)?;
        } else {
            // NB: we actually want total_var_count here since we need to clean up
            // _every_ variable declared until we reached the current return.
            let clean = c.scopes.total_var_count(span)?;
            c.locals_pop(clean, span);
            c.asm.push(Inst::ReturnUnit, span);
        }

        Ok(Asm::top(span))
    }
}
