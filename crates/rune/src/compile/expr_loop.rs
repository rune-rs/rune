use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::error::CompileResult;
use crate::loops::Loop;
use crate::traits::Compile;

/// Compile a loop.
impl Compile<(&ast::ExprLoop, Needs)> for Compiler<'_> {
    fn compile(&mut self, (expr_loop, needs): (&ast::ExprLoop, Needs)) -> CompileResult<()> {
        let span = expr_loop.span();
        log::trace!("ExprLoop => {:?}", self.source.source(span));

        let start_label = self.asm.new_label("loop_start");
        let end_label = self.asm.new_label("loop_end");

        let _guard = self.loops.push(Loop {
            label: expr_loop.label.map(|(label, _)| label),
            break_label: end_label,
            total_var_count: self.scopes.total_var_count(span)?,
            needs,
            drop: None,
        });

        self.asm.label(start_label)?;
        self.compile((&*expr_loop.body, Needs::None))?;
        self.asm.jump(start_label, span);
        self.asm.label(end_label)?;

        Ok(())
    }
}
