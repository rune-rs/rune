use crate::ast;
use crate::collections::HashMap;
use crate::error::{CompileError, CompileResult};
use crate::index_scopes::IndexScopes;
use crate::items::Items;
use crate::query::{Build, BuildEntry, Function, Indexed, IndexedEntry, InstanceFunction, Query};
use crate::worker::{Import, Macro, MacroKind, Task};
use crate::{Resolve as _, SourceId, Sources, Storage, Warnings};
use runestick::{Call, CompileMeta, CompileMetaKind, Hash, Item, Source, Span, Type};
use std::collections::VecDeque;
use std::sync::Arc;

pub(crate) struct Indexer<'a> {
    pub(crate) storage: Storage,
    pub(crate) loaded: &'a mut HashMap<Item, (SourceId, Span)>,
    pub(crate) query: &'a mut Query,
    /// Imports to process.
    pub(crate) queue: &'a mut VecDeque<Task>,
    /// Source builders.
    pub(crate) sources: &'a mut Sources,
    /// Native context.
    pub(crate) source_id: SourceId,
    pub(crate) source: Arc<Source>,
    pub(crate) warnings: &'a mut Warnings,
    pub(crate) items: Items,
    pub(crate) scopes: IndexScopes,
    /// Set if we are inside of an impl block.
    pub(crate) impl_items: Vec<Item>,
}

impl<'a> Indexer<'a> {
    /// Construct the calling convention based on the parameters.
    fn call(generator: bool, is_async: bool) -> Call {
        if is_async {
            if generator {
                Call::Stream
            } else {
                Call::Async
            }
        } else if generator {
            Call::Generator
        } else {
            Call::Immediate
        }
    }

    /// Handle a filesystem module.
    pub(crate) fn handle_file_mod(&mut self, item_mod: &ast::ItemMod) -> CompileResult<()> {
        let span = item_mod.span();
        let name = item_mod.name.resolve(&self.storage, &*self.source)?;
        let _guard = self.items.push_name(name.as_ref());

        let path = match self.source.path() {
            Some(path) => path,
            None => {
                return Err(CompileError::UnsupportedFileMod { span });
            }
        };

        let base = match path.parent() {
            Some(parent) => parent.join(name.as_ref()),
            None => {
                return Err(CompileError::UnsupportedFileMod { span });
            }
        };

        let candidates = [
            base.join("mod").with_extension("rn"),
            base.with_extension("rn"),
        ];

        let mut found = None;

        for path in &candidates[..] {
            if path.is_file() {
                found = Some(path);
                break;
            }
        }

        let path = match found {
            Some(path) => path,
            None => {
                return Err(CompileError::ModNotFound { path: base, span });
            }
        };

        let item = self.items.item();

        if let Some(existing) = self.loaded.insert(item.clone(), (self.source_id, span)) {
            return Err(CompileError::ModAlreadyLoaded {
                item,
                span,
                existing,
            });
        }

        let source = match Source::from_path(path) {
            Ok(source) => source,
            Err(error) => {
                return Err(CompileError::ModFileError {
                    span,
                    path: path.to_owned(),
                    error,
                });
            }
        };

        self.sources.insert(item, source);
        Ok(())
    }
}

pub(crate) trait Index<T> {
    /// Walk the current type with the given item.
    fn index(&mut self, item: &T) -> CompileResult<()>;
}

impl Index<ast::File> for Indexer<'_> {
    fn index(&mut self, decl_file: &ast::File) -> CompileResult<()> {
        for (decl, semi_colon) in &decl_file.items {
            if let Some(semi_colon) = semi_colon {
                if !decl.needs_semi_colon() {
                    self.warnings
                        .uneccessary_semi_colon(self.source_id, semi_colon.span());
                }
            }

            self.index(decl)?;
        }

        Ok(())
    }
}

