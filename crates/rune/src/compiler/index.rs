use crate::ast;
use crate::compiler::index_scopes::IndexScopes;
use crate::compiler::query::{Build, Function, Indexed, InstanceFunction, Query};
use crate::compiler::warning::Warnings;
use crate::compiler::Items;
use crate::error::CompileError;
use crate::traits::Resolve as _;
use runestick::{Hash, Meta, ValueType};
use std::sync::Arc;

pub(super) struct Indexer<'a, 'source> {
    pub(super) items: Items,
    pub(super) scopes: IndexScopes,
    pub(super) query: &'a mut Query<'source>,
    pub(super) warnings: &'a mut Warnings,
}

pub(super) trait Index<T> {
    /// Walk the current type with the given item.
    fn index(&mut self, item: &T) -> Result<(), CompileError>;
}

impl Index<ast::DeclFile> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::DeclFile) -> Result<(), CompileError> {
        for (decl, semi_colon) in &item.decls {
            if let Some(semi_colon) = semi_colon {
                if !decl.needs_semi_colon() {
                    self.warnings.uneccessary_semi_colon(semi_colon.span());
                }
            }

            self.index(decl)?;
        }

        Ok(())
    }
}

impl Index<ast::DeclFn> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::DeclFn) -> Result<(), CompileError> {
        let span = item.span();
        let scope = self.scopes.push_function();

        for (arg, _) in &item.args.items {
            match arg {
                ast::FnArg::Self_(s) => {
                    let span = s.span();
                    self.scopes.declare("self", span)?;
                }
                ast::FnArg::Ident(ident) => {
                    let span = ident.span();
                    let ident = ident.resolve(self.query.source)?;
                    self.scopes.declare(ident, span)?;
                }
                _ => (),
            }
        }

        self.index(&item.body)?;
        self.scopes.pop_function(scope, span)?;
        Ok(())
    }
}

impl Index<ast::ExprBlock> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprBlock) -> Result<(), CompileError> {
        let span = item.span();
        let guard = self.items.push_block();
        let scope_guard = self.scopes.push_scope();

        for (expr, _) in &item.exprs {
            self.index(expr)?;
        }

        if let Some(expr) = &item.trailing_expr {
            self.index(&**expr)?;
        }

        self.scopes.pop_scope(scope_guard, span)?;
        self.items.pop(guard, span)?;
        Ok(())
    }
}

impl Index<ast::ExprLet> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprLet) -> Result<(), CompileError> {
        self.index(&item.pat)?;
        self.index(&*item.expr)?;
        Ok(())
    }
}

impl Index<ast::Ident> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::Ident) -> Result<(), CompileError> {
        let span = item.span();
        let ident = item.resolve(self.query.source)?;
        self.scopes.declare(ident, span)?;
        Ok(())
    }
}

impl Index<ast::Pat> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::Pat) -> Result<(), CompileError> {
        match item {
            ast::Pat::PatPath(pat_path) => {
                if let Some(ident) = pat_path.path.try_as_ident() {
                    self.index(ident)?;
                }
            }
            ast::Pat::PatObject(pat_object) => {
                self.index(pat_object)?;
            }
            ast::Pat::PatVec(pat_vec) => {
                self.index(pat_vec)?;
            }
            ast::Pat::PatTuple(pat_tuple) => {
                self.index(pat_tuple)?;
            }
            ast::Pat::PatByte(..) => (),
            ast::Pat::PatIgnore(..) => (),
            ast::Pat::PatNumber(..) => (),
            ast::Pat::PatString(..) => (),
            ast::Pat::PatUnit(..) => (),
            ast::Pat::PatChar(..) => (),
        }

        Ok(())
    }
}

impl Index<ast::PatTuple> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::PatTuple) -> Result<(), CompileError> {
        for (pat, _) in &item.items {
            self.index(&**pat)?;
        }

        Ok(())
    }
}

impl Index<ast::PatObject> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::PatObject) -> Result<(), CompileError> {
        for (field, _) in &item.fields {
            if let Some((_, pat)) = &field.binding {
                self.index(pat)?;
            } else {
                match &field.key {
                    ast::LitObjectKey::Ident(ident) => {
                        self.index(ident)?;
                    }
                    ast::LitObjectKey::LitStr(..) => (),
                }
            }
        }

        Ok(())
    }
}

impl Index<ast::PatVec> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::PatVec) -> Result<(), CompileError> {
        for (pat, _) in &item.items {
            self.index(&**pat)?;
        }

