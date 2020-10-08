use crate::compiling::assemble::prelude::*;

impl Assemble for (&ast::ItemFn, bool) {
    fn assemble(&self, c: &mut Compiler<'_>, _: Needs) -> CompileResult<()> {
        let (item_fn, instance_fn) = *self;

        let span = item_fn.span();
        log::trace!("ItemFn => {:?}", c.source.source(span));

        let mut first = true;

        for (arg, _) in &item_fn.args {
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

        if item_fn.body.statements.is_empty() {
            let total_var_count = c.scopes.total_var_count(span)?;
            c.locals_pop(total_var_count, span);
            c.asm.push(Inst::ReturnUnit, span);
            return Ok(());
        }

        if !item_fn.body.produces_nothing() {
            item_fn.body.assemble(c, Needs::Value)?;

            let total_var_count = c.scopes.total_var_count(span)?;
            c.locals_clean(total_var_count, span);
            c.asm.push(Inst::Return, span);
        } else {
            item_fn.body.assemble(c, Needs::None)?;

            let total_var_count = c.scopes.total_var_count(span)?;
            c.locals_pop(total_var_count, span);
            c.asm.push(Inst::ReturnUnit, span);
        }

        c.scopes.pop_last(span)?;
        Ok(())
    }
}
