use crate::compiling::assemble::prelude::*;

impl AssembleFn for ast::ItemFn {
    fn assemble_fn(&self, c: &mut Compiler<'_>, instance_fn: bool) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ItemFn => {:?}", c.source.source(span));

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
                ast::FnArg::Ident(ident) => {
                    let span = ident.span();
                    let name = ident.resolve(&c.storage, &*c.source)?;
                    c.scopes.new_var(name.as_ref(), span)?;
                }
                ast::FnArg::Ignore(ignore) => {
                    let span = ignore.span();
                    c.scopes.decl_anon(span)?;
                }
            }

            first = false;
        }

        if self.body.statements.is_empty() {
            let total_var_count = c.scopes.total_var_count(span)?;
            c.locals_pop(total_var_count, span);
            c.asm.push(Inst::ReturnUnit, span);
            return Ok(());
        }

        if !self.body.produces_nothing() {
            self.body.assemble(c, Needs::Value)?;

            let total_var_count = c.scopes.total_var_count(span)?;
            c.locals_clean(total_var_count, span);
            c.asm.push(Inst::Return, span);
        } else {
            self.body.assemble(c, Needs::None)?;

            let total_var_count = c.scopes.total_var_count(span)?;
            c.locals_pop(total_var_count, span);
            c.asm.push(Inst::ReturnUnit, span);
        }

        c.scopes.pop_last(span)?;
        Ok(())
    }
}
