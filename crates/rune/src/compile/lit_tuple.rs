use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::traits::Compile;
use crate::CompileResult;
use crate::Spanned as _;
use runestick::Inst;

/// Compile a literal tuple.
impl Compile<(&ast::LitTuple, Needs)> for Compiler<'_> {
    fn compile(&mut self, (lit_tuple, needs): (&ast::LitTuple, Needs)) -> CompileResult<()> {
        let span = lit_tuple.span();
        log::trace!("LitTuple => {:?}", self.source.source(span));

        // If the value is not needed, no need to encode it.
        if !needs.value() && lit_tuple.is_const() {
            self.warnings.not_used(self.source_id, span, self.context());
            return Ok(());
        }

        for (expr, _) in lit_tuple.items.iter() {
            self.compile((expr, Needs::Value))?;
        }

        self.asm.push(
            Inst::Tuple {
                count: lit_tuple.items.len(),
            },
            span,
        );

        Ok(())
    }
}
