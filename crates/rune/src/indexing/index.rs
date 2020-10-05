use crate::ast;
use crate::collections::HashMap;
use crate::indexing::{IndexFnKind, IndexScopes, Visibility};
use crate::load::{SourceLoader, Sources};
use crate::macros::MacroCompiler;
use crate::parsing::Parse;
use crate::query::{
    Build, BuildEntry, Function, Indexed, IndexedEntry, InstanceFunction, Query, QueryMod, Used,
};
use crate::shared::{Consts, Items, Location};
use crate::worker::{Import, LoadFileKind, Task};
use crate::{
    CompileError, CompileErrorKind, CompileResult, CompileVisitor, OptionSpanned as _, Options,
    Resolve as _, Spanned as _, Storage, Warnings,
};
use runestick::{
    Call, CompileMeta, CompileMetaKind, CompileSource, Context, Hash, Item, Source, SourceId, Span,
    Type,
};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

pub(crate) struct Indexer<'a> {
    /// The root URL that the indexed file originated from.
    pub(crate) root: Option<PathBuf>,
    /// Storage associated with the compilation.
    pub(crate) storage: Storage,
    /// Loaded modules.
    pub(crate) loaded: &'a mut HashMap<Item, (SourceId, Span)>,
    /// Query engine.
    pub(crate) query: Query,
    /// Constants storage.
    pub(crate) consts: Consts,
    /// Imports to process.
    pub(crate) queue: &'a mut VecDeque<Task>,
    /// Source builders.
    pub(crate) sources: &'a mut Sources,
    /// Native context.
    pub(crate) context: &'a Context,
    pub(crate) options: &'a Options,
    pub(crate) source_id: SourceId,
    pub(crate) source: Arc<Source>,
    pub(crate) warnings: &'a mut Warnings,
    pub(crate) items: Items,
    pub(crate) scopes: IndexScopes,
    /// The current module being indexed.
    pub(crate) mod_item: Rc<QueryMod>,
    /// Set if we are inside of an impl block.
    pub(crate) impl_item: Option<Rc<Item>>,
    pub(crate) visitor: &'a mut dyn CompileVisitor,
    pub(crate) source_loader: &'a mut dyn SourceLoader,
}

impl<'a> Indexer<'a> {
    /// Perform a macro expansion.
    fn expand_macro<T>(&mut self, ast: &mut ast::MacroCall) -> Result<T, CompileError>
    where
        T: Parse,
    {
        let id =
            self.query
                .insert_path(&self.mod_item, self.impl_item.as_ref(), &*self.items.item());
        ast.path.id = Some(id);

        let item = self.query.get_item(ast.span(), &*self.items.item())?;

        let mut compiler = MacroCompiler {
            item,
            storage: self.query.storage(),
            options: self.options,
            context: self.context,
            source: self.source.clone(),
            query: self.query.clone(),
            consts: self.consts.clone(),
        };

        let expanded = compiler.eval_macro::<T>(ast)?;
        self.query.remove_path_by_id(ast.path.id);
        Ok(expanded)
    }

    /// pre-process uses and expand item macros.
    ///
    /// Uses are processed first in a file, and once processed any potential
    /// macro expansions are expanded.
    /// If these produce uses, these are processed, and so forth.
    fn preprocess_items(
        &mut self,
        items: &mut Vec<(ast::Item, Option<ast::SemiColon>)>,
    ) -> Result<(), CompileError> {
        let mut queue = items.drain(..).collect::<VecDeque<_>>();

        while let Some((item, semi)) = queue.pop_front() {
            match item {
                ast::Item::ItemUse(item_use) => {
                    let visibility = Visibility::from_ast(&item_use.visibility)?;
                    let queue = &mut *self.queue;

                    let import = Import {
                        visibility,
                        item: &*self.items.item(),
                        source: &self.source,
                        source_id: self.source_id,
                        ast: item_use,
                    };

                    import.process(
                        &self.mod_item,
                        &self.context,
                        &self.storage,
                        &self.query,
                        |expand| {
                            queue.push_back(Task::ExpandUnitWildcard(expand));
                        },
                    )?;
                }
                ast::Item::MacroCall(mut macro_call) => {
                    let file = self.expand_macro::<ast::File>(&mut macro_call)?;

                    for entry in file.items.into_iter().rev() {
                        queue.push_front(entry);
                    }
                }
                item => {
                    items.push((item, semi));
                }
            }
        }

        Ok(())
    }

    /// Preprocess uses in statements.
    fn preprocess_stmts(&mut self, stmts: &mut Vec<ast::Stmt>) -> Result<(), CompileError> {
        let mut queue = stmts.drain(..).collect::<VecDeque<_>>();

        while let Some(stmt) = queue.pop_front() {
            match stmt {
                ast::Stmt::Item(ast::Item::ItemUse(item_use), _) => {
                    let visibility = Visibility::from_ast(&item_use.visibility)?;
                    let queue = &mut *self.queue;

                    let import = Import {
                        visibility,
                        item: &*self.items.item(),
                        source: &self.source,
                        source_id: self.source_id,
                        ast: item_use,
                    };

                    import.process(
                        &self.mod_item,
                        self.context,
                        &self.storage,
                        &self.query,
                        |expand| {
                            queue.push_back(Task::ExpandUnitWildcard(expand));
                        },
                    )?;
                }
                ast::Stmt::Item(ast::Item::MacroCall(mut macro_call), _) => {
                    let out = self.expand_macro::<Vec<ast::Stmt>>(&mut macro_call)?;

                    for stmt in out.into_iter().rev() {
                        queue.push_front(stmt);
                    }
                }
                item => {
                    stmts.push(item);
                }
            }
        }

        Ok(())
    }