impl Index<ast::ItemFn> for Indexer<'_> {
    fn index(&mut self, decl_fn: &ast::ItemFn) -> CompileResult<()> {
        let span = decl_fn.span();
        let is_toplevel = self.items.is_empty();
        let name = decl_fn.name.resolve(&self.storage, &*self.source)?;
        let _guard = self.items.push_name(name.as_ref());

        let item = self.items.item();

        let guard = self.scopes.push_function(decl_fn.async_.is_some());

        for (arg, _) in &decl_fn.args.items {
            match arg {
                ast::FnArg::Self_(s) => {
                    let span = s.span();
                    self.scopes.declare("self", span)?;
                }
                ast::FnArg::Ident(ident) => {
                    let span = ident.span();
                    let ident = ident.resolve(&self.storage, &*self.source)?;
                    self.scopes.declare(ident.as_ref(), span)?;
                }
                _ => (),
            }
        }

        self.index(&decl_fn.body)?;

        let f = guard.into_function(span)?;
        let call = Self::call(f.generator, f.is_async);

        let fun = Function {
            ast: decl_fn.clone(),
            call,
        };

        if decl_fn.is_instance() {
            let impl_item = self
                .impl_items
                .last()
                .ok_or_else(|| CompileError::InstanceFunctionOutsideImpl { span })?;

            let f = InstanceFunction {
                ast: fun.ast,
                impl_item: impl_item.clone(),
                instance_span: span,
                call: fun.call,
            };

            // NB: all instance functions must be pre-emptively built,
            // because statically we don't know if they will be used or
            // not.
            self.query.queue.push_back(BuildEntry {
                item: item.clone(),
                build: Build::InstanceFunction(f),
                source: self.source.clone(),
                source_id: self.source_id,
                unused: false,
            });

            let meta = CompileMeta {
                span: Some(span),
                kind: CompileMetaKind::Function {
                    type_of: Type::from(Hash::type_hash(&item)),
                    item,
                },
            };

            self.query.unit.borrow_mut().insert_meta(meta)?;
        } else if is_toplevel {
            // NB: immediately compile all toplevel functions.
            self.query.queue.push_back(BuildEntry {
                item: item.clone(),
                build: Build::Function(fun),
                source: self.source.clone(),
                source_id: self.source_id,
                unused: false,
            });

            self.query.unit.borrow_mut().insert_meta(CompileMeta {
                span: Some(span),
                kind: CompileMetaKind::Function {
                    type_of: Type::from(Hash::type_hash(&item)),
                    item,
                },
            })?;
        } else {
            // NB: non toplevel functions can be indexed for later construction.
            self.query.index(
                item,
                IndexedEntry {
                    span,
                    source: self.source.clone(),
                    source_id: self.source_id,
                    indexed: Indexed::Function(fun),
                },
            )?;
        }

        Ok(())
    }
}

impl Index<ast::ExprAsync> for Indexer<'_> {
    fn index(&mut self, expr_async: &ast::ExprAsync) -> CompileResult<()> {
        let span = expr_async.span();

        let _guard = self.items.push_async_block();
        let guard = self.scopes.push_closure(true);
        self.index(&expr_async.block)?;

        let c = guard.into_closure(span)?;

        let captures = Arc::new(c.captures);
        let call = Self::call(c.generator, c.is_async);

        self.query.index_async_block(
            self.items.item(),
            expr_async.block.clone(),
            captures,
            call,
            self.source.clone(),
            self.source_id,
        )?;

        Ok(())
    }
}

impl Index<ast::ExprBlock> for Indexer<'_> {
    fn index(&mut self, expr_block: &ast::ExprBlock) -> CompileResult<()> {
        self.index(&expr_block.block)?;
        Ok(())
    }
}

impl Index<ast::Block> for Indexer<'_> {
    fn index(&mut self, block: &ast::Block) -> CompileResult<()> {
        let _guard = self.items.push_block();
        let _guard = self.scopes.push_scope();

        for stmt in &block.statements {
            self.index(stmt)?;
        }

        Ok(())
    }
}

impl Index<ast::Stmt> for Indexer<'_> {
    fn index(&mut self, item: &ast::Stmt) -> CompileResult<()> {
        match item {
            ast::Stmt::Item(decl) => self.index(decl),
            ast::Stmt::Expr(expr) => self.index(expr),
            ast::Stmt::Semi(expr, _) => self.index(expr),
        }
    }
}

impl Index<ast::ExprLet> for Indexer<'_> {
    fn index(&mut self, expr_let: &ast::ExprLet) -> CompileResult<()> {
        self.index(&expr_let.pat)?;
        self.index(&*expr_let.expr)?;
        Ok(())
    }
}

impl Index<ast::Ident> for Indexer<'_> {
    fn index(&mut self, ident: &ast::Ident) -> CompileResult<()> {
        let span = ident.span();
        let ident = ident.resolve(&self.storage, &*self.source)?;
        self.scopes.declare(ident.as_ref(), span)?;
        Ok(())
    }
}