        Ok(())
    }
}

impl Index<ast::Expr> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::Expr) -> Result<(), CompileError> {
        match item {
            ast::Expr::Self_(..) => {
                self.scopes.mark_use("self");
            }
            ast::Expr::Path(path) => {
                self.index(path)?;
            }
            ast::Expr::ExprLet(expr_let) => {
                self.index(expr_let)?;
            }
            ast::Expr::ExprBlock(block) => {
                self.index(block)?;
            }
            ast::Expr::ExprGroup(expr) => {
                self.index(&*expr.expr)?;
            }
            ast::Expr::ExprIf(expr_if) => {
                self.index(expr_if)?;
            }
            ast::Expr::ExprBinary(expr_binary) => {
                self.index(expr_binary)?;
            }
            ast::Expr::ExprMatch(expr_if) => {
                self.index(expr_if)?;
            }
            ast::Expr::Decl(decl) => {
                self.index(decl)?;
            }
            ast::Expr::ExprClosure(expr_closure) => {
                self.index(expr_closure)?;
            }
            ast::Expr::ExprWhile(expr_while) => {
                self.index(expr_while)?;
            }
            ast::Expr::ExprLoop(expr_loop) => {
                self.index(expr_loop)?;
            }
            ast::Expr::ExprFor(expr_for) => {
                self.index(expr_for)?;
            }
            ast::Expr::ExprIndexSet(expr_index_set) => {
                self.index(expr_index_set)?;
            }
            ast::Expr::ExprFieldAccess(expr_field_access) => {
                self.index(expr_field_access)?;
            }
            ast::Expr::ExprUnary(expr_unary) => {
                self.index(expr_unary)?;
            }
            ast::Expr::ExprIndexGet(expr_index_get) => {
                self.index(expr_index_get)?;
            }
            ast::Expr::ExprBreak(expr_break) => {
                self.index(expr_break)?;
            }
            ast::Expr::ExprReturn(expr_return) => {
                self.index(expr_return)?;
            }
            ast::Expr::ExprAwait(expr_await) => {
                self.index(expr_await)?;
            }
            ast::Expr::ExprTry(expr_try) => {
                self.index(expr_try)?;
            }
            ast::Expr::ExprSelect(expr_select) => {
                self.index(expr_select)?;
            }
            // ignored because they have no effect on indexing.
            ast::Expr::ExprCall(expr_call) => {
                self.index(expr_call)?;
            }
            ast::Expr::LitUnit(..) => (),
            ast::Expr::LitBool(..) => (),
            ast::Expr::LitByte(..) => (),
            ast::Expr::LitChar(..) => (),
            ast::Expr::LitNumber(..) => (),
            ast::Expr::LitObject(..) => (),
            ast::Expr::LitStr(..) => (),
            ast::Expr::LitByteStr(..) => (),
            ast::Expr::LitTemplate(..) => (),
            ast::Expr::LitTuple(..) => (),
            ast::Expr::LitVec(..) => (),
        }

        Ok(())
    }
}

impl Index<ast::ExprIf> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprIf) -> Result<(), CompileError> {
        self.index(&item.condition)?;
        self.index(&*item.block)?;

        for expr_else_if in &item.expr_else_ifs {
            self.index(&expr_else_if.condition)?;
            self.index(&*expr_else_if.block)?;
        }

        if let Some(expr_else) = &item.expr_else {
            self.index(&*expr_else.block)?;
        }

        Ok(())
    }
}

impl Index<ast::ExprBinary> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprBinary) -> Result<(), CompileError> {
        self.index(&*item.lhs)?;
        self.index(&*item.rhs)?;
        Ok(())
    }
}

impl Index<ast::ExprMatch> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprMatch) -> Result<(), CompileError> {
        self.index(&*item.expr)?;

        for (branch, _) in &item.branches {
            let span = branch.span();

            if let Some((_, condition)) = &branch.condition {
                self.index(&**condition)?;
            }

            let scope = self.scopes.push_scope();
            self.index(&branch.pat)?;
            self.index(&*branch.body)?;
            self.scopes.pop_scope(scope, span)?;
        }

        Ok(())
    }
}

impl Index<ast::Condition> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::Condition) -> Result<(), CompileError> {
        match item {
            ast::Condition::Expr(expr) => {
                self.index(&**expr)?;
            }
            ast::Condition::ExprLet(expr_let) => {
                self.index(&**expr_let)?;
            }
        }