    /// Construct the calling convention based on the parameters.
    fn call(generator: bool, kind: IndexFnKind) -> Option<Call> {
        match kind {
            IndexFnKind::None if generator => Some(Call::Generator),
            IndexFnKind::None => Some(Call::Immediate),
            IndexFnKind::Async if generator => Some(Call::Stream),
            IndexFnKind::Async => Some(Call::Async),
            IndexFnKind::Const => None,
        }
    }

    /// Handle a filesystem module.
    pub(crate) fn handle_file_mod(&mut self, item_mod: &mut ast::ItemMod) -> CompileResult<()> {
        let span = item_mod.span();
        let name = item_mod.name.resolve(&self.storage, &*self.source)?;
        let _guard = self.items.push_name(name.as_ref());

        let root = match &self.root {
            Some(root) => root,
            None => {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::UnsupportedModuleSource,
                ));
            }
        };

        let item = self.items.item();
        let visibility = Visibility::from_ast(&item_mod.visibility)?;
        let (id, mod_item) =
            self.query
                .insert_mod(self.source_id, item_mod.name_span(), &*item, visibility)?;
        item_mod.id = Some(id);

        let source = self.source_loader.load(root, &*item, span)?;

        if let Some(existing) = self.loaded.insert(item.clone(), (self.source_id, span)) {
            return Err(CompileError::new(
                span,
                CompileErrorKind::ModAlreadyLoaded {
                    item: item.clone(),
                    existing,
                },
            ));
        }

        let source_id = self.sources.insert(source);
        self.visitor.visit_mod(source_id, span);

        self.queue.push_back(Task::LoadFile {
            kind: LoadFileKind::Module {
                root: self.root.clone(),
            },
            source_id,
            mod_item,
        });

        Ok(())
    }
}

pub(crate) trait Index<T> {
    /// Walk the current type with the given item.
    fn index(&mut self, item: &mut T) -> CompileResult<()>;
}

impl Index<ast::File> for Indexer<'_> {
    fn index(&mut self, file: &mut ast::File) -> CompileResult<()> {
        if let Some(first) = file.attributes.first() {
            return Err(CompileError::internal(
                first,
                "file attributes are not supported",
            ));
        }

        self.preprocess_items(&mut file.items)?;

        for (item, semi_colon) in &mut file.items {
            if let Some(semi_colon) = semi_colon {
                if !item.needs_semi_colon() {
                    self.warnings
                        .uneccessary_semi_colon(self.source_id, semi_colon.span());
                }
            }

            self.index(item)?;
        }

        Ok(())
    }
}

