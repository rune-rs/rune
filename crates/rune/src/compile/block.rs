use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::traits::Compile;
use crate::CompileResult;
use crate::{CompileError, Spanned as _};
use runestick::{CompileMetaCapture, Inst};

/// Compile the async block.
impl Compile<(&ast::Block, &[CompileMetaCapture])> for Compiler<'_> {
    fn compile(
        &mut self,
        (block, captures): (&ast::Block, &[CompileMetaCapture]),
    ) -> CompileResult<()> {
        let span = block.span();
        log::trace!("ExprBlock (procedure) => {:?}", self.source.source(span));

        let guard = self.scopes.push_child(span)?;

        for capture in captures {
            self.scopes.new_var(&capture.ident, span)?;
        }

        self.compile((block, Needs::Value))?;
        self.clean_last_scope(span, guard, Needs::Value)?;
        self.asm.push(Inst::Return, span);
        Ok(())
    }
}

/// Call a block.
impl Compile<(&ast::Block, Needs)> for Compiler<'_> {
    fn compile(&mut self, (block, needs): (&ast::Block, Needs)) -> CompileResult<()> {
        let span = block.span();
        log::trace!("Block => {:?}", self.source.source(span));
        let _guard = self.items.push_block();

        self.contexts.push(span);
        let scopes_count = self.scopes.push_child(span)?;

        let mut last = None::<(&ast::Expr, bool)>;

        for stmt in &block.statements {
            let (expr, term) = match stmt {
                ast::Stmt::Expr(expr) => (expr, false),
                ast::Stmt::Semi(expr, _) => (expr, true),
                _ => continue,
            };

            if let Some((stmt, _)) = std::mem::replace(&mut last, Some((expr, term))) {
                // NB: terminated expressions do not need to produce a value.
                self.compile((stmt, Needs::None))?;
            }
        }

        let produced = if let Some((expr, term)) = last {
            if term {
                self.compile((expr, Needs::None))?;
                false
            } else {
                self.compile((expr, needs))?;
                true
            }
        } else {
            false
        };

        let scope = self.scopes.pop(scopes_count, span)?;

        if needs.value() {
            if produced {
                self.locals_clean(scope.local_var_count, span);
            } else {
                self.locals_pop(scope.local_var_count, span);
                self.asm.push(Inst::Unit, span);
            }
        } else {
            self.locals_pop(scope.local_var_count, span);
        }

        self.contexts
            .pop()
            .ok_or_else(|| CompileError::internal(span, "missing parent context"))?;

        Ok(())
    }
}
