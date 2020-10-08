use crate::compiling::compile::prelude::*;

/// Compile a literal number.
impl Compile2 for ast::LitNumber {
    fn compile2(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        use num::ToPrimitive as _;

        let span = self.span();
        log::trace!("LitNumber => {:?}", c.source.source(span));

        // NB: don't encode unecessary literal.
        if !needs.value() {
            c.warnings.not_used(c.source_id, span, c.context());
            return Ok(());
        }

        let number = self.resolve(&c.storage, &*c.source)?;

        match number {
            ast::Number::Float(number) => {
                c.asm.push(Inst::float(number), span);
            }
            ast::Number::Integer(number) => {
                let n = match number.to_i64() {
                    Some(n) => n,
                    None => {
                        return Err(CompileError::new(
                            span,
                            ParseErrorKind::BadNumberOutOfBounds,
                        ));
                    }
                };

                c.asm.push(Inst::integer(n), span);
            }
        }

        Ok(())
    }
}
