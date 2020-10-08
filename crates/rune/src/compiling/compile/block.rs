use crate::compiling::compile::prelude::*;

/// Compile an async block.
impl Compile2 for (&ast::Block, &[CompileMetaCapture]) {
    fn compile2(&self, c: &mut Compiler<'_>, _: Needs) -> CompileResult<()> {
        let (block, captures) = *self;

        let span = block.span();
        log::trace!("ExprBlock (procedure) => {:?}", c.source.source(span));

        let guard = c.scopes.push_child(span)?;

        for capture in captures {
            c.scopes.new_var(&capture.ident, span)?;
        }

        block.compile2(c, Needs::Value)?;
        c.clean_last_scope(span, guard, Needs::Value)?;
        c.asm.push(Inst::Return, span);
        Ok(())
    }
}

/// Call a block.
impl Compile2 for ast::Block {
    fn compile2(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("Block => {:?}", c.source.source(span));

        c.contexts.push(span);
        let scopes_count = c.scopes.push_child(span)?;

        let mut last = None::<(&ast::Expr, bool)>;

        for stmt in &self.statements {
            let (expr, term) = match stmt {
                ast::Stmt::Local(local) => {
                    local.compile2(c, Needs::None)?;
                    continue;
                }
                ast::Stmt::Expr(expr) => (expr, false),
                ast::Stmt::Semi(expr, _) => (expr, true),
                ast::Stmt::Item(..) => continue,
            };

            if let Some((stmt, _)) = std::mem::replace(&mut last, Some((expr, term))) {
                // NB: terminated expressions do not need to produce a value.
                stmt.compile2(c, Needs::None)?;
            }
        }

        let produced = if let Some((expr, term)) = last {
            if term {
                expr.compile2(c, Needs::None)?;
                false
            } else {
                expr.compile2(c, needs)?;
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
            .ok_or_else(|| CompileError::internal(&span, "missing parent context"))?;

        Ok(())
    }
}
