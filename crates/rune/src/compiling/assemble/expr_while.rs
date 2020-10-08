use crate::compiling::assemble::prelude::*;

/// Compile a while loop.
impl Assemble for ast::ExprWhile {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprWhile => {:?}", c.source.source(span));

        let start_label = c.asm.new_label("while_test");
        let then_label = c.asm.new_label("while_then");
        let end_label = c.asm.new_label("while_end");
        let break_label = c.asm.new_label("while_break");

        let _guard = c.loops.push(Loop {
            label: self.label.map(|(label, _)| label),
            break_label,
            total_var_count: c.scopes.total_var_count(span)?,
            needs,
            drop: None,
        });

        c.asm.label(start_label)?;

        let then_scope = c.compile_condition(&self.condition, then_label)?;
        c.asm.jump(end_label, span);
        c.asm.label(then_label)?;

        let expected = c.scopes.push(then_scope);
        self.body.assemble(c, Needs::None)?;
        c.clean_last_scope(span, expected, Needs::None)?;

        c.asm.jump(start_label, span);
        c.asm.label(end_label)?;

        if needs.value() {
            c.asm.push(Inst::unit(), span);
        }

        // NB: breaks produce their own value.
        c.asm.label(break_label)?;
        Ok(())
    }
}
