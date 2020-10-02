use crate::compiling::compile::prelude::*;

/// Compile a literal number.
impl Compile<(&ast::LitNumber, Needs)> for Compiler<'_> {
    fn compile(&mut self, (lit_number, needs): (&ast::LitNumber, Needs)) -> CompileResult<()> {
        use num::ToPrimitive as _;

        let span = lit_number.span();
        log::trace!("LitNumber => {:?}", self.source.source(span));

        // NB: don't encode unecessary literal.
        if !needs.value() {
            self.warnings.not_used(self.source_id, span, self.context());
            return Ok(());
        }

        let lit_number = lit_number.resolve(&self.storage, &*self.source)?;

        match lit_number {
            ast::Number::Float(number) => {
                self.asm.push(Inst::float(number), span);
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

                self.asm.push(Inst::integer(n), span);
            }
        }

        Ok(())
    }
}