impl Index<ast::Pat> for Indexer<'_> {
    fn index(&mut self, pat: &ast::Pat) -> CompileResult<()> {
        match pat {
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

impl Index<ast::PatTuple> for Indexer<'_> {
    fn index(&mut self, pat_tuple: &ast::PatTuple) -> CompileResult<()> {
        for (pat, _) in &pat_tuple.items {
            self.index(&**pat)?;
        }

        Ok(())
    }
}

impl Index<ast::PatObject> for Indexer<'_> {
    fn index(&mut self, pat_object: &ast::PatObject) -> CompileResult<()> {
        for (field, _) in &pat_object.fields {
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

impl Index<ast::PatVec> for Indexer<'_> {
    fn index(&mut self, pat_vec: &ast::PatVec) -> CompileResult<()> {
        for (pat, _) in &pat_vec.items {
            self.index(&**pat)?;
        }

        Ok(())
    }
}

impl Index<ast::Expr> for Indexer<'_> {
    fn index(&mut self, expr: &ast::Expr) -> CompileResult<()> {
        match expr {
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
            ast::Expr::ExprAsync(expr_async) => {
                self.index(expr_async)?;
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
            ast::Expr::Item(decl) => {
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
            ast::Expr::ExprYield(expr_yield) => {
                self.index(expr_yield)?;
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
            ast::Expr::LitTemplate(lit_template) => {
                self.index(lit_template)?;
            }
            // NB: literals have nothing to index, they don't export language
            // items.
            ast::Expr::LitUnit(..) => (),
            ast::Expr::LitBool(..) => (),
            ast::Expr::LitByte(..) => (),
            ast::Expr::LitChar(..) => (),
            ast::Expr::LitNumber(..) => (),
            ast::Expr::LitObject(..) => (),
            ast::Expr::LitStr(..) => (),
            ast::Expr::LitByteStr(..) => (),
            ast::Expr::LitTuple(..) => (),
            ast::Expr::LitVec(..) => (),
            // NB: macros have nothing to index, they don't export language
            // items.
            ast::Expr::MacroCall(expr_call_macro) => {
                let _guard = self.items.push_macro();

                self.queue.push_back(Task::ExpandMacro(Macro {
                    items: self.items.snapshot(),
                    ast: expr_call_macro.clone(),
                    source: self.source.clone(),
                    source_id: self.source_id,
                    scopes: self.scopes.snapshot(),
                    impl_items: self.impl_items.clone(),
                    kind: MacroKind::Expr,
                }));
            }
        }

        Ok(())
    }
}

impl Index<ast::ExprIf> for Indexer<'_> {
    fn index(&mut self, expr_if: &ast::ExprIf) -> CompileResult<()> {
        self.index(&expr_if.condition)?;
        self.index(&*expr_if.block)?;

        for expr_else_if in &expr_if.expr_else_ifs {
            self.index(&expr_else_if.condition)?;
            self.index(&*expr_else_if.block)?;
        }

        if let Some(expr_else) = &expr_if.expr_else {
            self.index(&*expr_else.block)?;
        }

        Ok(())
    }
}

impl Index<ast::ExprBinary> for Indexer<'_> {
    fn index(&mut self, expr_binary: &ast::ExprBinary) -> CompileResult<()> {
        self.index(&*expr_binary.lhs)?;
        self.index(&*expr_binary.rhs)?;
        Ok(())
    }
}

impl Index<ast::ExprMatch> for Indexer<'_> {
    fn index(&mut self, expr_match: &ast::ExprMatch) -> CompileResult<()> {
        self.index(&*expr_match.expr)?;

        for (branch, _) in &expr_match.branches {
            if let Some((_, condition)) = &branch.condition {
                self.index(&**condition)?;
            }

            let _guard = self.scopes.push_scope();
            self.index(&branch.pat)?;
            self.index(&*branch.body)?;
        }

        Ok(())
    }
}

impl Index<ast::Condition> for Indexer<'_> {
    fn index(&mut self, condition: &ast::Condition) -> CompileResult<()> {
        match condition {
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

impl Index<ast::Item> for Indexer<'_> {
    fn index(&mut self, decl: &ast::Item) -> CompileResult<()> {
        match decl {
            ast::Item::ItemUse(import) => {
                self.queue.push_back(Task::Import(Import {
                    item: self.items.item(),
                    ast: import.clone(),
                    source: self.source.clone(),
                    source_id: self.source_id,
                }));
            }
            ast::Item::ItemEnum(decl_enum) => {
                let name = decl_enum.name.resolve(&self.storage, &*self.source)?;
                let _guard = self.items.push_name(name.as_ref());

                let span = decl_enum.span();
                let enum_item = self.items.item();

                self.query.index_enum(
                    enum_item.clone(),
                    self.source.clone(),
                    self.source_id,
                    span,
                )?;

                for (variant, body, _) in &decl_enum.variants {
                    let variant_ident = variant.resolve(&self.storage, &*self.source)?;
                    let _guard = self.items.push_name(variant_ident.as_ref());

                    let span = variant.span();

                    self.query.index_variant(
                        self.items.item(),
                        enum_item.clone(),
                        body.clone(),
                        self.source.clone(),
                        self.source_id,
                        span,
                    )?;
                }
            }
            ast::Item::ItemStruct(decl_struct) => {
                let ident = decl_struct.ident.resolve(&self.storage, &*self.source)?;
                let _guard = self.items.push_name(ident.as_ref());

                self.query.index_struct(
                    self.items.item(),
                    decl_struct.clone(),
                    self.source.clone(),
                    self.source_id,
                )?;
            }
            ast::Item::ItemFn(decl_fn) => {
                self.index(decl_fn)?;
            }
            ast::Item::ItemImpl(decl_impl) => {
                let mut guards = Vec::new();

                for ident in decl_impl.path.components() {
                    let ident = ident.resolve(&self.storage, &*self.source)?;
                    guards.push(self.items.push_name(ident.as_ref()));
                }

                self.impl_items.push(self.items.item());

                for decl_fn in &decl_impl.functions {
                    self.index(decl_fn)?;
                }

                self.impl_items.pop();
            }
            ast::Item::ItemMod(item_mod) => match &item_mod.body {
                ast::ItemModBody::EmptyBody(..) => {
                    self.handle_file_mod(item_mod)?;
                }
                ast::ItemModBody::InlineBody(body) => {
                    let name = item_mod.name.resolve(&self.storage, &*self.source)?;
                    let _guard = self.items.push_name(name.as_ref());
                    self.index(&*body.file)?;
                }
            },
            ast::Item::MacroCall(expr_call_macro) => {
                let _guard = self.items.push_macro();

                self.queue.push_back(Task::ExpandMacro(Macro {
                    items: self.items.snapshot(),
                    ast: expr_call_macro.clone(),
                    source: self.source.clone(),
                    source_id: self.source_id,
                    scopes: self.scopes.snapshot(),
                    impl_items: self.impl_items.clone(),
                    kind: MacroKind::Item,
                }));
            }
        }

        Ok(())
    }
}

impl Index<ast::Path> for Indexer<'_> {
    fn index(&mut self, path: &ast::Path) -> CompileResult<()> {
        if let Some(ident) = path.try_as_ident() {
            let ident = ident.resolve(&self.storage, &*self.source)?;
            self.scopes.mark_use(ident.as_ref());
        }

        Ok(())
    }
}

impl Index<ast::ExprWhile> for Indexer<'_> {
    fn index(&mut self, expr_while: &ast::ExprWhile) -> CompileResult<()> {
        let _guard = self.scopes.push_scope();
        self.index(&expr_while.condition)?;
        self.index(&*expr_while.body)?;
        Ok(())
    }
}

