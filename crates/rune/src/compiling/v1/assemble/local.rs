use crate::compiling::v1::assemble::prelude::*;

/// Compile a let expression.
impl Assemble for ast::Local {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("Local => {:?}", c.source.source(span));

        let load = |c: &mut Compiler, needs: Needs| {
            // NB: assignments "move" the value being assigned.
            self.expr.assemble(c, needs)?.apply(c)?;
            Ok(())
        };

        let false_label = c.asm.new_label("let_panic");

        if c.compile_pat(&self.pat, false_label, &load)? {
            c.warnings
                .let_pattern_might_panic(c.source_id, span, c.context());

            let ok_label = c.asm.new_label("let_ok");
            c.asm.jump(ok_label, span);
            c.asm.label(false_label)?;
            c.asm.push(
                Inst::Panic {
                    reason: runestick::PanicReason::UnmatchedPattern,
                },
                span,
            );

            c.asm.label(ok_label)?;
        }

        // If a value is needed for a let expression, it is evaluated as a unit.
        if needs.value() {
            c.asm.push(Inst::unit(), span);
        }

        Ok(Asm::top(span))
    }
}
