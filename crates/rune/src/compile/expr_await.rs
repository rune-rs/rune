use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::traits::Compile;
use crate::CompileResult;
use crate::Spanned as _;
use runestick::Inst;

/// Compile an `.await` expression.
impl Compile<(&ast::ExprAwait, Needs)> for Compiler<'_> {
    fn compile(&mut self, (expr_await, needs): (&ast::ExprAwait, Needs)) -> CompileResult<()> {
        let span = expr_await.span();
        log::trace!("ExprAwait => {:?}", self.source.source(span));

        self.compile((&*expr_await.expr, Needs::Value))?;
        self.asm.push(Inst::Await, span);

        if !needs.value() {
            self.asm.push(Inst::Pop, span);
        }

        Ok(())
    }
}
