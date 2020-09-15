use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::traits::Compile;
use crate::CompileResult;
use crate::{CompileError, CompileErrorKind, Spanned as _};
use runestick::{CompileMetaKind, Hash, Inst};

/// Call an async block.
impl Compile<(&ast::ExprAsync, Needs)> for Compiler<'_> {
    fn compile(&mut self, (expr_async, needs): (&ast::ExprAsync, Needs)) -> CompileResult<()> {
        let span = expr_async.span();

        let _guard = self.items.push_async_block();
        let item = self.items.item();

        let meta = match self.lookup_meta(&item, span)? {
            Some(meta) => meta,
            None => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::MissingType { item },
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
                .get_var(&ident.ident, self.source.url(), self.visitor, span)?;
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
