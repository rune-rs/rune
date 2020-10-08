use crate::compiling::assemble::prelude::*;

/// Compile a try expression.
impl Assemble for ast::ExprTry {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprTry => {:?}", c.source.source(span));

        let not_error = c.asm.new_label("try_not_error");

        self.expr.assemble(c, Needs::Value)?;
        c.asm.push(Inst::Dup, span);
        c.asm.push(Inst::IsValue, span);
        c.asm.jump_if(not_error, span);

        // Clean up all locals so far and return from the current function.
        let total_var_count = c.scopes.total_var_count(span)?;
        c.locals_clean(total_var_count, span);
        c.asm.push(Inst::Return, span);

        c.asm.label(not_error)?;

        if needs.value() {
            c.asm.push(Inst::Unwrap, span);
        } else {
            c.asm.push(Inst::Pop, span);
        }

        Ok(())
    }
}
