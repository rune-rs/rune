use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::traits::Compile;
use crate::CompileResult;
use crate::Spanned as _;

/// Compile a block expression.
///
/// Blocks are special in that they do not produce a value unless there is
/// an item in them which does.
impl Compile<(&ast::ExprBlock, Needs)> for Compiler<'_> {
    fn compile(&mut self, (expr_block, needs): (&ast::ExprBlock, Needs)) -> CompileResult<()> {
        log::trace!("ExprBlock => {:?}", self.source.source(expr_block.span()));
        self.compile((&expr_block.block, needs))?;
        Ok(())
    }
}