impl Index<ast::ItemFn> for Indexer<'_> {
    fn index(&mut self, decl_fn: &mut ast::ItemFn) -> CompileResult<()> {
        let span = decl_fn.span();
        log::trace!("ItemFn => {:?}", self.source.source(span));

        if let Some(first) = decl_fn.attributes.first() {
            return Err(CompileError::internal(
                first,
                "function attributes are not supported",
            ));
        }

        let is_toplevel = self.items.is_empty();
        let name = decl_fn.name.resolve(&self.storage, &*self.source)?;
        let _guard = self.items.push_name(name.as_ref());

        let visibility = Visibility::from_ast(&decl_fn.visibility)?;
        let item = self.query.insert_new_item(
            self.source_id,
            span,
            &*self.items.item(),
            &self.mod_item,
            visibility,
        )?;

        let kind = match (decl_fn.const_token, decl_fn.async_token) {
            (Some(const_token), Some(async_token)) => {
                return Err(CompileError::new(
                    const_token.span().join(async_token.span()),
                    CompileErrorKind::FnConstAsyncConflict,
                ));
            }
            (Some(..), _) => IndexFnKind::Const,
            (_, Some(..)) => IndexFnKind::Async,
            _ => IndexFnKind::None,
        };

        if let (Some(const_token), Some(async_token)) = (decl_fn.const_token, decl_fn.async_token) {
            return Err(CompileError::new(
                const_token.span().join(async_token.span()),
                CompileErrorKind::FnConstAsyncConflict,
            ));
        }

        let guard = self.scopes.push_function(kind);

        for (arg, _) in &decl_fn.args {
            match arg {
                ast::FnArg::SelfValue(s) => {
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

        self.index(&mut decl_fn.body)?;

        let f = guard.into_function(span)?;
        decl_fn.id = Some(item.id);

        let call = match Self::call(f.generator, f.kind) {
            Some(call) => call,
            // const function.
            None => {
                if f.generator {
                    return Err(CompileError::new(
                        span,
                        CompileErrorKind::FnConstNotGenerator,
                    ));
                }

                self.query
                    .index_const_fn(&item, &self.source, decl_fn.clone())?;

                return Ok(());
            }
        };

        let fun = Function {
            ast: decl_fn.clone(),
            call,
        };

        if decl_fn.is_instance() {
            let impl_item = self.impl_item.as_ref().ok_or_else(|| {
                CompileError::new(span, CompileErrorKind::InstanceFunctionOutsideImpl)
            })?;

            let f = InstanceFunction {
                ast: fun.ast,
                impl_item: impl_item.clone(),
                instance_span: span,
                call: fun.call,
            };

            // NB: all instance functions must be pre-emptively built,
            // because statically we don't know if they will be used or
            // not.
            self.query.push_build_entry(BuildEntry {
                location: Location::new(self.source_id, f.ast.span()),
                item: item.clone(),
                build: Build::InstanceFunction(f),
                source: self.source.clone(),
                used: Used::Used,
            });

            let meta = CompileMeta {
                item: item.item.clone(),
                kind: CompileMetaKind::Function {
                    type_of: Type::from(Hash::type_hash(&item.item)),
                },
                source: Some(CompileSource {
                    span,
                    path: self.source.path().map(ToOwned::to_owned),
                    source_id: self.source_id,
                }),
            };

            self.query.insert_meta(span, meta)?;
        } else if is_toplevel {
            // NB: immediately compile all toplevel functions.
            self.query.push_build_entry(BuildEntry {
                location: Location::new(self.source_id, fun.ast.item_span()),
                item: item.clone(),
                build: Build::Function(fun),
                source: self.source.clone(),
                used: Used::Used,
            });

            let meta = CompileMeta {
                item: item.item.clone(),
                kind: CompileMetaKind::Function {
                    type_of: Type::from(Hash::type_hash(&item.item)),
                },
                source: Some(CompileSource {
                    span,
                    path: self.source.path().map(ToOwned::to_owned),
                    source_id: self.source_id,
                }),
            };

            self.query.insert_meta(span, meta)?;
        } else {
            self.query.index(IndexedEntry {
                query_item: item.clone(),
                source: self.source.clone(),
                indexed: Indexed::Function(fun),
            })?;
        }

        Ok(())
    }
}

impl Index<ast::ExprBlock> for Indexer<'_> {
    fn index(&mut self, expr_block: &mut ast::ExprBlock) -> CompileResult<()> {
        let span = expr_block.span();
        log::trace!("ExprBlock => {:?}", self.source.source(span));

        if let Some(span) = expr_block.attributes.option_span() {
            return Err(CompileError::internal(
                span,
                "block attributes are not supported yet",
            ));
        }

        if expr_block.async_token.is_none() {
            return self.index(&mut expr_block.block);
        }

        let _guard = self.items.push_async_block();
        let guard = self.scopes.push_closure(IndexFnKind::Async);

        let item = self.query.insert_new_item(
            self.source_id,
            span,
            &*self.items.item(),
            &self.mod_item,
            Visibility::Inherited,
        )?;
        expr_block.block.id = Some(item.id);

        self.index(&mut expr_block.block)?;

        let c = guard.into_closure(span)?;

        let captures = Arc::new(c.captures);

        let call = match Self::call(c.generator, c.kind) {
            Some(call) => call,
            None => {
                return Err(CompileError::new(span, CompileErrorKind::ClosureKind));
            }
        };

        self.query.index_async_block(
            &item,
            &self.source,
            expr_block.block.clone(),
            captures,
            call,
        )?;

        Ok(())
    }
}

impl Index<ast::Block> for Indexer<'_> {
    fn index(&mut self, block: &mut ast::Block) -> CompileResult<()> {
        let span = block.span();
        log::trace!("Block => {:?}", self.source.source(span));

        let _guard = self.items.push_block();
        let _guard = self.scopes.push_scope();

        self.query.insert_new_item(
            self.source_id,
            span,
            &*self.items.item(),
            &self.mod_item,
            Visibility::Inherited,
        )?;

        self.preprocess_stmts(&mut block.statements)?;
        let mut must_be_last = None;

        for stmt in &mut block.statements {
            if let Some(span) = must_be_last {
                return Err(CompileError::new(
                    span,
                    CompileErrorKind::ExpectedBlockSemiColon {
                        followed_span: stmt.span(),
                    },
                ));
            }

            match stmt {
                ast::Stmt::Local(local) => {
                    self.index(local)?;
                }
                ast::Stmt::Item(item, semi) => {
                    if let Some(semi) = semi {
                        if !item.needs_semi_colon() {
                            self.warnings
                                .uneccessary_semi_colon(self.source_id, semi.span());
                        }
                    }

                    self.index(item)?;
                }
                ast::Stmt::Expr(expr) => {
                    if expr.needs_semi() {
                        must_be_last = Some(expr.span());
                    }

                    self.index(expr)?;
                }
                ast::Stmt::Semi(expr, semi) => {
                    if !expr.needs_semi() {
                        self.warnings
                            .uneccessary_semi_colon(self.source_id, semi.span());
                    }

                    self.index(expr)?;
                }
            }
        }

        Ok(())
    }
}

