use crate::compiling::v1::assemble::prelude::*;

/// Compile a block expression.
impl Assemble for ast::ExprBlock {
    fn assemble(&self, c: &mut Compiler<'_>, needs: Needs) -> CompileResult<Asm> {
        let span = self.span();
        log::trace!("ExprBlock => {:?}", c.source.source(span));

        if self.async_token.is_none() && self.const_token.is_none() {
            return self.block.assemble(c, needs);
        }

        let item = c.query.item_for(&self.block)?;
        let meta = c.lookup_meta(span, &item.item)?;

        match &meta.kind {
            CompileMetaKind::AsyncBlock {
                captures, do_move, ..
            } => {
                let captures = &**captures;
                let do_move = *do_move;

                for ident in captures {
                    if do_move {
                        let var = c.scopes.take_var(&ident.ident, c.source_id, span)?;
                        var.do_move(&mut c.asm, span, format!("captures `{}`", ident.ident));
                    } else {
                        let var = c.scopes.get_var(&ident.ident, c.source_id, span)?;
                        var.copy(&mut c.asm, span, format!("captures `{}`", ident.ident));
                    }
                }

                let hash = Hash::type_hash(&meta.item.item);
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
            }
            CompileMetaKind::Const { const_value } => {
                const_value.assemble_const(c, needs, span)?;
            }
            _ => {
                return Err(CompileError::expected_meta(span, meta, "async block"));
            }
        };

        Ok(Asm::top(span))
    }
}