impl Index<ast::ExprLoop> for Indexer<'_> {
    fn index(&mut self, expr_loop: &ast::ExprLoop) -> CompileResult<()> {
        let _guard = self.scopes.push_scope();
        self.index(&*expr_loop.body)?;
        Ok(())
    }
}

impl Index<ast::ExprFor> for Indexer<'_> {
    fn index(&mut self, expr_for: &ast::ExprFor) -> CompileResult<()> {
        // NB: creating the iterator is evaluated in the parent scope.
        self.index(&*expr_for.iter)?;

        let _guard = self.scopes.push_scope();
        self.index(&expr_for.var)?;
        self.index(&*expr_for.body)?;
        Ok(())
    }
}

impl Index<ast::ExprClosure> for Indexer<'_> {
    fn index(&mut self, expr_closure: &ast::ExprClosure) -> CompileResult<()> {
        let _guard = self.items.push_closure();
        let guard = self.scopes.push_closure(expr_closure.async_.is_some());
        let span = expr_closure.span();

        for (arg, _) in expr_closure.args.as_slice() {
            match arg {
                ast::FnArg::Self_(s) => {
                    return Err(CompileError::UnsupportedSelf { span: s.span() });
                }
                ast::FnArg::Ident(ident) => {
                    let ident = ident.resolve(&self.storage, &*self.source)?;
                    self.scopes.declare(ident.as_ref(), span)?;
                }
                ast::FnArg::Ignore(..) => (),
            }
        }

        self.index(&*expr_closure.body)?;

        let c = guard.into_closure(span)?;

        let captures = Arc::new(c.captures);
        let call = Self::call(c.generator, c.is_async);

        self.query.index_closure(
            self.items.item(),
            expr_closure.clone(),
            captures,
            call,
            self.source.clone(),
            self.source_id,
        )?;

        Ok(())
    }
}

