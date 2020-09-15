use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::traits::Compile;
use crate::CompileResult;
use crate::Spanned as _;
use runestick::Inst;

/// Compile an expression.
impl Compile<(&ast::ExprIndexGet, Needs)> for Compiler<'_> {
    fn compile(
        &mut self,
        (expr_index_get, needs): (&ast::ExprIndexGet, Needs),
    ) -> CompileResult<()> {
        let span = expr_index_get.span();
        log::trace!("ExprIndexGet => {:?}", self.source.source(span));

        let guard = self.scopes.push_child(span)?;

        self.compile((&*expr_index_get.index, Needs::Value))?;
        self.scopes.decl_anon(span)?;

        self.compile((&*expr_index_get.target, Needs::Value))?;
        self.scopes.decl_anon(span)?;

        self.asm.push(Inst::IndexGet, span);

        // NB: we still need to perform the operation since it might have side
        // effects, but pop the result in case a value is not needed.
        if !needs.value() {
            self.asm.push(Inst::Pop, span);
        }

        self.scopes.pop(guard, span)?;
        Ok(())
    }
}
