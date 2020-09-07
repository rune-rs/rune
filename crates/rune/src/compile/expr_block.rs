use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::error::CompileResult;
use crate::traits::Compile;
use crate::CompileError;
use runestick::{CompileMeta, CompileMetaCapture, Hash, Inst};

struct CallAsync(());
struct BlockBody(());

/// Compile the async block.
impl Compile<(ast::ExprBlock, &[CompileMetaCapture])> for Compiler<'_> {
    fn compile(
        &mut self,
        (expr_block, captures): (ast::ExprBlock, &[CompileMetaCapture]),
    ) -> CompileResult<()> {
        let span = expr_block.span();
        log::trace!("ExprBlock (procedure) => {:?}", self.source.source(span));

        let scope = self.scopes.last(span)?.child();
        let guard = self.scopes.push(scope);

        for capture in captures {
            self.scopes.new_var(&capture.ident, span)?;
        }

        self.compile((BlockBody(()), &expr_block, Needs::Value))?;
        self.clean_last_scope(span, guard, Needs::Value)?;
        self.asm.push(Inst::Return, span);
        Ok(())
    }
}

/// Compile a block expression.
///
/// Blocks are special in that they do not produce a value unless there is
/// an item in them which does.
impl Compile<(&ast::ExprBlock, Needs)> for Compiler<'_> {
    fn compile(&mut self, (expr_block, needs): (&ast::ExprBlock, Needs)) -> CompileResult<()> {
        if let Some(..) = expr_block.async_ {
            self.compile((CallAsync(()), expr_block))?;
        } else {
            self.compile((BlockBody(()), expr_block, needs))?;
        }

        Ok(())
    }
}

/// Call an async block.
impl Compile<(CallAsync, &ast::ExprBlock)> for Compiler<'_> {
    fn compile(&mut self, (_, expr_block): (CallAsync, &ast::ExprBlock)) -> CompileResult<()> {
        let span = expr_block.span();

        let _guard = self.items.push_async_block();
        let item = self.items.item();

        let meta = match self.lookup_meta(&item, span)? {
            Some(meta) => meta,
            None => {
                return Err(CompileError::MissingType { span, item });
            }
        };

        let captures = match &meta {
            CompileMeta::AsyncBlock { captures, .. } => captures,
            _ => {
                return Err(CompileError::UnsupportedAsyncBlock { span, meta });
            }
        };

        for ident in &**captures {
            let var = self.scopes.get_var(&ident.ident, span)?;
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

        Ok(())
    }
}

/// Call a block.
impl Compile<(BlockBody, &ast::ExprBlock, Needs)> for Compiler<'_> {
    fn compile(
        &mut self,
        (_, expr_block, needs): (BlockBody, &ast::ExprBlock, Needs),
    ) -> CompileResult<()> {
        let span = expr_block.span();
        log::trace!("ExprBlock => {:?}", self.source.source(span));

        let _guard = self.items.push_block();

        self.contexts.push(span);

        let span = expr_block.span();

        let new_scope = self.scopes.child(span)?;
        let scopes_count = self.scopes.push(new_scope);

        for (expr, _) in &expr_block.exprs {
            // NB: terminated expressions do not need to produce a value.
            self.compile((expr, Needs::None))?;
        }

        if let Some(expr) = &expr_block.trailing_expr {
            self.compile((&**expr, needs))?;
        }

        let scope = self.scopes.pop(scopes_count, span)?;

        if needs.value() {
            if expr_block.trailing_expr.is_none() {
                self.locals_pop(scope.local_var_count, span);
                self.asm.push(Inst::Unit, span);
            } else {
                self.locals_clean(scope.local_var_count, span);
            }
        } else {
            self.locals_pop(scope.local_var_count, span);
        }

        self.contexts
            .pop()
            .ok_or_else(|| CompileError::internal("missing parent context", span))?;

        Ok(())
    }
}
