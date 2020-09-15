use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::traits::Compile;
use crate::CompileResult;
use crate::Spanned as _;
use runestick::Inst;

/// Compile a return.
impl Compile<(&ast::ExprReturn, Needs)> for Compiler<'_> {
    fn compile(&mut self, (return_expr, _needs): (&ast::ExprReturn, Needs)) -> CompileResult<()> {
        let span = return_expr.span();
        log::trace!("ExprReturn => {:?}", self.source.source(span));

        // NB: drop any loop temporaries.
        for l in self.loops.iter() {
            if let Some(offset) = l.drop {
                self.asm.push(Inst::Drop { offset }, span);
            }
        }

        // NB: we actually want total_var_count here since we need to clean up
        // _every_ variable declared until we reached the current return.
        let total_var_count = self.scopes.total_var_count(span)?;

        if let Some(expr) = &return_expr.expr {
            self.compile((&**expr, Needs::Value))?;
            self.locals_clean(total_var_count, span);
            self.asm.push(Inst::Return, span);
        } else {
            self.locals_pop(total_var_count, span);
            self.asm.push(Inst::ReturnUnit, span);
        }

        Ok(())
    }
}
