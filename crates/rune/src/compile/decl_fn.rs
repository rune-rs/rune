use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::error::CompileResult;
use crate::traits::Compile;
use crate::{traits::Resolve as _, CompileError};
use runestick::Inst;

impl Compile<(ast::DeclFn, bool)> for Compiler<'_, '_> {
    fn compile(&mut self, (fn_decl, instance_fn): (ast::DeclFn, bool)) -> CompileResult<()> {
        let span = fn_decl.span();
        log::trace!("DeclFn => {:?}", self.source.source(span));
        let _guard = self.items.push_block();

        let mut first = true;

        for (arg, _) in fn_decl.args.items.iter() {
            let span = arg.span();

            match arg {
                ast::FnArg::Self_(s) => {
                    if !instance_fn || !first {
                        return Err(CompileError::UnsupportedSelf { span });
                    }

                    let span = s.span();
                    self.scopes.last_mut(span)?.new_var("self", span)?;
                }
                ast::FnArg::Ident(ident) => {
                    let span = ident.span();
                    let name = ident.resolve(self.source)?;
                    self.scopes.last_mut(span)?.new_var(name, span)?;
                }
                ast::FnArg::Ignore(ignore) => {
                    let span = ignore.span();
                    self.scopes.decl_anon(span)?;
                }
            }

            first = false;
        }

        if fn_decl.body.exprs.is_empty() && fn_decl.body.trailing_expr.is_none() {
            self.asm.push(Inst::ReturnUnit, span);
            return Ok(());
        }

        for (expr, _) in &fn_decl.body.exprs {
            self.compile((expr, Needs::None))?;
        }

        if let Some(expr) = &fn_decl.body.trailing_expr {
            self.compile((&**expr, Needs::Value))?;

            let total_var_count = self.scopes.last(span)?.total_var_count;
            self.locals_clean(total_var_count, span);
            self.asm.push(Inst::Return, span);
        } else {
            let total_var_count = self.scopes.last(span)?.total_var_count;
            self.locals_pop(total_var_count, span);
            self.asm.push(Inst::ReturnUnit, span);
        }

        self.scopes.pop_last(span)?;
        Ok(())
    }
}