        Ok(())
    }
}

impl Index<ast::Decl> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::Decl) -> Result<(), CompileError> {
        match item {
            ast::Decl::DeclUse(import) => {
                let name = import.path.resolve(self.query.source)?;
                let item = self.items.item();
                self.query.unit.borrow_mut().new_import(item, &name)?;
            }
            ast::Decl::DeclEnum(decl_enum) => {
                let span = decl_enum.span();
                let name = decl_enum.name.resolve(self.query.source)?;
                let guard = self.items.push_name(name);
                let enum_item = self.items.item();
                self.query.index_enum(enum_item.clone(), span)?;

                for (variant, body, _) in &decl_enum.variants {
                    let span = variant.span();
                    let variant = variant.resolve(self.query.source)?;
                    let guard = self.items.push_name(variant);

                    self.query.index_variant(
                        self.items.item(),
                        enum_item.clone(),
                        body.clone(),
                        span,
                    )?;

                    self.items.pop(guard, span)?;
                }

                self.items.pop(guard, span)?;
            }
            ast::Decl::DeclStruct(decl_struct) => {
                let span = decl_struct.span();
                let name = decl_struct.ident.resolve(self.query.source)?;
                let guard = self.items.push_name(name);
                self.query
                    .index_struct(self.items.item(), decl_struct.clone())?;
                self.items.pop(guard, span)?;
            }
            ast::Decl::DeclFn(decl_fn) => {
                let span = decl_fn.span();

                let is_toplevel = self.items.is_empty();

                let name = decl_fn.name.resolve(self.query.source)?;
                let guard = self.items.push_name(name);

                let item = self.items.item();

                // NB: immediately compile all toplevel functions.
                if is_toplevel {
                    self.query.queue.push_back((
                        item.clone(),
                        Build::Function(Function {
                            ast: decl_fn.clone(),
                        }),
                    ));

                    self.query
                        .unit
                        .borrow_mut()
                        .insert_meta(Meta::MetaFunction {
                            value_type: ValueType::Type(Hash::type_hash(&item)),
                            item,
                        })?;
                } else {
                    self.query.index(
                        item.clone(),
                        Indexed::Function(Function {
                            ast: decl_fn.clone(),
                        }),
                        span,
                    )?;
                }

                self.index(decl_fn)?;
                self.items.pop(guard, span)?;
            }
            ast::Decl::DeclImpl(decl_impl) => {
                let span = decl_impl.span();

                let mut guards = Vec::new();

                let first = decl_impl.path.first.resolve(self.query.source)?;
                guards.push(self.items.push_name(first));

                for (_, ident) in &decl_impl.path.rest {
                    let ident = ident.resolve(self.query.source)?;
                    guards.push(self.items.push_name(ident));
                }

                let instance_item = self.items.item();

                for decl_fn in &decl_impl.functions {
                    let name = decl_fn.name.resolve(self.query.source)?;
                    let guard = self.items.push_name(name);

                    let item = self.items.item();

                    // NB: all instance functions must be pre-emptively built,
                    // because statically we don't know if they will be used or
                    // not.
                    if decl_fn.is_instance() {
                        let f = InstanceFunction {
                            ast: decl_fn.clone(),
                            instance_item: instance_item.clone(),
                            instance_span: span,
                        };

                        self.query
                            .queue
                            .push_back((item.clone(), Build::InstanceFunction(f)));

                        let meta = Meta::MetaFunction {
                            value_type: ValueType::Type(Hash::type_hash(&item)),
                            item: item.clone(),
                        };

                        self.query.unit.borrow_mut().insert_meta(meta)?;
                    } else {
                        self.query.index(
                            item.clone(),
                            Indexed::Function(Function {
                                ast: decl_fn.clone(),
                            }),
                            span,
                        )?;
                    }

                    self.index(decl_fn)?;
                    self.items.pop(guard, span)?;
                }

                while let Some(guard) = guards.pop() {
                    self.items.pop(guard, span)?;
                }
            }
        }

        Ok(())
    }
}

impl Index<ast::Path> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::Path) -> Result<(), CompileError> {
        if let Some(ident) = item.try_as_ident() {
            let ident = ident.resolve(self.query.source)?;
            self.scopes.mark_use(ident);
        }

        Ok(())
    }
}

