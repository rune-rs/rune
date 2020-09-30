use crate::compiling::compile::prelude::*;

/// Compile a block expression.
impl Compile<(&ast::ExprBlock, Needs)> for Compiler<'_> {
    fn compile(&mut self, (expr_block, needs): (&ast::ExprBlock, Needs)) -> CompileResult<()> {
        let span = expr_block.span();
        log::trace!("ExprBlock => {:?}", self.source.source(span));

        if expr_block.async_token.is_none() {
            return Ok(self.compile((&expr_block.block, needs))?);
        }

        let item = self.query.item_for(&expr_block.block)?.clone();

        let meta = match self.lookup_exact_meta(span, &item.item)? {
            Some(meta) => meta,
            None => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::MissingType {
                        item: item.item.clone(),
                    },
                ));
            }
        };

        let captures = match &meta.kind {
            CompileMetaKind::AsyncBlock { captures, .. } => captures,
            _ => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::UnsupportedAsyncBlock { meta },
                ));
            }
        };

        for ident in &**captures {
            let var = self
                .scopes
                .get_var(&ident.ident, self.source_id, self.visitor, span)?;
            var.copy(&mut self.asm, span, format!("captures `{}`", ident.ident));
        }

        let item = meta.item();
        let hash = Hash::type_hash(item);
        self.asm.push_with_comment(
            Inst::Call {
                hash,
                args: captures.len(),
            },
            span,
            format!("fn `{}`", item),
        );

        if !needs.value() {
            self.asm
                .push_with_comment(Inst::Pop, span, "value is not needed");
        }

        Ok(())
    }
}