impl Index<ast::Local> for Indexer<'_> {
    fn index(&mut self, local: &mut ast::Local) -> CompileResult<()> {
        let span = local.span();
        log::trace!("Local => {:?}", self.source.source(span));

        if let Some(span) = local.attributes.option_span() {
            return Err(CompileError::internal(span, "attributes are not supported"));
        }

        self.index(&mut local.pat)?;
        self.index(&mut *local.expr)?;
        Ok(())
    }
}

impl Index<ast::ExprLet> for Indexer<'_> {
    fn index(&mut self, expr_let: &mut ast::ExprLet) -> CompileResult<()> {
        let span = expr_let.span();
        log::trace!("ExprLet => {:?}", self.source.source(span));

        self.index(&mut expr_let.pat)?;
        self.index(&mut *expr_let.expr)?;
        Ok(())
    }
}

impl Index<ast::Ident> for Indexer<'_> {
    fn index(&mut self, ident: &mut ast::Ident) -> CompileResult<()> {
        let span = ident.span();
        log::trace!("Ident => {:?}", self.source.source(span));

        let ident = ident.resolve(&self.storage, &*self.source)?;
        self.scopes.declare(ident.as_ref(), span)?;
        Ok(())
    }
}

impl Index<ast::Pat> for Indexer<'_> {
    fn index(&mut self, pat: &mut ast::Pat) -> CompileResult<()> {
        let span = pat.span();
        log::trace!("Pat => {:?}", self.source.source(span));

        match pat {
            ast::Pat::PatPath(pat_path) => {
                self.index(&mut pat_path.path)?;

                if let Some(ident) = pat_path.path.try_as_ident_mut() {
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
            ast::Pat::PatBinding(pat_binding) => {
                self.index(pat_binding)?;
            }
            ast::Pat::PatIgnore(..) => (),
            ast::Pat::PatLit(..) => (),
            ast::Pat::PatRest(..) => (),
        }

        Ok(())
    }
}

impl Index<ast::PatTuple> for Indexer<'_> {
    fn index(&mut self, pat_tuple: &mut ast::PatTuple) -> CompileResult<()> {
        let span = pat_tuple.span();
        log::trace!("PatTuple => {:?}", self.source.source(span));

        if let Some(path) = &mut pat_tuple.path {
            self.index(path)?;
        }

        for (pat, _) in &mut pat_tuple.items {
            self.index(pat)?;
        }

        Ok(())
    }
}

impl Index<ast::PatBinding> for Indexer<'_> {
    fn index(&mut self, pat_binding: &mut ast::PatBinding) -> CompileResult<()> {
        let span = pat_binding.span();
        log::trace!("PatBinding => {:?}", self.source.source(span));
        self.index(&mut *pat_binding.pat)?;
        Ok(())
    }
}

impl Index<ast::PatObject> for Indexer<'_> {
    fn index(&mut self, pat_object: &mut ast::PatObject) -> CompileResult<()> {
        let span = pat_object.span();
        log::trace!("PatObject => {:?}", self.source.source(span));

        match &mut pat_object.ident {
            ast::LitObjectIdent::Anonymous(..) => (),
            ast::LitObjectIdent::Named(path) => {
                self.index(path)?;
            }
        }

        for (pat, _) in &mut pat_object.items {
            self.index(pat)?;
        }

        Ok(())
    }
}

impl Index<ast::PatVec> for Indexer<'_> {
    fn index(&mut self, pat_vec: &mut ast::PatVec) -> CompileResult<()> {
        let span = pat_vec.span();
        log::trace!("PatVec => {:?}", self.source.source(span));

        for (pat, _) in &mut pat_vec.items {
            self.index(pat)?;
        }

        Ok(())
    }
}

impl Index<ast::Expr> for Indexer<'_> {
    fn index(&mut self, expr: &mut ast::Expr) -> CompileResult<()> {
        let span = expr.span();
        log::trace!("Expr => {:?}", self.source.source(span));

        if let Some(span) = expr.attributes().option_span() {
            return Err(CompileError::internal(span, "attributes are not supported"));
        }

        match expr {
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
                self.index(&mut *expr.expr)?;
            }
            ast::Expr::ExprIf(expr_if) => {
                self.index(expr_if)?;
            }
            ast::Expr::ExprAssign(expr_assign) => {
                self.index(expr_assign)?;
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
            ast::Expr::ExprFieldAccess(expr_field_access) => {
                self.index(expr_field_access)?;
            }
            ast::Expr::ExprUnary(expr_unary) => {
                self.index(expr_unary)?;
            }
            ast::Expr::ExprIndex(expr_index_get) => {
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
            ast::Expr::ExprLit(expr_lit) => {
                self.index(expr_lit)?;
            }
            // NB: macros have nothing to index, they don't export language
            // items.
            ast::Expr::MacroCall(macro_call) => {
                let out = self.expand_macro::<ast::Expr>(macro_call)?;
                *expr = out;
                self.index(expr)?;
            }
        }

        Ok(())
    }
}