impl Index<ast::ExprWhile> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprWhile) -> Result<(), CompileError> {
        let span = item.span();
        let scope = self.scopes.push_scope();
        self.index(&item.condition)?;
        self.index(&*item.body)?;
        self.scopes.pop_scope(scope, span)?;
        Ok(())
    }
}

impl Index<ast::ExprLoop> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprLoop) -> Result<(), CompileError> {
        let span = item.span();
        let scope = self.scopes.push_scope();
        self.index(&*item.body)?;
        self.scopes.pop_scope(scope, span)?;
        Ok(())
    }
}

impl Index<ast::ExprFor> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprFor) -> Result<(), CompileError> {
        // NB: creating the iterator is evaluated in the parent scope.
        self.index(&*item.iter)?;
        let span = item.span();

        let scope = self.scopes.push_scope();
        self.index(&item.var)?;
        self.index(&*item.body)?;
        self.scopes.pop_scope(scope, span)?;

        Ok(())
    }
}

impl Index<ast::ExprClosure> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprClosure) -> Result<(), CompileError> {
        let span = item.span();
        let guard = self.items.push_closure();
        let closure_guard = self.scopes.push_closure();

        for (arg, _) in item.args.as_slice() {
            match arg {
                ast::FnArg::Self_(s) => {
                    return Err(CompileError::UnsupportedSelf { span: s.span() });
                }
                ast::FnArg::Ident(ident) => {
                    let ident = ident.resolve(self.query.source)?;
                    self.scopes.declare(ident, span)?;
                }
                ast::FnArg::Ignore(..) => (),
            }
        }

        self.index(&*item.body)?;

        let captures = Arc::new(self.scopes.pop_closure(closure_guard, span)?);
        self.query
            .index_closure(self.items.item(), item.clone(), captures)?;

        self.items.pop(guard, span)?;
        Ok(())
    }
}

impl Index<ast::ExprIndexSet> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprIndexSet) -> Result<(), CompileError> {
        self.index(&*item.value)?;
        self.index(&*item.index)?;
        self.index(&*item.target)?;
        Ok(())
    }
}

impl Index<ast::ExprFieldAccess> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprFieldAccess) -> Result<(), CompileError> {
        self.index(&*item.expr)?;
        Ok(())
    }
}

impl Index<ast::ExprUnary> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprUnary) -> Result<(), CompileError> {
        self.index(&*item.expr)?;
        Ok(())
    }
}

impl Index<ast::ExprIndexGet> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprIndexGet) -> Result<(), CompileError> {
        self.index(&*item.index)?;
        self.index(&*item.target)?;
        Ok(())
    }
}

impl Index<ast::ExprBreak> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprBreak) -> Result<(), CompileError> {
        if let Some(expr) = &item.expr {
            match expr {
                ast::ExprBreakValue::Expr(expr) => {
                    self.index(&**expr)?;
                }
                ast::ExprBreakValue::Label(..) => (),
            }
        }

        Ok(())
    }
}

impl Index<ast::ExprReturn> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprReturn) -> Result<(), CompileError> {
        if let Some(expr) = item.expr.as_deref() {
            self.index(expr)?;
        }

        Ok(())
    }
}

impl Index<ast::ExprAwait> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprAwait) -> Result<(), CompileError> {
        self.index(&*item.expr)?;
        Ok(())
    }
}

impl Index<ast::ExprTry> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprTry) -> Result<(), CompileError> {
        self.index(&*item.expr)?;
        Ok(())
    }
}

impl Index<ast::ExprSelect> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprSelect) -> Result<(), CompileError> {
        for (branch, _) in &item.branches {
            let span = branch.span();

            // NB: expression to evaluate future is evaled in parent scope.
            self.index(&*branch.expr)?;

            let scope = self.scopes.push_scope();
            self.index(&branch.pat)?;
            self.index(&*branch.body)?;
            self.scopes.pop_scope(scope, span)?;
        }

        if let Some((branch, _)) = &item.default_branch {
            let span = branch.span();

            let scope = self.scopes.push_scope();
            self.index(&*branch.body)?;
            self.scopes.pop_scope(scope, span)?;
        }

        Ok(())
    }
}

impl Index<ast::ExprCall> for Indexer<'_, '_> {
    fn index(&mut self, item: &ast::ExprCall) -> Result<(), CompileError> {
        for (expr, _) in item.args.items.iter().rev() {
            self.index(expr)?;
        }

        self.index(&*item.expr)?;
        Ok(())
    }
}
