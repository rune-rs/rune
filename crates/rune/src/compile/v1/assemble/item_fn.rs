use crate::compile::v1::assemble::prelude::*;

impl AssembleFn for ast::ItemFn {
    fn assemble_fn(&self, c: &mut Compiler<'_, '_>, instance_fn: bool) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ItemFn => {:?}", c.q.sources.source(c.source_id, span));

        let mut patterns = Vec::new();
        let mut first = true;

        for (arg, _) in &self.args {
            let span = arg.span();

            match arg {
                ast::FnArg::SelfValue(s) => {
                    if !instance_fn || !first {
                        return Err(CompileError::new(span, CompileErrorKind::UnsupportedSelf));
                    }

                    let span = s.span();
                    c.scopes.new_var("self", span)?;
                }
                ast::FnArg::Pat(pat) => {
                    let offset = c.scopes.decl_anon(pat.span())?;
                    patterns.push((pat, offset));
                }
            }

            first = false;
        }

        for (pat, offset) in patterns {
            c.compile_pat_offset(pat, offset)?;
        }

        if self.body.statements.is_empty() {
            let total_var_count = c.scopes.total_var_count(span)?;
            c.locals_pop(total_var_count, span);
            c.asm.push(Inst::ReturnUnit, span);
            return Ok(());
        }

        if !self.body.produces_nothing() {
            c.return_(span, &self.body)?;
        } else {
            self.body.assemble(c, Needs::None)?.apply(c)?;

            let total_var_count = c.scopes.total_var_count(span)?;
            c.locals_pop(total_var_count, span);
            c.asm.push(Inst::ReturnUnit, span);
        }

        c.scopes.pop_last(span)?;
        Ok(())
    }
}
