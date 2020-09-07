use crate::ast;
use crate::compiler::Compiler;
use crate::error::CompileResult;
use crate::{traits::Compile, CompileError};
use runestick::Inst;

/// Compile a break expression.
///
/// NB: loops are expected to produce a value at the end of their expression.
impl Compile<&ast::ExprBreak> for Compiler<'_> {
    fn compile(&mut self, expr_break: &ast::ExprBreak) -> CompileResult<()> {
        let span = expr_break.span();
        log::trace!("ExprBreak => {:?}", self.source.source(span));

        let current_loop = match self.loops.last() {
            Some(current_loop) => current_loop,
            None => {
                return Err(CompileError::BreakOutsideOfLoop { span });
            }
        };

        let (last_loop, to_drop, has_value) = if let Some(expr) = &expr_break.expr {
            match expr {
                ast::ExprBreakValue::Expr(expr) => {
                    self.compile((&**expr, current_loop.needs))?;
                    (current_loop, current_loop.drop.into_iter().collect(), true)
                }
                ast::ExprBreakValue::Label(label) => {
                    let (last_loop, to_drop) =
                        self.loops
                            .walk_until_label(self.storage, &*self.source, *label)?;
                    (last_loop, to_drop, false)
                }
            }
        } else {
            (current_loop, current_loop.drop.into_iter().collect(), false)
        };

        // Drop loop temporary. Typically an iterator.
        for offset in to_drop {
            self.asm.push(Inst::Drop { offset }, span);
        }

        let vars = self
            .scopes
            .last(span)?
            .total_var_count
            .checked_sub(last_loop.total_var_count)
            .ok_or_else(|| CompileError::internal("var count should be larger", span))?;

        if last_loop.needs.value() {
            if has_value {
                self.locals_clean(vars, span);
            } else {
                self.locals_pop(vars, span);
                self.asm.push(Inst::Unit, span);
            }
        } else {
            self.locals_pop(vars, span);
        }

        self.asm.jump(last_loop.break_label, span);
        Ok(())
    }
}
