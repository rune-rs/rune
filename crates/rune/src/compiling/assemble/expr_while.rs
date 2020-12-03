use crate::compiling::assemble::prelude::*;

/// Compile a while loop.
impl Assemble for ast::ExprWhile {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("ExprWhile => {:?}", c.source.source(span));

        let continue_label = c.asm.new_label("while_continue");
        let then_label = c.asm.new_label("whiel_then");
        let end_label = c.asm.new_label("while_end");
        let break_label = c.asm.new_label("while_break");

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

        let then_scope = c.compile_condition(&self.condition, then_label)?;
        let expected = c.scopes.push(then_scope);

        c.asm.jump(end_label, span);
        c.asm.label(then_label)?;

        self.body.assemble(c, Needs::None)?.apply(c)?;
        c.clean_last_scope(span, expected, Needs::None)?;

        c.asm.jump(continue_label, span);
        c.asm.label(end_label)?;

        if needs.value() {
            c.asm.push(Inst::unit(), span);
        }

        // NB: breaks produce their own value / perform their own cleanup.
        c.asm.label(break_label)?;
        Ok(Asm::top(span))
    }
}
