use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::traits::Compile;
use crate::CompileResult;
use crate::Spanned as _;
use runestick::Inst;

/// Compile a let expression.
impl Compile<(&ast::ExprLet, Needs)> for Compiler<'_> {
    fn compile(&mut self, (expr_let, needs): (&ast::ExprLet, Needs)) -> CompileResult<()> {
        let span = expr_let.span();
        log::trace!("ExprLet => {:?}", self.source.source(span));

        let load = |this: &mut Compiler, needs: Needs| {
            // NB: assignments "move" the value being assigned.
            this.compile((&*expr_let.expr, needs))?;
            Ok(())
        };

        let false_label = self.asm.new_label("let_panic");

        if self.compile_pat(&expr_let.pat, false_label, &load)? {
            self.warnings
                .let_pattern_might_panic(self.source_id, span, self.context());

            let ok_label = self.asm.new_label("let_ok");
            self.asm.jump(ok_label, span);
            self.asm.label(false_label)?;
            self.asm.push(
                Inst::Panic {
                    reason: runestick::PanicReason::UnmatchedPattern,
                },
                span,
            );

            self.asm.label(ok_label)?;
        }

        // If a value is needed for a let expression, it is evaluated as a unit.
        if needs.value() {
            self.asm.push(Inst::unit(), span);
        }

        Ok(())
    }
}