impl Index<ast::ExprIndexSet> for Indexer<'_> {
    fn index(&mut self, expr_index_set: &ast::ExprIndexSet) -> CompileResult<()> {
        self.index(&*expr_index_set.value)?;
        self.index(&*expr_index_set.index)?;
        self.index(&*expr_index_set.target)?;
        Ok(())
    }
}

impl Index<ast::ExprFieldAccess> for Indexer<'_> {
    fn index(&mut self, expr_field_access: &ast::ExprFieldAccess) -> CompileResult<()> {
        self.index(&*expr_field_access.expr)?;
        Ok(())
    }
}

impl Index<ast::ExprUnary> for Indexer<'_> {
    fn index(&mut self, expr_unary: &ast::ExprUnary) -> CompileResult<()> {
        self.index(&*expr_unary.expr)?;
        Ok(())
    }
}

impl Index<ast::ExprIndexGet> for Indexer<'_> {
    fn index(&mut self, expr_index_get: &ast::ExprIndexGet) -> CompileResult<()> {
        self.index(&*expr_index_get.index)?;
        self.index(&*expr_index_get.target)?;
        Ok(())
    }
}

impl Index<ast::ExprBreak> for Indexer<'_> {
    fn index(&mut self, expr_break: &ast::ExprBreak) -> CompileResult<()> {
        if let Some(expr) = &expr_break.expr {
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

impl Index<ast::ExprYield> for Indexer<'_> {
    fn index(&mut self, expr_yield: &ast::ExprYield) -> CompileResult<()> {
        let span = expr_yield.span();
        self.scopes.mark_yield(span)?;

        if let Some(expr) = &expr_yield.expr {
            self.index(&**expr)?;
        }

        Ok(())
    }
}

impl Index<ast::ExprReturn> for Indexer<'_> {
    fn index(&mut self, expr_return: &ast::ExprReturn) -> CompileResult<()> {
        if let Some(expr) = expr_return.expr.as_deref() {
            self.index(expr)?;
        }

        Ok(())
    }
}

impl Index<ast::ExprAwait> for Indexer<'_> {
    fn index(&mut self, expr_await: &ast::ExprAwait) -> CompileResult<()> {
        let span = expr_await.span();
        self.scopes.mark_await(span)?;
        self.index(&*expr_await.expr)?;
        Ok(())
    }
}

impl Index<ast::ExprTry> for Indexer<'_> {
    fn index(&mut self, expr_try: &ast::ExprTry) -> CompileResult<()> {
        self.index(&*expr_try.expr)?;
        Ok(())
    }
}

impl Index<ast::ExprSelect> for Indexer<'_> {
    fn index(&mut self, expr_select: &ast::ExprSelect) -> CompileResult<()> {
        self.scopes.mark_await(expr_select.span())?;

        for (branch, _) in &expr_select.branches {
            // NB: expression to evaluate future is evaled in parent scope.
            self.index(&*branch.expr)?;

            let _guard = self.scopes.push_scope();
            self.index(&branch.pat)?;
            self.index(&*branch.body)?;
        }

        if let Some((branch, _)) = &expr_select.default_branch {
            let _guard = self.scopes.push_scope();
            self.index(&*branch.body)?;
        }

        Ok(())
    }
}

impl Index<ast::ExprCall> for Indexer<'_> {
    fn index(&mut self, expr_call: &ast::ExprCall) -> CompileResult<()> {
        for (expr, _) in expr_call.args.items.iter() {
            self.index(expr)?;
        }

        self.index(&*expr_call.expr)?;
        Ok(())
    }
}

impl Index<ast::LitTemplate> for Indexer<'_> {
    fn index(&mut self, lit_template: &ast::LitTemplate) -> CompileResult<()> {
        let template = lit_template.resolve(&self.storage, &*self.source)?;

        for c in &template.components {
            match c {
                ast::TemplateComponent::Expr(expr) => {
                    self.index(&**expr)?;
                }
                ast::TemplateComponent::String(..) => (),
            }
        }

        Ok(())
    }
}
