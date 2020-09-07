use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::error::CompileResult;
use crate::traits::Compile;
use crate::{traits::Resolve as _, CompileError};
use runestick::{CompileMeta, CompileMetaCapture, Hash, Inst};

/// Compile the body of a closure function.
impl Compile<(ast::ExprClosure, &[CompileMetaCapture])> for Compiler<'_> {
    fn compile(
        &mut self,
        (expr_closure, captures): (ast::ExprClosure, &[CompileMetaCapture]),
    ) -> CompileResult<()> {
        let span = expr_closure.span();
        log::trace!("ExprClosure => {:?}", self.source.source(span));

        let count = {
            let scope = self.scopes.last_mut(span)?;

            for (arg, _) in expr_closure.args.as_slice() {
                let span = arg.span();

                match arg {
                    ast::FnArg::Self_(s) => {
                        return Err(CompileError::UnsupportedSelf { span: s.span() })
                    }
                    ast::FnArg::Ident(ident) => {
                        let ident = ident.resolve(&self.storage, &*self.source)?;
                        scope.new_var(ident.as_ref(), span)?;
                    }
                    ast::FnArg::Ignore(..) => {
                        // Ignore incoming variable.
                        let _ = scope.decl_anon(span);
                    }
                }
            }

            if !captures.is_empty() {
                self.asm.push(Inst::PushTuple, span);

                for capture in captures {
                    scope.new_var(&capture.ident, span)?;
                }
            }

            scope.total_var_count
        };

        self.compile((&*expr_closure.body, Needs::Value))?;

        if count != 0 {
            self.asm.push(Inst::Clean { count }, span);
        }

        self.asm.push(Inst::Return, span);

        self.scopes.pop_last(span)?;
        Ok(())
    }
}

/// Compile a closure expression.
impl Compile<(&ast::ExprClosure, Needs)> for Compiler<'_> {
    fn compile(&mut self, (expr_closure, needs): (&ast::ExprClosure, Needs)) -> CompileResult<()> {
        let span = expr_closure.span();
        log::trace!("ExprClosure => {:?}", self.source.source(span));

        if !needs.value() {
            self.warnings.not_used(self.source_id, span, self.context());
            return Ok(());
        }

        let _guard = self.items.push_closure();
        let item = self.items.item();
        let hash = Hash::type_hash(&item);

        let meta =
            self.query
                .query_meta(&item, span)?
                .ok_or_else(|| CompileError::MissingType {
                    item: item.clone(),
                    span,
                })?;

        let captures = match meta {
            CompileMeta::Closure { captures, .. } => captures,
            meta => {
                return Err(CompileError::UnsupportedMetaClosure { meta, span });
            }
        };

        log::trace!("captures: {} => {:?}", item, captures);

        if captures.is_empty() {
            // NB: if closure doesn't capture the environment it acts like a regular
            // function. No need to store and load the environment.
            self.asm
                .push_with_comment(Inst::Fn { hash }, span, format!("closure `{}`", item));
        } else {
            // Construct a closure environment.
            for capture in &*captures {
                let var = self.scopes.get_var(&capture.ident, span)?;
                var.copy(&mut self.asm, span, format!("capture `{}`", capture.ident));
            }

            self.asm.push_with_comment(
                Inst::Closure {
                    hash,
                    count: captures.len(),
                },
                span,
                format!("closure `{}`", item),
            );
        }

        Ok(())
    }
}