impl Index<ast::ExprIf> for Indexer<'_> {
    fn index(&mut self, expr_if: &mut ast::ExprIf) -> CompileResult<()> {
        let span = expr_if.span();
        log::trace!("ExprIf => {:?}", self.source.source(span));

        self.index(&mut expr_if.condition)?;
        self.index(&mut *expr_if.block)?;

        for expr_else_if in &mut expr_if.expr_else_ifs {
            self.index(&mut expr_else_if.condition)?;
            self.index(&mut *expr_else_if.block)?;
        }

        if let Some(expr_else) = &mut expr_if.expr_else {
            self.index(&mut *expr_else.block)?;
        }

        Ok(())
    }
}

impl Index<ast::ExprAssign> for Indexer<'_> {
    fn index(&mut self, expr_binary: &mut ast::ExprAssign) -> CompileResult<()> {
        let span = expr_binary.span();
        log::trace!("ExprAssign => {:?}", self.source.source(span));

        self.index(&mut *expr_binary.lhs)?;
        self.index(&mut *expr_binary.rhs)?;
        Ok(())
    }
}

impl Index<ast::ExprBinary> for Indexer<'_> {
    fn index(&mut self, expr_binary: &mut ast::ExprBinary) -> CompileResult<()> {
        let span = expr_binary.span();
        log::trace!("ExprBinary => {:?}", self.source.source(span));

        self.index(&mut *expr_binary.lhs)?;
        self.index(&mut *expr_binary.rhs)?;
        Ok(())
    }
}

impl Index<ast::ExprMatch> for Indexer<'_> {
    fn index(&mut self, expr_match: &mut ast::ExprMatch) -> CompileResult<()> {
        let span = expr_match.span();
        log::trace!("ExprMatch => {:?}", self.source.source(span));

        self.index(&mut *expr_match.expr)?;

        for (branch, _) in &mut expr_match.branches {
            if let Some((_, condition)) = &mut branch.condition {
                self.index(&mut **condition)?;
            }

            let _guard = self.scopes.push_scope();
            self.index(&mut branch.pat)?;
            self.index(&mut *branch.body)?;
        }

        Ok(())
    }
}

impl Index<ast::Condition> for Indexer<'_> {
    fn index(&mut self, condition: &mut ast::Condition) -> CompileResult<()> {
        let span = condition.span();
        log::trace!("Condition => {:?}", self.source.source(span));

        match condition {
            ast::Condition::Expr(expr) => {
                self.index(&mut **expr)?;
            }
            ast::Condition::ExprLet(expr_let) => {
                self.index(&mut **expr_let)?;
            }
        }

        Ok(())
    }
}

