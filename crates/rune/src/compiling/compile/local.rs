use crate::compiling::compile::prelude::*;

/// Compile a let expression.
impl Compile<(&ast::Local, Needs)> for Compiler<'_> {
    fn compile(&mut self, (local, needs): (&ast::Local, Needs)) -> CompileResult<()> {
        let span = local.span();
        log::trace!("Local => {:?}", self.source.source(span));

        let load = |this: &mut Compiler, needs: Needs| {
            // NB: assignments "move" the value being assigned.
            this.compile((&local.expr, needs))?;
            Ok(())
        };

        let false_label = self.asm.new_label("let_panic");

        if self.compile_pat(&local.pat, false_label, &load)? {
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
