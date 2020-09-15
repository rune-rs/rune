use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::traits::Compile;
use crate::CompileResult;
use crate::Spanned as _;
use runestick::Inst;

impl Compile<(&ast::ExprMatch, Needs)> for Compiler<'_> {
    fn compile(&mut self, (expr_match, needs): (&ast::ExprMatch, Needs)) -> CompileResult<()> {
        let span = expr_match.span();
        log::trace!("ExprMatch => {:?}", self.source.source(span));

        let expected_scopes = self.scopes.push_child(span)?;

        self.compile((&*expr_match.expr, Needs::Value))?;
        // Offset of the expression.
        let offset = self.scopes.decl_anon(span)?;

        let end_label = self.asm.new_label("match_end");
        let mut branches = Vec::new();

        for (branch, _) in &expr_match.branches {
            let span = branch.span();

            let branch_label = self.asm.new_label("match_branch");
            let match_false = self.asm.new_label("match_false");

            let scope = self.scopes.child(span)?;
            let parent_guard = self.scopes.push(scope);

            let load = move |this: &mut Compiler, needs: Needs| {
                if needs.value() {
                    this.asm.push(Inst::Copy { offset }, span);
                }

                Ok(())
            };

            self.compile_pat(&branch.pat, match_false, &load)?;

            let scope = if let Some((_, condition)) = &branch.condition {
                let span = condition.span();

                let scope = self.scopes.child(span)?;
                let guard = self.scopes.push(scope);

                self.compile((&**condition, Needs::Value))?;
                self.clean_last_scope(span, guard, Needs::Value)?;
                let scope = self.scopes.pop(parent_guard, span)?;

                self.asm
                    .pop_and_jump_if_not(scope.local_var_count, match_false, span);

                self.asm.jump(branch_label, span);
                scope
            } else {
                self.scopes.pop(parent_guard, span)?
            };

            self.asm.jump(branch_label, span);
            self.asm.label(match_false)?;

            branches.push((branch_label, scope));
        }

        // what to do in case nothing matches and the pattern doesn't have any
        // default match branch.
        if needs.value() {
            self.asm.push(Inst::Unit, span);
        }

        self.asm.jump(end_label, span);

        let mut it = expr_match.branches.iter().zip(&branches).peekable();

        while let Some(((branch, _), (label, scope))) = it.next() {
            let span = branch.span();

            self.asm.label(*label)?;

            let expected = self.scopes.push(scope.clone());
            self.compile((&*branch.body, needs))?;
            self.clean_last_scope(span, expected, needs)?;

            if it.peek().is_some() {
                self.asm.jump(end_label, span);
            }
        }

        self.asm.label(end_label)?;

        // pop the implicit scope where we store the anonymous match variable.
        self.clean_last_scope(span, expected_scopes, needs)?;
        Ok(())
    }
}
