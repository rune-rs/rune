use crate::compiling::assemble::prelude::*;
use crate::query::BuiltInFormatSpec;
use runestick::format_spec;

/// Compile a literal template string.
impl Assemble for BuiltInFormatSpec {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span;
        log::trace!("BuiltInFormatSpec => {:?}", c.source.source(span));

        let ty = if let Some((_, ty)) = &self.ty {
            *ty
        } else {
            format_spec::Type::default()
        };

        self.value.assemble(c, Needs::Value)?;
        c.asm.push(Inst::FormatSpec { ty }, span);

        if !needs.value() {
            c.asm.push(Inst::Pop, span);
        }

        Ok(())
    }
}
