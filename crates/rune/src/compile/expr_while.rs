use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::loops::Loop;
use crate::traits::Compile;
use crate::CompileResult;
use crate::Spanned as _;
use runestick::Inst;

/// Compile a while loop.
impl Compile<(&ast::ExprWhile, Needs)> for Compiler<'_> {
    fn compile(&mut self, (expr_while, needs): (&ast::ExprWhile, Needs)) -> CompileResult<()> {
        let span = expr_while.span();
        log::trace!("ExprWhile => {:?}", self.source.source(span));

        let start_label = self.asm.new_label("while_test");
        let then_label = self.asm.new_label("while_then");
        let end_label = self.asm.new_label("while_end");
        let break_label = self.asm.new_label("while_break");

        let _guard = self.loops.push(Loop {
            label: expr_while.label.map(|(label, _)| label),
            break_label,
            total_var_count: self.scopes.total_var_count(span)?,
            needs,
            drop: None,
        });

        self.asm.label(start_label)?;

        let then_scope = self.compile_condition(&expr_while.condition, then_label)?;
        self.asm.jump(end_label, span);
        self.asm.label(then_label)?;

        let expected = self.scopes.push(then_scope);
        self.compile((&*expr_while.body, Needs::None))?;
        self.clean_last_scope(span, expected, Needs::None)?;

        self.asm.jump(start_label, span);
        self.asm.label(end_label)?;

        if needs.value() {
            self.asm.push(Inst::Unit, span);
        }

        // NB: breaks produce their own value.
        self.asm.label(break_label)?;
        Ok(())
    }
}
