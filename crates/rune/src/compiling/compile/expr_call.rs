use crate::compiling::compile::prelude::*;

/// Compile a call expression.
impl Compile<(&ast::ExprCall, Needs)> for Compiler<'_> {
    fn compile(&mut self, (expr_call, needs): (&ast::ExprCall, Needs)) -> CompileResult<()> {
        let span = expr_call.span();
        log::trace!("ExprCall => {:?}", self.source.source(span));

        let guard = self.scopes.push_child(span)?;
        let args = expr_call.args.len();

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

                    for (expr, _) in &expr_call.args {
                        self.compile((expr, Needs::Value))?;
                        self.scopes.decl_anon(span)?;
                    }

                    let ident = ident.resolve(&self.storage, &*self.source)?;
                    let hash = Hash::instance_fn_name(ident.as_ref());
                    self.asm.push(Inst::CallInstance { hash, args }, span);
                }
                expr => {
                    log::trace!("ExprCall(Other) => {:?}", self.source.source(span));

                    for (expr, _) in &expr_call.args {
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

        let (query_path, named) = self.convert_path_to_named(path)?;

        if let Some(name) = named.as_local() {
            let local = self
                .scopes
                .try_get_var(name, self.source_id, self.visitor, path.span())
                .copied();

            if let Some(var) = local {
                for (expr, _) in &expr_call.args {
                    self.compile((expr, Needs::Value))?;
                    self.scopes.decl_anon(span)?;
                }

                var.copy(&mut self.asm, span, format!("var `{}`", name));
                self.asm.push(Inst::CallFn { args }, span);

                if !needs.value() {
                    self.asm.push(Inst::Pop, span);
                }

                self.scopes.pop(guard, span)?;
                return Ok(());
            }
        }

        let meta = match self.lookup_meta(path.span(), &*query_path, &named)? {
            Some(meta) => meta,
            None => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::MissingFunction {
                        item: named.item.clone(),
                    },
                ));
            }
        };

        match &meta.kind {
            CompileMetaKind::UnitStruct { .. } | CompileMetaKind::UnitVariant { .. } => {
                if 0 != expr_call.args.len() {
                    return Err(CompileError::new(
                        span,
                        CompileErrorKind::UnsupportedArgumentCount {
                            meta: meta.clone(),
                            expected: 0,
                            actual: expr_call.args.len(),
                        },
                    ));
                }
            }
            CompileMetaKind::TupleStruct { tuple, .. }
            | CompileMetaKind::TupleVariant { tuple, .. } => {
                if tuple.args != expr_call.args.len() {
                    return Err(CompileError::new(
                        span,
                        CompileErrorKind::UnsupportedArgumentCount {
                            meta: meta.clone(),
                            expected: tuple.args,
                            actual: expr_call.args.len(),
                        },
                    ));
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
            }
            CompileMetaKind::Function { .. } => (),
            CompileMetaKind::ConstFn { id, .. } => {
                let from = self.query.item_for(expr_call)?.clone();
                let const_fn = self.query.const_fn_for((expr_call.span(), *id))?;

                let value = self.call_const_fn(
                    expr_call,
                    &meta,
                    &from,
                    &*const_fn,
                    expr_call.args.as_slice(),
                )?;

                self.compile((&value, expr_call.span()))?;
                self.scopes.pop(guard, span)?;
                return Ok(());
            }
            _ => {
                return Err(CompileError::expected_meta(
                    span,
                    meta,
                    "something that can be called as a function",
                ));
            }
        };

        for (expr, _) in &expr_call.args {
            self.compile((expr, Needs::Value))?;
            self.scopes.decl_anon(span)?;
        }

        let hash = Hash::type_hash(&meta.item);
        self.asm
            .push_with_comment(Inst::Call { hash, args }, span, meta.to_string());

        // NB: we put it here to preserve the call in case it has side effects.
        // But if we don't need the value, then pop it from the stack.
        if !needs.value() {
            self.asm.push(Inst::Pop, span);
        }

        self.scopes.pop(guard, span)?;
        Ok(())
    }
}
