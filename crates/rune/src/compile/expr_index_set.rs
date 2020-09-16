use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::traits::Compile;
use crate::CompileResult;
use crate::Spanned as _;
use runestick::Inst;

/// An expr index set operation.
impl Compile<(&ast::ExprIndexSet, Needs)> for Compiler<'_> {
    fn compile(
        &mut self,
        (expr_index_set, needs): (&ast::ExprIndexSet, Needs),
    ) -> CompileResult<()> {
        let span = expr_index_set.span();
        log::trace!("ExprIndexSet => {:?}", self.source.source(span));

        self.compile((&*expr_index_set.value, Needs::Value))?;
        self.scopes.decl_anon(span)?;

        self.compile((&*expr_index_set.target, Needs::Value))?;
        self.scopes.decl_anon(span)?;

        self.compile((&*expr_index_set.index, Needs::Value))?;
        self.scopes.decl_anon(span)?;

        self.asm.push(Inst::IndexSet, span);
        self.scopes.undecl_anon(3, span)?;

        // Encode a unit in case a value is needed.
        if needs.value() {
            self.asm.push(Inst::unit(), span);
        }

        Ok(())
    }
}
