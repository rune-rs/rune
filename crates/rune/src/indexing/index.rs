use crate::ast;
use crate::collections::HashMap;
use crate::indexing::IndexScopes;
use crate::load::{SourceLoader, Sources};
use crate::macros::{MacroCompiler, MacroContext};
use crate::parsing::Parse;
use crate::query::{
    Build, BuildEntry, Function, Indexed, IndexedEntry, InstanceFunction, Query, Used,
};
use crate::shared::Items;
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
use std::sync::Arc;

pub(crate) struct Indexer<'a> {
    /// The root URL that the indexed file originated from.
    pub(crate) root: Option<PathBuf>,
    /// Storage associated with the compilation.
    pub(crate) storage: Storage,
    pub(crate) loaded: &'a mut HashMap<Item, (SourceId, Span)>,
    pub(crate) query: &'a mut Query,
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
    /// Set if we are inside of an impl block.
    pub(crate) impl_items: Vec<Item>,
    pub(crate) visitor: &'a mut dyn CompileVisitor,
    pub(crate) source_loader: &'a mut dyn SourceLoader,
}

impl<'a> Indexer<'a> {
    /// Perform a macro expansion.
    fn expand_macro<T>(&mut self, ast: &ast::MacroCall) -> Result<T, CompileError>
    where
        T: Parse,
    {
        let mut macro_context = MacroContext::new(self.query.storage.clone(), self.source.clone());

        let mut compiler = MacroCompiler {
            storage: self.query.storage.clone(),
            item: self.items.item(),
            macro_context: &mut macro_context,
            options: self.options,
            context: self.context,
            unit: self.query.unit.clone(),
            source: self.source.clone(),
        };

        Ok(compiler.eval_macro::<T>(ast)?)
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
        let mut item_queue = items
            .drain(..)
            .map(ProcessEntry::Item)
            .collect::<VecDeque<_>>();

        while let Some(entry) = item_queue.pop_front() {
            match entry {
                ProcessEntry::Item((item, semi)) => {
                    match item {
                        ast::Item::ItemUse(item_use) => {
                            self.process_use(item_use)?;
                        }
                        // Expand items in case they are imports.
                        ast::Item::MacroCall(macro_call) => {
                            item_queue.push_back(ProcessEntry::MacroCall(macro_call));
                            continue;
                        }
                        item => {
                            items.push((item, semi));
                        }
                    }
                }
                ProcessEntry::MacroCall(macro_call) => {
                    let file = self.expand_macro::<ast::File>(&macro_call)?;
                    item_queue.extend(file.items.into_iter().map(ProcessEntry::Item));
                }
            }
        }

        return Ok(());

        enum ProcessEntry {
            Item((ast::Item, Option<ast::SemiColon>)),
            MacroCall(ast::MacroCall),
        }
    }

    /// Preprocess uses in statements.
    fn preprocess_stmts(&mut self, stmts: &mut Vec<ast::Stmt>) -> Result<(), CompileError> {
        let mut item_queue = stmts
            .drain(..)
            .map(ProcessEntry::Stmt)
            .collect::<VecDeque<_>>();

        while let Some(entry) = item_queue.pop_front() {
            match entry {
                ProcessEntry::Stmt(stmt) => match stmt {
                    ast::Stmt::Item(ast::Item::ItemUse(item_use), _) => {
                        self.process_use(item_use)?;
                    }
                    ast::Stmt::Item(ast::Item::MacroCall(macro_call), _) => {
                        item_queue.push_back(ProcessEntry::MacroCall(macro_call));
                        continue;
                    }
                    item => {
                        stmts.push(item);
                    }
                },
                ProcessEntry::MacroCall(macro_call) => {
                    let out = self.expand_macro::<Vec<ast::Stmt>>(&macro_call)?;
                    item_queue.extend(out.into_iter().map(ProcessEntry::Stmt));
                }
            }
        }

        return Ok(());

        enum ProcessEntry {
            Stmt(ast::Stmt),
            MacroCall(ast::MacroCall),
        }
    }

