use crate::compiling::assemble::prelude::*;

/// Compile a literal value.
impl Assemble for ast::Lit {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("Lit => {:?}", c.source.source(span));

        match self {
            ast::Lit::Unit(lit_unit) => {
                lit_unit.assemble(c, needs)?;
            }
            ast::Lit::Tuple(lit_tuple) => {
                lit_tuple.assemble(c, needs)?;
            }
            ast::Lit::Bool(lit_bool) => {
                lit_bool.assemble(c, needs)?;
            }
            ast::Lit::Number(lit_number) => {
                lit_number.assemble(c, needs)?;
            }
            ast::Lit::Vec(lit_vec) => {
                lit_vec.assemble(c, needs)?;
            }
            ast::Lit::Object(lit_object) => {
                lit_object.assemble(c, needs)?;
            }
            ast::Lit::Char(lit_char) => {
                lit_char.assemble(c, needs)?;
            }
            ast::Lit::Str(lit_str) => {
                lit_str.assemble(c, needs)?;
            }
            ast::Lit::Byte(lit_char) => {
                lit_char.assemble(c, needs)?;
            }
            ast::Lit::ByteStr(lit_byte_str) => {
                lit_byte_str.assemble(c, needs)?;
            }
        }

        Ok(())
    }
}
