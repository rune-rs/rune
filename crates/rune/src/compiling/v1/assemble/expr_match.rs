use crate::compiling::v1::assemble::prelude::*;

impl Assemble for ast::ExprMatch {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("ExprMatch => {:?}", c.source.source(span));

        let expected_scopes = c.scopes.push_child(span)?;

        self.expr.assemble(c, Needs::Value)?.apply(c)?;
        // Offset of the expression.
        let offset = c.scopes.decl_anon(span)?;

        let end_label = c.asm.new_label("match_end");
        let mut branches = Vec::new();

        for (branch, _) in &self.branches {
            let span = branch.span();

            let branch_label = c.asm.new_label("match_branch");
            let match_false = c.asm.new_label("match_false");

            let scope = c.scopes.child(span)?;
            let parent_guard = c.scopes.push(scope);

            let load = move |this: &mut Compiler, needs: Needs| {
                if needs.value() {
                    this.asm.push(Inst::Copy { offset }, span);
                }

                Ok(())
            };

            c.compile_pat(&branch.pat, match_false, &load)?;

            let scope = if let Some((_, condition)) = &branch.condition {
                let span = condition.span();

                let scope = c.scopes.child(span)?;
                let guard = c.scopes.push(scope);

                condition.assemble(c, Needs::Value)?.apply(c)?;
                c.clean_last_scope(span, guard, Needs::Value)?;
                let scope = c.scopes.pop(parent_guard, span)?;

                c.asm
                    .pop_and_jump_if_not(scope.local_var_count, match_false, span);

                c.asm.jump(branch_label, span);
                scope
            } else {
                c.scopes.pop(parent_guard, span)?
            };

            c.asm.jump(branch_label, span);
            c.asm.label(match_false)?;

            branches.push((branch_label, scope));
        }

        // what to do in case nothing matches and the pattern doesn't have any
        // default match branch.
        if needs.value() {
            c.asm.push(Inst::unit(), span);
        }

        c.asm.jump(end_label, span);

        let mut it = self.branches.iter().zip(&branches).peekable();

        while let Some(((branch, _), (label, scope))) = it.next() {
            let span = branch.span();

            c.asm.label(*label)?;

            let expected = c.scopes.push(scope.clone());
            branch.body.assemble(c, needs)?.apply(c)?;
            c.clean_last_scope(span, expected, needs)?;

            if it.peek().is_some() {
                c.asm.jump(end_label, span);
            }
        }

        c.asm.label(end_label)?;

        // pop the implicit scope where we store the anonymous match variable.
        c.clean_last_scope(span, expected_scopes, needs)?;
        Ok(Asm::top(span))
    }
}
