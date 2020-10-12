use crate::compiling::assemble::prelude::*;

/// Compile a block expression.
impl Assemble for ast::ExprBlock {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprBlock => {:?}", c.source.source(span));

        if self.async_token.is_none() {
            return Ok(self.block.assemble(c, needs)?);
        }

        let item = c.query.item_for(&self.block)?;

        let meta = match c.lookup_exact_meta(span, &item.item)? {
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

        let (captures, do_move) = match &meta.kind {
            CompileMetaKind::AsyncBlock {
                captures, do_move, ..
            } => (&**captures, *do_move),
            _ => {
                return Err(CompileError::expected_meta(span, meta, "async block"));
            }
        };

        for ident in captures {
            if do_move {
                let var = c
                    .scopes
                    .take_var(&ident.ident, c.source_id, c.visitor, span)?;

                var.do_move(&mut c.asm, span, format!("captures `{}`", ident.ident));
            } else {
                let var = c
                    .scopes
                    .get_var(&ident.ident, c.source_id, c.visitor, span)?;

                var.copy(&mut c.asm, span, format!("captures `{}`", ident.ident));
            }
        }

        let hash = Hash::type_hash(&meta.item);
        c.asm.push_with_comment(
            Inst::Call {
                hash,
                args: captures.len(),
            },
            span,
            meta.to_string(),
        );

        if !needs.value() {
            c.asm
                .push_with_comment(Inst::Pop, span, "value is not needed");
        }

        Ok(())
    }
}