impl Index<ast::Item> for Indexer<'_> {
    fn index(&mut self, item: &mut ast::Item) -> CompileResult<()> {
        let span = item.span();
        log::trace!("Item => {:?}", self.source.source(span));

        match item {
            ast::Item::ItemEnum(item_enum) => {
                if let Some(first) = item_enum.attributes.first() {
                    return Err(CompileError::internal(
                        first,
                        "enum attributes are not supported",
                    ));
                }

                let name = item_enum.name.resolve(&self.storage, &*self.source)?;
                let _guard = self.items.push_name(name.as_ref());

                let visibility = Visibility::from_ast(&item_enum.visibility)?;
                let enum_item = self.query.insert_new_item(
                    self.source_id,
                    span,
                    &*self.items.item(),
                    &self.mod_item,
                    visibility,
                )?;

                self.query.index_enum(&enum_item, &self.source)?;

                for (variant, _) in &mut item_enum.variants {
                    if let Some(first) = variant.attributes.first() {
                        return Err(CompileError::internal(
                            first,
                            "variant attributes are not supported yet",
                        ));
                    }

                    for (field, _) in variant.body.fields() {
                        if let Some(first) = field.attributes.first() {
                            return Err(CompileError::internal(
                                first,
                                "field attributes are not supported",
                            ));
                        }
                    }

                    let span = variant.name.span();
                    let name = variant.name.resolve(&self.storage, &*self.source)?;
                    let _guard = self.items.push_name(name.as_ref());

                    let item = self.query.insert_new_item(
                        self.source_id,
                        span,
                        &*self.items.item(),
                        &self.mod_item,
                        visibility,
                    )?;
                    variant.id = Some(item.id);

                    self.query
                        .index_variant(&item, &self.source, enum_item.id, variant.clone())?;
                }
            }
            ast::Item::ItemStruct(item_struct) => {
                if let Some(first) = item_struct.attributes.first() {
                    return Err(CompileError::internal(
                        first,
                        "struct attributes are not supported",
                    ));
                }

                for (field, _) in item_struct.body.fields() {
                    if let Some(first) = field.attributes.first() {
                        return Err(CompileError::internal(
                            first,
                            "field attributes are not supported",
                        ));
                    } else if !field.visibility.is_inherited() {
                        return Err(CompileError::internal(
                            &field,
                            "field visibility levels are not supported",
                        ));
                    }
                }

                let ident = item_struct.ident.resolve(&self.storage, &*self.source)?;
                let _guard = self.items.push_name(ident.as_ref());

                let visibility = Visibility::from_ast(&item_struct.visibility)?;
                let item = self.query.insert_new_item(
                    self.source_id,
                    span,
                    &*self.items.item(),
                    &self.mod_item,
                    visibility,
                )?;
                item_struct.id = Some(item.id);

                self.query
                    .index_struct(&item, &self.source, item_struct.clone())?;
            }
            ast::Item::ItemFn(item_fn) => {
                self.index(item_fn)?;
            }
            ast::Item::ItemImpl(item_impl) => {
                if let Some(first) = item_impl.attributes.first() {
                    return Err(CompileError::internal(
                        first,
                        "impl attributes are not supported",
                    ));
                }

                let mut guards = Vec::new();

                if let Some(global) = &item_impl.path.global {
                    return Err(CompileError::internal(
                        global,
                        "global scopes are not supported yet",
                    ));
                }

                for path_segment in item_impl.path.into_components() {
                    let ident_segment = path_segment
                        .try_as_ident()
                        .ok_or_else(|| CompileError::internal_unsupported_path(path_segment))?;
                    let ident = ident_segment.resolve(&self.storage, &*self.source)?;
                    guards.push(self.items.push_name(ident.as_ref()));
                }

                let new = Rc::new(self.items.item().clone());
                let old = std::mem::replace(&mut self.impl_item, Some(new));

                for item_fn in &mut item_impl.functions {
                    self.index(item_fn)?;
                }

                self.impl_item = old;
            }
            ast::Item::ItemMod(item_mod) => {
                if let Some(first) = item_mod.attributes.first() {
                    return Err(CompileError::internal(
                        first,
                        "module attributes are not supported",
                    ));
                }

                let name_span = item_mod.name_span();

                match &mut item_mod.body {
                    ast::ItemModBody::EmptyBody(..) => {
                        self.handle_file_mod(item_mod)?;
                    }
                    ast::ItemModBody::InlineBody(body) => {
                        let name = item_mod.name.resolve(&self.storage, &*self.source)?;
                        let _guard = self.items.push_name(name.as_ref());

                        let visibility = Visibility::from_ast(&item_mod.visibility)?;
                        let (id, mod_item) = self.query.insert_mod(
                            self.source_id,
                            name_span,
                            &*self.items.item(),
                            visibility,
                        )?;
                        item_mod.id = Some(id);

                        let replaced = std::mem::replace(&mut self.mod_item, mod_item);
                        self.index(&mut *body.file)?;
                        self.mod_item = replaced;
                    }
                }
            }
            ast::Item::ItemConst(item_const) => {
                if let Some(first) = item_const.attributes.first() {
                    return Err(CompileError::internal(
                        first,
                        "attributes on constants are not supported",
                    ));
                }

                let span = item_const.span();
                let name = item_const.name.resolve(&self.storage, &*self.source)?;
                let _guard = self.items.push_name(name.as_ref());

                let item = self.query.insert_new_item(
                    self.source_id,
                    span,
                    &*self.items.item(),
                    &self.mod_item,
                    Visibility::from_ast(&item_const.visibility)?,
                )?;

                item_const.id = Some(item.id);

                self.index(item_const)?;

                self.query
                    .index_const(&item, &self.source, item_const.clone())?;
            }
            ast::Item::MacroCall(macro_call) => {
                let out = self.expand_macro::<ast::Item>(macro_call)?;
                *item = out;
                self.index(item)?;
            }
            // NB: imports are ignored during indexing.
            ast::Item::ItemUse(..) => {}
        }

        Ok(())
    }
}

impl Index<ast::ItemConst> for Indexer<'_> {
    fn index(&mut self, item_const: &mut ast::ItemConst) -> CompileResult<()> {
        self.index(&mut *item_const.expr)?;
        Ok(())
    }
}

impl Index<ast::Path> for Indexer<'_> {
    fn index(&mut self, path: &mut ast::Path) -> CompileResult<()> {
        let span = path.span();
        log::trace!("Path => {:?}", self.source.source(span));

        let id =
            self.query
                .insert_path(&self.mod_item, self.impl_item.as_ref(), &*self.items.item());
        path.id = Some(id);

        match path.as_kind() {
            Some(ast::PathKind::SelfValue) => {
                self.scopes.mark_use("self");
            }
            Some(ast::PathKind::Ident(ident)) => {
                let ident = ident.resolve(&self.storage, &*self.source)?;
                self.scopes.mark_use(ident.as_ref());
            }
            None => (),
        }

        Ok(())
    }
}

impl Index<ast::ExprWhile> for Indexer<'_> {
    fn index(&mut self, expr_while: &mut ast::ExprWhile) -> CompileResult<()> {
        let span = expr_while.span();
        log::trace!("ExprWhile => {:?}", self.source.source(span));

        let _guard = self.scopes.push_scope();
        self.index(&mut expr_while.condition)?;
        self.index(&mut *expr_while.body)?;
        Ok(())
    }
}

