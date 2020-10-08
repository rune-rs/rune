use crate::compiling::compile::prelude::*;

/// Compile a literal value.
impl Compile2 for ast::Lit {
    fn compile2(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("Lit => {:?}", c.source.source(span));

        match self {
            ast::Lit::Unit(lit_unit) => {
                lit_unit.compile2(c, needs)?;
            }
            ast::Lit::Tuple(lit_tuple) => {
                lit_tuple.compile2(c, needs)?;
            }
            ast::Lit::Bool(lit_bool) => {
                lit_bool.compile2(c, needs)?;
            }
            ast::Lit::Number(lit_number) => {
                lit_number.compile2(c, needs)?;
            }
            ast::Lit::Vec(lit_vec) => {
                lit_vec.compile2(c, needs)?;
            }
            ast::Lit::Object(lit_object) => {
                lit_object.compile2(c, needs)?;
            }
            ast::Lit::Char(lit_char) => {
                lit_char.compile2(c, needs)?;
            }
            ast::Lit::Str(lit_str) => {
                lit_str.compile2(c, needs)?;
            }
            ast::Lit::Byte(lit_char) => {
                lit_char.compile2(c, needs)?;
            }
            ast::Lit::ByteStr(lit_byte_str) => {
                lit_byte_str.compile2(c, needs)?;
            }
            ast::Lit::Template(lit_template) => {
                lit_template.compile2(c, needs)?;
            }
        }

        Ok(())
    }
}
