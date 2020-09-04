use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::error::CompileResult;
use crate::traits::Compile;
use crate::CompileError;
use runestick::Inst;

/// Compile a block.
///
/// Blocks are special in that they do not produce a value unless there is
/// an item in them which does.
impl Compile<(&ast::ExprBlock, Needs)> for Compiler<'_, '_> {
    fn compile(&mut self, (block, needs): (&ast::ExprBlock, Needs)) -> CompileResult<()> {
        let span = block.span();

        if let Some(..) = block.async_ {
            return Err(CompileError::UnsupportedAsyncExpr { span: block.span() });
        }

        log::trace!("ExprBlock => {:?}", self.source.source(span)?);
        let _guard = self.items.push_block();

        self.contexts.push(span);

        let span = block.span();

        let new_scope = self.scopes.child(span)?;
        let scopes_count = self.scopes.push(new_scope);

        for (expr, _) in &block.exprs {
            // NB: terminated expressions do not need to produce a value.
            self.compile((expr, Needs::None))?;
        }

        if let Some(expr) = &block.trailing_expr {
            self.compile((&**expr, needs))?;
        }

        let scope = self.scopes.pop(scopes_count, span)?;

        if needs.value() {
            if block.trailing_expr.is_none() {
                self.locals_pop(scope.local_var_count, span);
                self.asm.push(Inst::Unit, span);
            } else {
                self.locals_clean(scope.local_var_count, span);
            }
        } else {
            self.locals_pop(scope.local_var_count, span);
        }

        self.contexts
            .pop()
            .ok_or_else(|| CompileError::internal("missing parent context", span))?;

        Ok(())
    }
}