impl Index<ast::ExprLoop> for Indexer<'_> {
    fn index(&mut self, expr_loop: &mut ast::ExprLoop) -> CompileResult<()> {
        let span = expr_loop.span();
        log::trace!("ExprLoop => {:?}", self.source.source(span));

        let _guard = self.scopes.push_scope();
        self.index(&mut *expr_loop.body)?;
        Ok(())
    }
}

impl Index<ast::ExprFor> for Indexer<'_> {
    fn index(&mut self, expr_for: &mut ast::ExprFor) -> CompileResult<()> {
        let span = expr_for.span();
        log::trace!("ExprFor => {:?}", self.source.source(span));

        // NB: creating the iterator is evaluated in the parent scope.
        self.index(&mut *expr_for.iter)?;

        let _guard = self.scopes.push_scope();
        self.index(&mut expr_for.var)?;
        self.index(&mut *expr_for.body)?;
        Ok(())
    }
}

impl Index<ast::ExprClosure> for Indexer<'_> {
    fn index(&mut self, expr_closure: &mut ast::ExprClosure) -> CompileResult<()> {
        let span = expr_closure.span();
        log::trace!("ExprClosure => {:?}", self.source.source(span));

        let _guard = self.items.push_closure();

        let kind = match expr_closure.async_token {
            Some(..) => IndexFnKind::Async,
            _ => IndexFnKind::None,
        };

        let guard = self.scopes.push_closure(kind);
        let span = expr_closure.span();

        let item = self.query.insert_new_item(
            self.source_id,
            span,
            &*self.items.item(),
            &self.mod_item,
            Visibility::Inherited,
        )?;

        expr_closure.id = Some(item.id);

        for (arg, _) in expr_closure.args.as_slice() {
            match arg {
                ast::FnArg::SelfValue(s) => {
                    return Err(CompileError::new(s, CompileErrorKind::UnsupportedSelf));
                }
                ast::FnArg::Ident(ident) => {
                    let ident = ident.resolve(&self.storage, &*self.source)?;
                    self.scopes.declare(ident.as_ref(), span)?;
                }
                ast::FnArg::Ignore(..) => (),
            }
        }

        self.index(&mut *expr_closure.body)?;

        let c = guard.into_closure(span)?;

        let captures = Arc::new(c.captures);

        let call = match Self::call(c.generator, c.kind) {
            Some(call) => call,
            None => {
                return Err(CompileError::new(span, CompileErrorKind::ClosureKind));
            }
        };

        self.query
            .index_closure(&item, &self.source, expr_closure.clone(), captures, call)?;

        Ok(())
    }
}

impl Index<ast::ExprFieldAccess> for Indexer<'_> {
    fn index(&mut self, expr_field_access: &mut ast::ExprFieldAccess) -> CompileResult<()> {
        let span = expr_field_access.span();
        log::trace!("ExprIndexSet => {:?}", self.source.source(span));

        self.index(&mut *expr_field_access.expr)?;
        Ok(())
    }
}

impl Index<ast::ExprUnary> for Indexer<'_> {
    fn index(&mut self, expr_unary: &mut ast::ExprUnary) -> CompileResult<()> {
        let span = expr_unary.span();
        log::trace!("ExprUnary => {:?}", self.source.source(span));

        self.index(&mut *expr_unary.expr)?;
        Ok(())
    }
}

impl Index<ast::ExprIndex> for Indexer<'_> {
    fn index(&mut self, expr_index_get: &mut ast::ExprIndex) -> CompileResult<()> {
        let span = expr_index_get.span();
        log::trace!("ExprIndex => {:?}", self.source.source(span));

        self.index(&mut *expr_index_get.index)?;
        self.index(&mut *expr_index_get.target)?;
        Ok(())
    }
}

impl Index<ast::ExprBreak> for Indexer<'_> {
    fn index(&mut self, expr_break: &mut ast::ExprBreak) -> CompileResult<()> {
        let span = expr_break.span();
        log::trace!("ExprBreak => {:?}", self.source.source(span));

        if let Some(expr) = &mut expr_break.expr {
            match expr {
                ast::ExprBreakValue::Expr(expr) => {
                    self.index(&mut **expr)?;
                }
                ast::ExprBreakValue::Label(..) => (),
            }
        }

        Ok(())
    }
}

impl Index<ast::ExprYield> for Indexer<'_> {
    fn index(&mut self, expr_yield: &mut ast::ExprYield) -> CompileResult<()> {
        let span = expr_yield.span();
        log::trace!("ExprYield => {:?}", self.source.source(span));

        let span = expr_yield.span();
        self.scopes.mark_yield(span)?;

        if let Some(expr) = &mut expr_yield.expr {
            self.index(&mut **expr)?;
        }

        Ok(())
    }
}

impl Index<ast::ExprReturn> for Indexer<'_> {
    fn index(&mut self, expr_return: &mut ast::ExprReturn) -> CompileResult<()> {
        let span = expr_return.span();
        log::trace!("ExprReturn => {:?}", self.source.source(span));

        if let Some(expr) = expr_return.expr.as_deref_mut() {
            self.index(expr)?;
        }

        Ok(())
    }
}

