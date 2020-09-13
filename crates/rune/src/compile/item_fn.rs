use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::error::CompileResult;
use crate::traits::Compile;
use crate::{traits::Resolve as _, CompileError};
use runestick::Inst;

impl Compile<(ast::ItemFn, bool)> for Compiler<'_> {
    fn compile(&mut self, (fn_decl, instance_fn): (ast::ItemFn, bool)) -> CompileResult<()> {
        let span = fn_decl.span();
        log::trace!("ItemFn => {:?}", self.source.source(span));

        let mut first = true;

        for (arg, _) in fn_decl.args.items.iter() {
            let span = arg.span();

            match arg {
                ast::FnArg::Self_(s) => {
                    if !instance_fn || !first {
                        return Err(CompileError::UnsupportedSelf { span });
                    }

                    let span = s.span();
                    self.scopes.new_var("self", span)?;
                }
                ast::FnArg::Ident(ident) => {
                    let span = ident.span();
                    let name = ident.resolve(&self.storage, &*self.source)?;
                    self.scopes.new_var(name.as_ref(), span)?;
                }
                ast::FnArg::Ignore(ignore) => {
                    let span = ignore.span();
                    self.scopes.decl_anon(span)?;
                }
            }

            first = false;
        }

        if fn_decl.body.statements.is_empty() {
            let total_var_count = self.scopes.total_var_count(span)?;
            self.locals_pop(total_var_count, span);
            self.asm.push(Inst::ReturnUnit, span);
            return Ok(());
        }

        if !fn_decl.body.produces_nothing() {
            self.compile((&fn_decl.body, Needs::Value))?;

            let total_var_count = self.scopes.total_var_count(span)?;
            self.locals_clean(total_var_count, span);
            self.asm.push(Inst::Return, span);
        } else {
            self.compile((&fn_decl.body, Needs::None))?;

            let total_var_count = self.scopes.total_var_count(span)?;
            self.locals_pop(total_var_count, span);
            self.asm.push(Inst::ReturnUnit, span);
        }

        self.scopes.pop_last(span)?;
        Ok(())
    }
}