    /// Process a single use declaration.
    fn process_use(&mut self, item_use: ast::ItemUse) -> Result<(), CompileError> {
        if let Some(first) = item_use.attributes.first() {
            return Err(CompileError::internal(
                first,
                "use attributes are not supported",
            ));
        } else if !item_use.visibility.is_inherited() {
            return Err(CompileError::internal(
                &item_use.visibility.option_span().unwrap(),
                "use visibility levels are not supported",
            ));
        }

        let import = Import {
            item: self.items.item(),
            ast: item_use.clone(),
            source: self.source.clone(),
            source_id: self.source_id,
        };

        import.process(self.context, &self.query.storage, &self.query.unit)?;
        Ok(())
    }

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
        let source = self.source_loader.load(root, &item, span)?;

        if let Some(existing) = self.loaded.insert(item.clone(), (self.source_id, span)) {
            return Err(CompileError::new(
                span,
                CompileErrorKind::ModAlreadyLoaded { item, existing },
            ));
        }

        let source_id = self.sources.insert(source);
        self.visitor.visit_mod(source_id, span);

        self.queue.push_back(Task::LoadFile {
            kind: LoadFileKind::Module {
                root: self.root.clone(),
            },
            item,
            source_id,
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
        } else if !decl_fn.visibility.is_inherited() {
            return Err(CompileError::internal(
                &decl_fn.visibility.option_span().unwrap(),
                "function visibility levels are not supported",
            ));
        }

        let is_toplevel = self.items.is_empty();
        let name = decl_fn.name.resolve(&self.storage, &*self.source)?;
        let _guard = self.items.push_name(name.as_ref());

        let item = self.items.item();

        let guard = self.scopes.push_function(decl_fn.async_token.is_some());

        for (arg, _) in &decl_fn.args {
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

        self.index(&mut decl_fn.body)?;

        let f = guard.into_function(span)?;
        let call = Self::call(f.generator, f.is_async);

        let fun = Function {
            ast: decl_fn.clone(),
            call,
        };

        if decl_fn.is_instance() {
            let impl_item = self.impl_items.last().ok_or_else(|| {
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
            self.query.queue.push_back(BuildEntry {
                span: f.ast.span(),
                item: item.clone(),
                build: Build::InstanceFunction(f),
                source: self.source.clone(),
                source_id: self.source_id,
                used: Used::Used,
            });

            let meta = CompileMeta {
                kind: CompileMetaKind::Function {
                    type_of: Type::from(Hash::type_hash(&item)),
                    item,
                },
                source: Some(CompileSource {
                    span,
                    path: self.source.path().map(ToOwned::to_owned),
                    source_id: self.source_id,
                }),
            };

            self.query
                .unit
                .insert_meta(meta)
                .map_err(|e| CompileError::new(span, e))?;
        } else if is_toplevel {
            // NB: immediately compile all toplevel functions.
            self.query.queue.push_back(BuildEntry {
                span: fun.ast.span(),
                item: item.clone(),
                build: Build::Function(fun),
                source: self.source.clone(),
                source_id: self.source_id,
                used: Used::Used,
            });

            let meta = CompileMeta {
                kind: CompileMetaKind::Function {
                    type_of: Type::from(Hash::type_hash(&item)),
                    item,
                },
                source: Some(CompileSource {
                    span,
                    path: self.source.path().map(ToOwned::to_owned),
                    source_id: self.source_id,
                }),
            };

            self.query
                .unit
                .insert_meta(meta)
                .map_err(|e| CompileError::new(span, e))?;
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

        if expr_block.async_token.is_some() {
            let _guard = self.items.push_async_block();
            let guard = self.scopes.push_closure(true);
            self.index(&mut expr_block.block)?;

            let c = guard.into_closure(span)?;

            let captures = Arc::new(c.captures);
            let call = Self::call(c.generator, c.is_async);

            self.query.index_async_block(
                self.items.item(),
                expr_block.block.clone(),
                captures,
                call,
                self.source.clone(),
                self.source_id,
            )?;
        } else {
            self.index(&mut expr_block.block)?;
        }

        Ok(())
    }
}

impl Index<ast::Block> for Indexer<'_> {
    fn index(&mut self, block: &mut ast::Block) -> CompileResult<()> {
        let span = block.span();
        log::trace!("Block => {:?}", self.source.source(span));

        let _guard = self.items.push_block();
        let _guard = self.scopes.push_scope();

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
    fn index(&mut self, pat_tuple: &mut ast::PatTuple) -> CompileResult<()> {
        let span = pat_tuple.span();
        log::trace!("PatTuple => {:?}", self.source.source(span));

        for (pat, _) in &mut pat_tuple.items {
            self.index(&mut **pat)?;
        }

        Ok(())
    }
}

impl Index<ast::PatObject> for Indexer<'_> {
    fn index(&mut self, pat_object: &mut ast::PatObject) -> CompileResult<()> {
        let span = pat_object.span();
        log::trace!("PatObject => {:?}", self.source.source(span));

        for (field, _) in &mut pat_object.fields {
            if let Some((_, pat)) = &mut field.binding {
                self.index(pat)?;
            } else {
                match &mut field.key {
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
    fn index(&mut self, pat_vec: &mut ast::PatVec) -> CompileResult<()> {
        let span = pat_vec.span();
        log::trace!("PatVec => {:?}", self.source.source(span));

        for (pat, _) in &mut pat_vec.items {
            self.index(&mut **pat)?;
        }

        Ok(())
    }
}

impl Index<ast::Expr> for Indexer<'_> {
    fn index(&mut self, expr: &mut ast::Expr) -> CompileResult<()> {
        let span = expr.span();
        log::trace!("Expr => {:?}", self.source.source(span));

        if let Some(span) = expr.attributes_span() {
            return Err(CompileError::internal(span, "attributes are not supported"));
        }

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
            ast::Expr::ExprGroup(expr) => {
                self.index(&mut *expr.expr)?;
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
                } else if !item_enum.visibility.is_inherited() {
                    return Err(CompileError::internal(
                        &item_enum.visibility.option_span().unwrap(),
                        "enum visibility levels are not supported",
                    ));
                }

                let name = item_enum.name.resolve(&self.storage, &*self.source)?;
                let _guard = self.items.push_name(name.as_ref());

                let span = item_enum.span();
                let enum_item = self.items.item();

                self.query.index_enum(
                    enum_item.clone(),
                    self.source.clone(),
                    self.source_id,
                    span,
                )?;

                for (
                    ast::ItemVariant {
                        attributes,
                        name,
                        body,
                        ..
                    },
                    _,
                ) in &item_enum.variants
                {
                    if let Some(first) = attributes.first() {
                        return Err(CompileError::internal(
                            first,
                            "variant attributes are not supported yet",
                        ));
                    }

                    for (field, _) in body.fields() {
                        if let Some(first) = field.attributes.first() {
                            return Err(CompileError::internal(
                                first,
                                "field attributes are not supported",
                            ));
                        }
                    }

                    let span = name.span();
                    let name = name.resolve(&self.storage, &*self.source)?;
                    let _guard = self.items.push_name(name.as_ref());

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
            ast::Item::ItemStruct(item_struct) => {
                if let Some(first) = item_struct.attributes.first() {
                    return Err(CompileError::internal(
                        first,
                        "struct attributes are not supported",
                    ));
                } else if !item_struct.visibility.is_inherited() {
                    return Err(CompileError::internal(
                        &item_struct.visibility.option_span().unwrap(),
                        "struct visibility levels are not supported",
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
                            &field.visibility.option_span().unwrap(),
                            "field visibility levels are not supported",
                        ));
                    }
                }

                let ident = item_struct.ident.resolve(&self.storage, &*self.source)?;
                let _guard = self.items.push_name(ident.as_ref());

                self.query.index_struct(
                    self.items.item(),
                    item_struct.clone(),
                    self.source.clone(),
                    self.source_id,
                )?;
            }
            ast::Item::ItemFn(item_fn) => {
                self.index(&mut **item_fn)?;
            }
            ast::Item::ItemImpl(item_impl) => {
                if let Some(first) = item_impl.attributes.first() {
                    return Err(CompileError::internal(
                        first,
                        "impl attributes are not supported",
                    ));
                }

                let mut guards = Vec::new();

                for path_segment in item_impl.path.into_components() {
                    let ident_segment = path_segment
                        .try_as_ident()
                        .ok_or_else(|| CompileError::internal_unsupported_path(path_segment))?;
                    let ident = ident_segment.resolve(&self.storage, &*self.source)?;
                    guards.push(self.items.push_name(ident.as_ref()));
                }

                self.impl_items.push(self.items.item());

                for item_fn in &mut item_impl.functions {
                    self.index(item_fn)?;
                }

                self.impl_items.pop();
            }
            ast::Item::ItemMod(item_mod) => {
                if let Some(first) = item_mod.attributes.first() {
                    return Err(CompileError::internal(
                        first,
                        "module attributes are not supported",
                    ));
                } else if !item_mod.visibility.is_inherited() {
                    return Err(CompileError::internal(
                        &item_mod.visibility.option_span().unwrap(),
                        "module visibility levels are not supported",
                    ));
                }

                match &mut item_mod.body {
                    ast::ItemModBody::EmptyBody(..) => {
                        self.handle_file_mod(item_mod)?;
                    }
                    ast::ItemModBody::InlineBody(body) => {
                        let name = item_mod.name.resolve(&self.storage, &*self.source)?;
                        let _guard = self.items.push_name(name.as_ref());
                        self.index(&mut *body.file)?;
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

                self.index(item_const)?;

                self.query.index_const(
                    self.items.item(),
                    self.source.clone(),
                    self.source_id,
                    *item_const.expr.clone(),
                    span,
                )?;
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

        if let Some(ident) = path.try_as_ident() {
            let ident = ident.resolve(&self.storage, &*self.source)?;
            self.scopes.mark_use(ident.as_ref());
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
        let guard = self.scopes.push_closure(expr_closure.async_token.is_some());
        let span = expr_closure.span();

        for (arg, _) in expr_closure.args.as_slice() {
            match arg {
                ast::FnArg::Self_(s) => {
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
    fn index(&mut self, expr_index_set: &mut ast::ExprIndexSet) -> CompileResult<()> {
        let span = expr_index_set.span();
        log::trace!("ExprIndexSet => {:?}", self.source.source(span));

        self.index(&mut *expr_index_set.value)?;
        self.index(&mut *expr_index_set.index)?;
        self.index(&mut *expr_index_set.target)?;
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

impl Index<ast::ExprIndexGet> for Indexer<'_> {
    fn index(&mut self, expr_index_get: &mut ast::ExprIndexGet) -> CompileResult<()> {
        let span = expr_index_get.span();
        log::trace!("ExprIndexGet => {:?}", self.source.source(span));

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

        let mut template = lit_template.resolve(&self.storage, &*self.source)?;

        for c in &mut template.components {
            match c {
                ast::TemplateComponent::Expr(expr) => {
                    self.index(&mut **expr)?;
                }
                ast::TemplateComponent::String(..) => (),
            }
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

        for (assign, _) in &mut lit_object.assignments {
            if let Some((_, expr)) = &mut assign.assign {
                self.index(expr)?;
            }
        }

        Ok(())
    }
}
