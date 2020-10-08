use crate::compiling::assemble::prelude::*;

/// Compile the body of a closure function.
impl Assemble for (&ast::ExprClosure, &[CompileMetaCapture]) {
    fn assemble(&self, c: &mut Compiler<'_>, _: Needs) -> CompileResult<()> {
        let (expr_closure, captures) = *self;

        let span = expr_closure.span();
        log::trace!("ExprClosure => {:?}", c.source.source(span));

        let count = {
            for (arg, _) in expr_closure.args.as_slice() {
                let span = arg.span();

                match arg {
                    ast::FnArg::SelfValue(s) => {
                        return Err(CompileError::new(s, CompileErrorKind::UnsupportedSelf))
                    }
                    ast::FnArg::Ident(ident) => {
                        let ident = ident.resolve(&c.storage, &*c.source)?;
                        c.scopes.new_var(ident.as_ref(), span)?;
                    }
                    ast::FnArg::Ignore(..) => {
                        // Ignore incoming variable.
                        let _ = c.scopes.decl_anon(span)?;
                    }
                }
            }

            if !captures.is_empty() {
                c.asm.push(Inst::PushTuple, span);

                for capture in captures {
                    c.scopes.new_var(&capture.ident, span)?;
                }
            }

            c.scopes.total_var_count(span)?
        };

        expr_closure.body.assemble(c, Needs::Value)?;

        if count != 0 {
            c.asm.push(Inst::Clean { count }, span);
        }

        c.asm.push(Inst::Return, span);

        c.scopes.pop_last(span)?;
        Ok(())
    }
}

/// Compile a closure expression.
impl Assemble for ast::ExprClosure {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprClosure => {:?}", c.source.source(span));

        if !needs.value() {
            c.warnings.not_used(c.source_id, span, c.context());
            return Ok(());
        }

        let item = c.query.item_for(self)?;
        let hash = Hash::type_hash(&item.item);

        let meta = c
            .query
            .query_meta_with(span, &item, Default::default())?
            .ok_or_else(|| {
                CompileError::new(
                    span,
                    CompileErrorKind::MissingType {
                        item: item.item.clone(),
                    },
                )
            })?;

        let captures = match &meta.kind {
            CompileMetaKind::Closure { captures, .. } => captures,
            _ => {
                return Err(CompileError::expected_meta(span, meta, "a closure"));
            }
        };

        log::trace!("captures: {} => {:?}", item.item, captures);

        if captures.is_empty() {
            // NB: if closure doesn't capture the environment it acts like a regular
            // function. No need to store and load the environment.
            c.asm.push_with_comment(
                Inst::LoadFn { hash },
                span,
                format!("closure `{}`", item.item),
            );
        } else {
            // Construct a closure environment.
            for capture in &**captures {
                let var = c
                    .scopes
                    .get_var(&capture.ident, c.source_id, c.visitor, span)?;
                var.copy(&mut c.asm, span, format!("capture `{}`", capture.ident));
            }

            c.asm.push_with_comment(
                Inst::Closure {
                    hash,
                    count: captures.len(),
                },
                span,
                format!("closure `{}`", item.item),
            );
        }

        Ok(())
    }
}