impl Index<ast::ExprAwait> for Indexer<'_> {
    fn index(&mut self, expr_await: &mut ast::ExprAwait) -> CompileResult<()> {
        let span = expr_await.span();
        log::trace!("ExprAwait => {:?}", self.source.source(span));

        let span = expr_await.span();
        self.scopes.mark_await(span)?;
        self.index(&mut *expr_await.expr)?;
        Ok(())
    }
}

impl Index<ast::ExprTry> for Indexer<'_> {
    fn index(&mut self, expr_try: &mut ast::ExprTry) -> CompileResult<()> {
        let span = expr_try.span();
        log::trace!("ExprTry => {:?}", self.source.source(span));

        self.index(&mut *expr_try.expr)?;
        Ok(())
    }
}

impl Index<ast::ExprSelect> for Indexer<'_> {
    fn index(&mut self, expr_select: &mut ast::ExprSelect) -> CompileResult<()> {
        let span = expr_select.span();
        log::trace!("ExprSelect => {:?}", self.source.source(span));

        self.scopes.mark_await(expr_select.span())?;

        let mut default_branch = None;

        for (branch, _) in &mut expr_select.branches {
            match branch {
                ast::ExprSelectBranch::Pat(pat) => {
                    // NB: expression to evaluate future is evaled in parent scope.
                    self.index(&mut *pat.expr)?;

                    let _guard = self.scopes.push_scope();
                    self.index(&mut pat.pat)?;
                    self.index(&mut *pat.body)?;
                }
                ast::ExprSelectBranch::Default(def) => {
                    default_branch = Some(def);
                }
            }
        }

        if let Some(def) = default_branch {
            let _guard = self.scopes.push_scope();
            self.index(&mut *def.body)?;
        }

        Ok(())
    }
}

impl Index<ast::ExprCall> for Indexer<'_> {
    fn index(&mut self, expr_call: &mut ast::ExprCall) -> CompileResult<()> {
        let span = expr_call.span();
        log::trace!("ExprCall => {:?}", self.source.source(span));

        expr_call.id = Some(self.query.get_item_id(span, &*self.items.item())?);

        for (expr, _) in &mut expr_call.args {
            self.index(expr)?;
        }

        self.index(&mut *expr_call.expr)?;
        Ok(())
    }
}

impl Index<ast::ExprLit> for Indexer<'_> {
    fn index(&mut self, expr_lit: &mut ast::ExprLit) -> CompileResult<()> {
        if let Some(first) = expr_lit.attributes.first() {
            return Err(CompileError::internal(
                first,
                "literal attributes are not supported",
            ));
        }

        match &mut expr_lit.lit {
            ast::Lit::Template(lit_template) => {
                self.index(lit_template)?;
            }
            ast::Lit::Tuple(lit_tuple) => {
                self.index(lit_tuple)?;
            }
            ast::Lit::Vec(lit_vec) => {
                self.index(lit_vec)?;
            }
            ast::Lit::Object(lit_object) => {
                self.index(lit_object)?;
            }
            // NB: literals have nothing to index, they don't export language
            // items.
            ast::Lit::Unit(..) => (),
            ast::Lit::Bool(..) => (),
            ast::Lit::Byte(..) => (),
            ast::Lit::Char(..) => (),
            ast::Lit::Number(..) => (),
            ast::Lit::Str(..) => (),
            ast::Lit::ByteStr(..) => (),
        }

        Ok(())
    }
}

impl Index<ast::LitTemplate> for Indexer<'_> {
    fn index(&mut self, lit_template: &mut ast::LitTemplate) -> CompileResult<()> {
        let span = lit_template.span();
        log::trace!("LitTemplate => {:?}", self.source.source(span));

        for (expr, _) in &mut lit_template.args {
            self.index(expr)?;
        }

        Ok(())
    }
}

impl Index<ast::LitTuple> for Indexer<'_> {
    fn index(&mut self, lit_tuple: &mut ast::LitTuple) -> CompileResult<()> {
        let span = lit_tuple.span();
        log::trace!("LitTuple => {:?}", self.source.source(span));

        for (expr, _) in &mut lit_tuple.items {
            self.index(expr)?;
        }

        Ok(())
    }
}

impl Index<ast::LitVec> for Indexer<'_> {
    fn index(&mut self, lit_vec: &mut ast::LitVec) -> CompileResult<()> {
        let span = lit_vec.span();
        log::trace!("LitVec => {:?}", self.source.source(span));

        for (expr, _) in &mut lit_vec.items {
            self.index(expr)?;
        }

        Ok(())
    }
}

impl Index<ast::LitObject> for Indexer<'_> {
    fn index(&mut self, lit_object: &mut ast::LitObject) -> CompileResult<()> {
        let span = lit_object.span();
        log::trace!("LitObject => {:?}", self.source.source(span));

        match &mut lit_object.ident {
            ast::LitObjectIdent::Named(path) => {
                self.index(path)?;
            }
            ast::LitObjectIdent::Anonymous(..) => (),
        }

        for (assign, _) in &mut lit_object.assignments {
            if let Some((_, expr)) = &mut assign.assign {
                self.index(expr)?;
            }
        }

        Ok(())
    }
}
