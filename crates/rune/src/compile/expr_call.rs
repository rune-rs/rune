use crate::ast;
use crate::compiler::{Compiler, Needs};
use crate::error::CompileResult;
use crate::traits::{Compile, Resolve as _};
use crate::CompileError;
use runestick::{CompileMeta, Hash, Inst};

/// Compile a call expression.
impl Compile<(&ast::ExprCall, Needs)> for Compiler<'_> {
    fn compile(&mut self, (expr_call, needs): (&ast::ExprCall, Needs)) -> CompileResult<()> {
        let span = expr_call.span();
        log::trace!("ExprCall => {:?}", self.source.source(span));

        let scope = self.scopes.child(span)?;
        let guard = self.scopes.push(scope);

        let args = expr_call.args.items.len();

        // NB: either handle a proper function call by resolving it's meta hash,
        // or expand the expression.
        #[allow(clippy::never_loop)]
        let path = loop {
            match &*expr_call.expr {
                ast::Expr::Path(path) => {
                    log::trace!("ExprCall(Path) => {:?}", self.source.source(span));
                    break path;
                }
                ast::Expr::ExprFieldAccess(ast::ExprFieldAccess {
                    expr,
                    expr_field: ast::ExprField::Ident(ident),
                    ..
                }) => {
                    log::trace!(
                        "ExprCall(ExprFieldAccess) => {:?}",
                        self.source.source(span)
                    );

                    self.compile((&**expr, Needs::Value))?;

                    for (expr, _) in expr_call.args.items.iter() {
                        self.compile((expr, Needs::Value))?;
                        self.scopes.decl_anon(span)?;
                    }

                    let ident = ident.resolve(&self.storage, &*self.source)?;
                    let hash = Hash::of(ident);
                    self.asm.push(Inst::CallInstance { hash, args }, span);
                }
                expr => {
                    log::trace!("ExprCall(Other) => {:?}", self.source.source(span));

                    for (expr, _) in expr_call.args.items.iter() {
                        self.compile((expr, Needs::Value))?;
                        self.scopes.decl_anon(span)?;
                    }

                    self.compile((expr, Needs::Value))?;
                    self.asm.push(Inst::CallFn { args }, span);
                }
            }

            if !needs.value() {
                self.asm.push(Inst::Pop, span);
            }

            self.scopes.pop(guard, span)?;
            return Ok(());
        };

        for (expr, _) in expr_call.args.items.iter() {
            self.compile((expr, Needs::Value))?;
            self.scopes.decl_anon(span)?;
        }

        let item = self.convert_path_to_item(path)?;

        if let Some(name) = item.as_local() {
            if let Some(var) = self.scopes.try_get_var(name)? {
                var.copy(&mut self.asm, span, format!("var `{}`", name));
                self.asm.push(Inst::CallFn { args }, span);

                if !needs.value() {
                    self.asm.push(Inst::Pop, span);
                }

                self.scopes.pop(guard, span)?;
                return Ok(());
            }
        }

        let meta = match self.lookup_meta(&item, path.span())? {
            Some(meta) => meta,
            None => {
                return Err(CompileError::MissingFunction { span, item });
            }
        };

        let item = match &meta {
            CompileMeta::Tuple { tuple, .. } | CompileMeta::TupleVariant { tuple, .. } => {
                if tuple.args != expr_call.args.items.len() {
                    return Err(CompileError::UnsupportedArgumentCount {
                        span,
                        meta: meta.clone(),
                        expected: tuple.args,
                        actual: expr_call.args.items.len(),
                    });
                }

                if tuple.args == 0 {
                    let tuple = path.span();
                    self.warnings.remove_tuple_call_parens(
                        self.source_id,
                        span,
                        tuple,
                        self.context(),
                    );
                }

                tuple.item.clone()
            }
            CompileMeta::Function { item, .. } => item.clone(),
            _ => {
                return Err(CompileError::MissingFunction { span, item });
            }
        };

        let hash = Hash::type_hash(&item);
        self.asm
            .push_with_comment(Inst::Call { hash, args }, span, format!("fn `{}`", item));

        // NB: we put it here to preserve the call in case it has side effects.
        // But if we don't need the value, then pop it from the stack.
        if !needs.value() {
            self.asm.push(Inst::Pop, span);
        }

        self.scopes.pop(guard, span)?;
        Ok(())
    }
}
