use crate::compiling::v1::assemble::prelude::*;

/// Compile an async block.
impl AssembleClosure for ast::Block {
    fn assemble_closure(
        &self,
        c: &mut Compiler<'_>,
        captures: &[CompileMetaCapture],
    ) -> CompileResult<()> {
        let span = self.span();
        log::trace!("Block (closure) => {:?}", c.source.source(span));

        let guard = c.scopes.push_child(span)?;

        for capture in captures {
            c.scopes.new_var(&capture.ident, span)?;
        }

        c.return_(span, self)?;
        c.scopes.pop(guard, span)?;
        Ok(())
    }
}

/// Call a block.
impl Assemble for ast::Block {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("Block => {:?}", c.source.source(span));

        c.contexts.push(span);
        let scopes_count = c.scopes.push_child(span)?;

        let mut last = None::<(&ast::Expr, bool)>;

        for stmt in &self.statements {
            let (expr, term) = match stmt {
                ast::Stmt::Local(local) => {
                    if let Some((stmt, _)) = std::mem::take(&mut last) {
                        // NB: terminated expressions do not need to produce a value.
                        stmt.assemble(c, Needs::None)?.apply(c)?;
                    }

                    local.assemble(c, Needs::None)?.apply(c)?;
                    continue;
                }
                ast::Stmt::Expr(expr, semi) => (expr, semi.is_some()),
                ast::Stmt::Item(..) => continue,
            };

            if let Some((stmt, _)) = std::mem::replace(&mut last, Some((expr, term))) {
                // NB: terminated expressions do not need to produce a value.
                stmt.assemble(c, Needs::None)?.apply(c)?;
            }
        }

        let produced = if let Some((expr, term)) = last {
            if term {
                expr.assemble(c, Needs::None)?.apply(c)?;
                false
            } else {
                expr.assemble(c, needs)?.apply(c)?;
                true
            }
        } else {
            false
        };

        let scope = c.scopes.pop(scopes_count, span)?;

        if needs.value() {
            if produced {
                c.locals_clean(scope.local_var_count, span);
            } else {
                c.locals_pop(scope.local_var_count, span);
                c.asm.push(Inst::unit(), span);
            }
        } else {
            c.locals_pop(scope.local_var_count, span);
        }

        c.contexts
            .pop()
            .ok_or_else(|| CompileError::msg(&span, "missing parent context"))?;

        Ok(Asm::top(span))
    }
}
