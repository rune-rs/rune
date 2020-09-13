use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::error::CompileResult;
use crate::traits::Compile;
use runestick::Inst;

/// Compile a try expression.
impl Compile<(&ast::ExprTry, Needs)> for Compiler<'_> {
    fn compile(&mut self, (expr_try, needs): (&ast::ExprTry, Needs)) -> CompileResult<()> {
        let span = expr_try.span();
        log::trace!("ExprTry => {:?}", self.source.source(span));

        let not_error = self.asm.new_label("try_not_error");

        self.compile((&*expr_try.expr, Needs::Value))?;
        self.asm.push(Inst::Dup, span);
        self.asm.push(Inst::IsValue, span);
        self.asm.jump_if(not_error, span);

        // Clean up all locals so far and return from the current function.
        let total_var_count = self.scopes.total_var_count(span)?;
        self.locals_clean(total_var_count, span);
        self.asm.push(Inst::Return, span);

        self.asm.label(not_error)?;

        if needs.value() {
            self.asm.push(Inst::Unwrap, span);
        } else {
            self.asm.push(Inst::Pop, span);
        }

        Ok(())
    }
}
