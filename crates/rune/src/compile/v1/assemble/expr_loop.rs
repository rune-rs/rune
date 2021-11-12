use crate::compile::v1::assemble::prelude::*;

/// Compile a loop.
impl Assemble for ast::ExprLoop {
    fn assemble(&self, c: &mut Compiler<'_, '_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("ExprLoop => {:?}", c.q.sources.source(c.source_id, span));

        let continue_label = c.asm.new_label("loop_continue");
        let break_label = c.asm.new_label("loop_break");

        let var_count = c.scopes.total_var_count(span)?;

        let _guard = c.loops.push(Loop {
            label: self.label.map(|(label, _)| label),
            continue_label,
            continue_var_count: var_count,
            break_label,
            break_var_count: var_count,
            needs,
            drop: None,
        });

        c.asm.label(continue_label)?;
        self.body.assemble(c, Needs::None)?.apply(c)?;
        c.asm.jump(continue_label, span);
        c.asm.label(break_label)?;

        Ok(Asm::top(span))
    }
}
