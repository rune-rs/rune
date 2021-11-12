use crate::compiling::v1::assemble::prelude::*;

/// Compile a break expression.
///
/// NB: loops are expected to produce a value at the end of their expression.
impl Assemble for ast::ExprBreak {
    fn assemble(&self, c: &mut Compiler<'_, '_>, _: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("ExprBreak => {:?}", c.source.source(span));

        let current_loop = match c.loops.last() {
            Some(current_loop) => current_loop,
            None => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::BreakOutsideOfLoop,
                ));
            }
        };

        let (last_loop, to_drop, has_value) = if let Some(expr) = &self.expr {
            match expr {
                ast::ExprBreakValue::Expr(expr) => {
                    expr.assemble(c, current_loop.needs)?.apply(c)?;
                    (current_loop, current_loop.drop.into_iter().collect(), true)
                }
                ast::ExprBreakValue::Label(label) => {
                    let (last_loop, to_drop) =
                        c.loops
                            .walk_until_label(c.query.storage(), c.sources, *label)?;
                    (last_loop, to_drop, false)
                }
            }
        } else {
            (current_loop, current_loop.drop.into_iter().collect(), false)
        };

        // Drop loop temporary. Typically an iterator.
        for offset in to_drop {
            c.asm.push(Inst::Drop { offset }, span);
        }

        let vars = c
            .scopes
            .total_var_count(span)?
            .checked_sub(last_loop.break_var_count)
            .ok_or_else(|| CompileError::msg(&span, "var count should be larger"))?;

        if last_loop.needs.value() {
            if has_value {
                c.locals_clean(vars, span);
            } else {
                c.locals_pop(vars, span);
                c.asm.push(Inst::unit(), span);
            }
        } else {
            c.locals_pop(vars, span);
        }

        c.asm.jump(last_loop.break_label, span);
        Ok(Asm::top(span))
    }
}
