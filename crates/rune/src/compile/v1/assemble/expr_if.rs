use crate::compile::v1::assemble::prelude::*;

/// Compile an if expression.
impl Assemble for ast::ExprIf {
    fn assemble(&self, c: &mut Compiler<'_, '_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("ExprIf => {:?}", c.q.sources.source(c.source_id, span));

        let then_label = c.asm.new_label("if_then");
        let end_label = c.asm.new_label("if_end");

        let mut branches = Vec::new();
        let then_scope = c.compile_condition(&self.condition, then_label)?;

        for branch in &self.expr_else_ifs {
            let label = c.asm.new_label("if_branch");
            let scope = c.compile_condition(&branch.condition, label)?;
            branches.push((branch, label, scope));
        }

        // use fallback as fall through.
        if let Some(fallback) = &self.expr_else {
            fallback.block.assemble(c, needs)?.apply(c)?;
        } else {
            // NB: if we must produce a value and there is no fallback branch,
            // encode the result of the statement as a unit.
            if needs.value() {
                c.asm.push(Inst::unit(), span);
            }
        }

        c.asm.jump(end_label, span);

        c.asm.label(then_label)?;

        let expected = c.scopes.push(then_scope);
        self.block.assemble(c, needs)?.apply(c)?;
        c.clean_last_scope(span, expected, needs)?;

        if !self.expr_else_ifs.is_empty() {
            c.asm.jump(end_label, span);
        }

        let mut it = branches.into_iter().peekable();
        while let Some((branch, label, scope)) = it.next() {
            let span = branch.span();

            c.asm.label(label)?;

            let scopes = c.scopes.push(scope);
            branch.block.assemble(c, needs)?.apply(c)?;
            c.clean_last_scope(span, scopes, needs)?;

            if it.peek().is_some() {
                c.asm.jump(end_label, span);
            }
        }

        c.asm.label(end_label)?;
        Ok(Asm::top(span))
    }
}
