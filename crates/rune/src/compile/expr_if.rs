use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::traits::Compile;
use crate::CompileResult;
use crate::Spanned as _;
use runestick::Inst;

/// Compile an if expression.
impl Compile<(&ast::ExprIf, Needs)> for Compiler<'_> {
    fn compile(&mut self, (expr_if, needs): (&ast::ExprIf, Needs)) -> CompileResult<()> {
        let span = expr_if.span();
        log::trace!("ExprIf => {:?}", self.source.source(span));

        let then_label = self.asm.new_label("if_then");
        let end_label = self.asm.new_label("if_end");

        let mut branches = Vec::new();
        let then_scope = self.compile_condition(&expr_if.condition, then_label)?;

        for branch in &expr_if.expr_else_ifs {
            let label = self.asm.new_label("if_branch");
            let scope = self.compile_condition(&branch.condition, label)?;
            branches.push((branch, label, scope));
        }

        // use fallback as fall through.
        if let Some(fallback) = &expr_if.expr_else {
            self.compile((&*fallback.block, needs))?;
        } else {
            // NB: if we must produce a value and there is no fallback branch,
            // encode the result of the statement as a unit.
            if needs.value() {
                self.asm.push(Inst::Unit, span);
            }
        }

        self.asm.jump(end_label, span);

        self.asm.label(then_label)?;

        let expected = self.scopes.push(then_scope);
        self.compile((&*expr_if.block, needs))?;
        self.clean_last_scope(span, expected, needs)?;

        if !expr_if.expr_else_ifs.is_empty() {
            self.asm.jump(end_label, span);
        }

        let mut it = branches.into_iter().peekable();

        if let Some((branch, label, scope)) = it.next() {
            let span = branch.span();

            self.asm.label(label)?;

            let scopes = self.scopes.push(scope);
            self.compile((&*branch.block, needs))?;
            self.clean_last_scope(span, scopes, needs)?;

            if it.peek().is_some() {
                self.asm.jump(end_label, span);
            }
        }

        self.asm.label(end_label)?;
        Ok(())
    }
}
