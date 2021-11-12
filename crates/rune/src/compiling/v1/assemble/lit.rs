use crate::compiling::v1::assemble::prelude::*;

/// Compile a literal value.
impl Assemble for ast::Lit {
    fn assemble(&self, c: &mut Compiler<'_, '_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("Lit => {:?}", c.q.sources.source(c.source_id, span));

        let asm = match self {
            ast::Lit::Bool(lit_bool) => lit_bool.assemble(c, needs)?,
            ast::Lit::Number(lit_number) => lit_number.assemble(c, needs)?,
            ast::Lit::Char(lit_char) => lit_char.assemble(c, needs)?,
            ast::Lit::Str(lit_str) => lit_str.assemble(c, needs)?,
            ast::Lit::Byte(lit_char) => lit_char.assemble(c, needs)?,
            ast::Lit::ByteStr(lit_byte_str) => lit_byte_str.assemble(c, needs)?,
        };

        Ok(asm)
    }
}
