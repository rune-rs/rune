use crate::ast;
use crate::attrs;
use crate::collections::HashMap;
use crate::indexing::{IndexFnKind, IndexLocal as _, IndexScopes};
use crate::load::{SourceLoader, Sources};
use crate::macros::MacroCompiler;
use crate::parsing::{Parse, Parser};
use crate::query::{
    Build, BuildEntry, BuiltInFile, BuiltInFormat, BuiltInLine, BuiltInMacro, BuiltInTemplate,
    Function, Indexed, IndexedEntry, InstanceFunction, Query, Used,
};
use crate::shared::{Consts, Items};
use crate::worker::{Import, ImportKind, LoadFileKind, Task};
use crate::{
    CompileError, CompileErrorKind, CompileResult, CompileVisitor, Diagnostics, OptionSpanned as _,
    Options, ParseError, Resolve as _, Spanned as _, Storage,
};
use runestick::format;
use runestick::{
    Call, CompileMeta, CompileMetaKind, CompileMod, CompileSource, Context, Hash, Item, Location,
    Source, SourceId, Span, Visibility,
};
use std::collections::VecDeque;
use std::num::NonZeroUsize;
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
    pub(crate) diagnostics: &'a mut Diagnostics,
    pub(crate) items: Items,
    pub(crate) scopes: IndexScopes,
    /// The current module being indexed.
    pub(crate) mod_item: Arc<CompileMod>,
    /// Set if we are inside of an impl self.
    pub(crate) impl_item: Option<Arc<Item>>,
    pub(crate) visitor: Rc<dyn CompileVisitor>,
    pub(crate) source_loader: Rc<dyn SourceLoader + 'a>,
    /// Indicates if indexer is nested privately inside of another item, and if
    /// so, the descriptive span of its declaration.
    ///
    /// Private items are nested declarations inside of for example fn
    /// declarations:
    ///
    /// ```text
    /// pub fn public() {
    ///     fn private() {
    ///     }
    /// }
    /// ```
    ///
    /// Then, `nested_item` would point to the span of `pub fn public`.
    pub(crate) nested_item: Option<Span>,
}

impl<'a> Indexer<'a> {
    /// Try to expand an internal macro.
    fn try_expand_internal_macro(
        &mut self,
        attributes: &mut attrs::Attributes,
        ast: &mut ast::MacroCall,
    ) -> Result<bool, CompileError> {
        let (_, builtin) = match attributes.try_parse::<attrs::BuiltIn>()? {
            Some(builtin) => builtin,
            None => return Ok(false),
        };

        let args = builtin.args(&self.storage, &self.source)?;

        // NB: internal macros are
        let ident = match ast.path.try_as_ident() {
            Some(ident) => ident,
            None => {
                return Err(CompileError::new(
                    ast.path.span(),
                    CompileErrorKind::NoSuchBuiltInMacro {
                        name: ast.path.resolve(&self.storage, &self.source)?,
                    },
                ))
            }
        };

        let ident = ident.resolve(&self.storage, &self.source)?;

        let mut internal_macro = match ident.as_ref() {
            "template" => self.expand_template_macro(ast, &args)?,
            "format" => self.expand_format_macro(ast, &args)?,
            "file" => self.expand_file_macro(ast)?,
            "line" => self.expand_line_macro(ast)?,
            _ => {
                return Err(CompileError::new(
                    ast.path.span(),
                    CompileErrorKind::NoSuchBuiltInMacro {
                        name: ast.path.resolve(&self.storage, &self.source)?,
                    },
                ))
            }
        };

        match &mut internal_macro {
            BuiltInMacro::Template(template) => {
                for expr in &mut template.exprs {
                    expr.index(self)?;
                }
            }
            BuiltInMacro::Format(format) => {
                format.value.index(self)?;
            }

            BuiltInMacro::Line(_) | BuiltInMacro::File(_) => { /* Nothing to index */ }
        }

        let id = self.query.insert_new_builtin_macro(internal_macro)?;
        ast.id = Some(id);
        Ok(true)
    }

    /// Expand the template macro.
    fn expand_template_macro(
        &mut self,
        ast: &mut ast::MacroCall,
        args: &attrs::BuiltInArgs,
    ) -> Result<BuiltInMacro, ParseError> {
        let mut p = Parser::from_token_stream(&ast.stream);
        let mut exprs = Vec::new();

        while !p.is_eof()? {
            exprs.push(p.parse::<ast::Expr>()?);

            if p.parse::<Option<T![,]>>()?.is_none() {
                break;
            }
        }

        p.eof()?;

        Ok(BuiltInMacro::Template(BuiltInTemplate {
            span: ast.span(),
            from_literal: args.literal,
            exprs,
        }))
    }

    /// Expand the template macro.
    fn expand_format_macro(
        &mut self,
        ast: &mut ast::MacroCall,
        _: &attrs::BuiltInArgs,
    ) -> Result<BuiltInMacro, ParseError> {
        let mut p = Parser::from_token_stream(&ast.stream);

        let value = p.parse::<ast::Expr>()?;

        // parsed options
        let mut fill = None;
        let mut align = None;
        let mut flags = None;
        let mut width = None;
        let mut precision = None;
        let mut format_type = None;

        while p.try_consume::<T![,]>()? && !p.is_eof()? {
            let key = p.parse::<ast::Ident>()?;
            let _ = p.parse::<T![=]>()?;

            let k = key.resolve(&self.storage, &self.source)?;

            match k.as_ref() {
                "fill" => {
                    if fill.is_some() {
                        return Err(ParseError::unsupported(
                            key.span(),
                            "multiple `format!(.., fill = ..)`",
                        ));
                    }

                    let arg = p.parse::<ast::LitChar>()?;
                    let f = arg.resolve(&self.storage, &self.source)?;

                    fill = Some((arg, f));
                }
                "align" => {
                    if align.is_some() {
                        return Err(ParseError::unsupported(
                            key.span(),
                            "multiple `format!(.., align = ..)`",
                        ));
                    }

                    let arg = p.parse::<ast::Ident>()?;
                    let a = arg.resolve(&self.storage, &self.source)?;

                    align = Some(match str::parse::<format::Alignment>(a.as_ref()) {
                        Ok(a) => (arg, a),
                        _ => {
                            return Err(ParseError::unsupported(
                                key.span(),
                                "`format!(.., align = ..)`",
                            ));
                        }
                    });
                }
                "flags" => {
                    if flags.is_some() {
                        return Err(ParseError::unsupported(
                            key.span(),
                            "multiple `format!(.., flags = ..)`",
                        ));
                    }

                    let arg = p.parse::<ast::LitNumber>()?;
                    let f = arg
                        .resolve(&self.storage, &self.source)?
                        .as_u32(arg.span(), false)?;

                    let f = format::Flags::from(f);
                    flags = Some((arg, f));
                }
                "width" => {
                    if width.is_some() {
                        return Err(ParseError::unsupported(
                            key.span(),
                            "multiple `format!(.., width = ..)`",
                        ));
                    }

                    let arg = p.parse::<ast::LitNumber>()?;
                    let f = arg
                        .resolve(&self.storage, &self.source)?
                        .as_usize(arg.span(), false)?;

                    width = Some((arg, NonZeroUsize::new(f)));
                }
                "precision" => {
                    if precision.is_some() {
                        return Err(ParseError::unsupported(
                            key.span(),
                            "multiple `format!(.., precision = ..)`",
                        ));
                    }

                    let arg = p.parse::<ast::LitNumber>()?;
                    let f = arg
                        .resolve(&self.storage, &self.source)?
                        .as_usize(arg.span(), false)?;

                    precision = Some((arg, NonZeroUsize::new(f)));
                }
                "type" => {
                    if format_type.is_some() {
                        return Err(ParseError::unsupported(
                            key.span(),
                            "multiple `format!(.., type = ..)`",
                        ));
                    }

                    let arg = p.parse::<ast::Ident>()?;
                    let a = arg.resolve(&self.storage, &self.source)?;

                    format_type = Some(match str::parse::<format::Type>(a.as_ref()) {
                        Ok(format_type) => (arg, format_type),
                        _ => {
                            return Err(ParseError::unsupported(
                                key.span(),
                                "`format!(.., type = ..)`",
                            ));
                        }
                    });
                }
                _ => {
                    return Err(ParseError::unsupported(key.span(), "`format!(.., <key>)`"));
                }
            }
        }

        p.eof()?;
        Ok(BuiltInMacro::Format(Box::new(BuiltInFormat {
            span: ast.span(),
            fill,
            align,
            width,
            precision,
            flags,
            format_type,
            value,
        })))
    }

    /// Expand a macro returning the current file
    fn expand_file_macro(&mut self, ast: &mut ast::MacroCall) -> Result<BuiltInMacro, ParseError> {
        let id = self.storage.insert_str(self.source.name());
        let source = ast::StrSource::Synthetic(id);
        let node = ast::LitStr {
            token: ast::Token {
                kind: ast::Kind::Str(ast::StrSource::Synthetic(id)),
                span: ast.span(),
            },
            source,
        };
        Ok(BuiltInMacro::File(BuiltInFile {
            span: ast.span(),
            value: node,
        }))
    }

    /// Expand a macro returning the current line for where the macro invocation begins
    fn expand_line_macro(&mut self, ast: &mut ast::MacroCall) -> Result<BuiltInMacro, ParseError> {
        let (l, _) = self
            .source
            .position_to_utf16cu_line_char(ast.open.span.start.into_usize())
            .unwrap_or((0, 0));

        let id = self.storage.insert_number(l + 1); // 1-indexed as that is what most editors will use
        let source = ast::NumberSource::Synthetic(id);

        Ok(BuiltInMacro::Line(BuiltInLine {
            span: ast.span(),

            value: ast::LitNumber {
                token: ast::Token {
                    kind: ast::Kind::Number(source),
                    span: ast.span(),
                },
                source,
            },
        }))
    }

    /// Perform a macro expansion.
    fn expand_macro<T>(&mut self, ast: &mut ast::MacroCall) -> Result<T, CompileError>
    where
        T: Parse,
    {
        let id =
            self.query
                .insert_path(&self.mod_item, self.impl_item.as_ref(), &*self.items.item());
        ast.path.id = Some(id);

        let item = self.query.get_item(ast.span(), self.items.id())?;

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
        items: &mut Vec<(ast::Item, Option<T![;]>)>,
    ) -> Result<(), CompileError> {
        let mut queue = items.drain(..).collect::<VecDeque<_>>();

        while let Some((item, semi)) = queue.pop_front() {
            match item {
                ast::Item::Use(item_use) => {
                    let visibility = ast_to_visibility(&item_use.visibility)?;

                    let import = Import {
                        kind: ImportKind::Global,
                        visibility,
                        module: self.mod_item.clone(),
                        item: self.items.item().clone(),
                        source: self.source.clone(),
                        source_id: self.source_id,
                        ast: item_use,
                    };

                    let queue = &mut self.queue;

                    import.process(self.context, &self.storage, &self.query, &mut |task| {
                        queue.push_back(task);
                    })?;
                }
                ast::Item::MacroCall(mut macro_call) => {
                    let mut attributes = attrs::Attributes::new(
                        macro_call.attributes.to_vec(),
                        self.storage.clone(),
                        self.source.clone(),
                    );

                    if self.try_expand_internal_macro(&mut attributes, &mut macro_call)? {
                        items.push((ast::Item::MacroCall(macro_call), semi));
                    } else {
                        let file = self.expand_macro::<ast::File>(&mut macro_call)?;

                        for entry in file.items.into_iter().rev() {
                            queue.push_front(entry);
                        }
                    }

                    if let Some(span) = attributes.remaining() {
                        return Err(CompileError::msg(span, "unsupported item attribute"));
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
        stmts.sort_by_key(|s| s.sort_key());

        let mut queue = stmts.drain(..).collect::<VecDeque<_>>();

        while let Some(stmt) = queue.pop_front() {
            match stmt {
                ast::Stmt::Item(ast::Item::Use(item_use), _) => {
                    let visibility = ast_to_visibility(&item_use.visibility)?;

                    let import = Import {
                        kind: ImportKind::Global,
                        visibility,
                        module: self.mod_item.clone(),
                        item: self.items.item().clone(),
                        source: self.source.clone(),
                        source_id: self.source_id,
                        ast: item_use,
                    };

                    let queue = &mut self.queue;

                    import.process(self.context, &self.storage, &self.query, &mut |task| {
                        queue.push_back(task);
                    })?;
                }
                ast::Stmt::Item(ast::Item::MacroCall(mut macro_call), semi) => {
                    let mut attributes = attrs::Attributes::new(
                        macro_call.attributes.to_vec(),
                        self.storage.clone(),
                        self.source.clone(),
                    );

                    if self.try_expand_internal_macro(&mut attributes, &mut macro_call)? {
                        // Expand into an expression so that it gets compiled.
                        stmts.push(ast::Stmt::Expr(ast::Expr::MacroCall(macro_call), semi));
                    } else if let Some(out) =
                        self.expand_macro::<Option<ast::ItemOrExpr>>(&mut macro_call)?
                    {
                        let stmt = match out {
                            ast::ItemOrExpr::Item(item) => ast::Stmt::Item(item, semi),
                            ast::ItemOrExpr::Expr(expr) => {
                                ast::Stmt::Expr(macro_call.adjust_expr_semi(expr), semi)
                            }
                        };

                        queue.push_front(stmt);
                    }

                    if let Some(span) = attributes.remaining() {
                        return Err(CompileError::msg(span, "unsupported statement attribute"));
                    }
                }
                ast::Stmt::Expr(expr, semi) => {
                    stmts.push(ast::Stmt::Expr(expr, semi));
                }
                ast::Stmt::Local(expr) => {
                    stmts.push(ast::Stmt::Local(expr));
                }
                ast::Stmt::Item(mut item, semi) => {
                    item.index(self)?;
                    stmts.push(ast::Stmt::Item(item, semi));
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

        let visibility = ast_to_visibility(&item_mod.visibility)?;

        let mod_item = self.query.insert_mod(
            &self.items,
            self.source_id,
            item_mod.name_span(),
            &self.mod_item,
            visibility,
        )?;

        item_mod.id = Some(self.items.id());

        let source = self.source_loader.load(root, &mod_item.item, span)?;

        if let Some(existing) = self
            .loaded
            .insert(mod_item.item.clone(), (self.source_id, span))
        {
            return Err(CompileError::new(
                span,
                CompileErrorKind::ModAlreadyLoaded {
                    item: mod_item.item.clone(),
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

pub(crate) trait Index {
    /// Walk the current type with the given item.
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()>;
}

impl Index for ast::File {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        if let Some(first) = self.attributes.first() {
            return Err(CompileError::msg(
                first,
                "file attributes are not supported",
            ));
        }

        idx.preprocess_items(&mut self.items)?;

        for (item, semi_colon) in &mut self.items {
            if let Some(semi_colon) = semi_colon {
                if !item.needs_semi_colon() {
                    idx.diagnostics
                        .uneccessary_semi_colon(idx.source_id, semi_colon.span());
                }
            }

            item.index(idx)?;
        }

        Ok(())
    }
}

impl Index for ast::ItemFn {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ItemFn => {:?}", idx.source.source(span));

        let name = self.name.resolve(&idx.storage, &*idx.source)?;
        let _guard = idx.items.push_name(name.as_ref());

        let visibility = ast_to_visibility(&self.visibility)?;
        let item = idx.query.insert_new_item(
            &idx.items,
            idx.source_id,
            span,
            &idx.mod_item,
            visibility,
        )?;

        let kind = match (self.const_token, self.async_token) {
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

        if let (Some(const_token), Some(async_token)) = (self.const_token, self.async_token) {
            return Err(CompileError::new(
                const_token.span().join(async_token.span()),
                CompileErrorKind::FnConstAsyncConflict,
            ));
        }

        let guard = idx.scopes.push_function(kind);

        for (arg, _) in &mut self.args {
            match arg {
                ast::FnArg::SelfValue(s) => {
                    let span = s.span();
                    idx.scopes.declare("self", span)?;
                }
                ast::FnArg::Pat(pat) => {
                    pat.index_local(idx)?;
                }
            }
        }

        // Take and restore item nesting.
        let last = idx.nested_item.replace(self.descriptive_span());
        self.body.index(idx)?;
        idx.nested_item = last;

        let f = guard.into_function(span)?;
        self.id = Some(item.id);

        let call = match Indexer::call(f.generator, f.kind) {
            Some(call) => call,
            // const function.
            None => {
                if f.generator {
                    return Err(CompileError::new(
                        span,
                        CompileErrorKind::FnConstNotGenerator,
                    ));
                }

                idx.query
                    .index_const_fn(&item, &idx.source, Box::new(self.clone()))?;

                return Ok(());
            }
        };

        let fun = Function {
            ast: Box::new(self.clone()),
            call,
        };

        // NB: it's only a public item in the sense of exporting it if it's not
        // inside of a nested item.
        let is_public = item.is_public() && idx.nested_item.is_none();

        let mut attributes = attrs::Attributes::new(
            self.attributes.clone(),
            idx.storage.clone(),
            idx.source.clone(),
        );

        let is_test = match attributes.try_parse::<attrs::Test>()? {
            Some((span, _)) => {
                if let Some(nested_span) = idx.nested_item {
                    let span = span.join(self.descriptive_span());

                    return Err(CompileError::new(
                        span,
                        CompileErrorKind::NestedTest { nested_span },
                    ));
                }

                true
            }
            _ => false,
        };

        if let Some(attrs) = attributes.remaining() {
            return Err(CompileError::msg(attrs, "unrecognized function attribute"));
        }

        if self.is_instance() {
            if is_test {
                return Err(CompileError::msg(
                    span,
                    "#[test] is not supported on member functions",
                ));
            }

            let impl_item = idx.impl_item.as_ref().ok_or_else(|| {
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
            idx.query.push_build_entry(BuildEntry {
                location: Location::new(idx.source_id, f.ast.span()),
                item: item.clone(),
                build: Build::InstanceFunction(f),
                source: idx.source.clone(),
                used: Used::Used,
            });

            let kind = CompileMetaKind::Function {
                type_hash: Hash::type_hash(&item.item),
                is_test: false,
            };

            let meta = CompileMeta {
                item,
                kind,
                source: Some(CompileSource {
                    span,
                    path: idx.source.path().map(ToOwned::to_owned),
                    source_id: idx.source_id,
                }),
            };

            idx.query.insert_meta(span, meta)?;
        } else if is_public || is_test {
            // NB: immediately compile all toplevel functions.
            idx.query.push_build_entry(BuildEntry {
                location: Location::new(idx.source_id, fun.ast.descriptive_span()),
                item: item.clone(),
                build: Build::Function(fun),
                source: idx.source.clone(),
                used: Used::Used,
            });

            let kind = CompileMetaKind::Function {
                type_hash: Hash::type_hash(&item.item),
                is_test,
            };

            let meta = CompileMeta {
                item,
                kind,
                source: Some(CompileSource {
                    span,
                    path: idx.source.path().map(ToOwned::to_owned),
                    source_id: idx.source_id,
                }),
            };

            idx.query.insert_meta(span, meta)?;
        } else {
            idx.query.index(IndexedEntry {
                item,
                source: idx.source.clone(),
                indexed: Indexed::Function(fun),
            });
        }

        Ok(())
    }
}

impl Index for ast::ExprBlock {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprBlock => {:?}", idx.source.source(span));

        if let Some(span) = self.attributes.option_span() {
            return Err(CompileError::msg(
                span,
                "block attributes are not supported yet",
            ));
        }

        if self.async_token.is_none() && self.const_token.is_none() {
            if let Some(span) = self.move_token.option_span() {
                return Err(CompileError::msg(
                    span,
                    "move modifier not support on blocks",
                ));
            }

            return self.block.index(idx);
        }

        let _guard = idx.items.push_id();

        let item = idx.query.insert_new_item(
            &idx.items,
            idx.source_id,
            span,
            &idx.mod_item,
            Visibility::default(),
        )?;

        self.block.id = Some(item.id);

        if self.const_token.is_some() {
            if let Some(async_token) = self.async_token {
                return Err(CompileError::new(
                    async_token.span(),
                    CompileErrorKind::BlockConstAsyncConflict,
                ));
            }

            self.block.index(idx)?;
            idx.query.index_const(&item, &idx.source, self)?;
            return Ok(());
        }

        let guard = idx
            .scopes
            .push_closure(IndexFnKind::Async, self.move_token.is_some());

        self.block.index(idx)?;

        let c = guard.into_closure(span)?;

        let captures = Arc::from(c.captures);

        let call = match Indexer::call(c.generator, c.kind) {
            Some(call) => call,
            None => {
                return Err(CompileError::new(span, CompileErrorKind::ClosureKind));
            }
        };

        idx.query.index_async_block(
            &item,
            &idx.source,
            self.block.clone(),
            captures,
            call,
            c.do_move,
        )?;

        Ok(())
    }
}

impl Index for ast::Block {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("Block => {:?}", idx.source.source(span));

        let _guard = idx.items.push_id();
        let _guard = idx.scopes.push_scope();

        idx.query.insert_new_item(
            &idx.items,
            idx.source_id,
            span,
            &idx.mod_item,
            Visibility::Inherited,
        )?;

        idx.preprocess_stmts(&mut self.statements)?;
        let mut must_be_last = None;

        for stmt in &mut self.statements {
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
                    local.index(idx)?;
                }
                ast::Stmt::Expr(expr, None) => {
                    if expr.needs_semi() {
                        must_be_last = Some(expr.span());
                    }

                    expr.index(idx)?;
                }
                ast::Stmt::Expr(expr, Some(semi)) => {
                    if !expr.needs_semi() {
                        idx.diagnostics
                            .uneccessary_semi_colon(idx.source_id, semi.span());
                    }

                    expr.index(idx)?;
                }
                ast::Stmt::Item(item, semi) => {
                    if let Some(semi) = semi {
                        if !item.needs_semi_colon() {
                            idx.diagnostics
                                .uneccessary_semi_colon(idx.source_id, semi.span());
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl Index for ast::Local {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("Local => {:?}", idx.source.source(span));

        if let Some(span) = self.attributes.option_span() {
            return Err(CompileError::msg(span, "attributes are not supported"));
        }

        self.pat.index(idx)?;
        self.expr.index(idx)?;
        Ok(())
    }
}

impl Index for ast::ExprLet {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprLet => {:?}", idx.source.source(span));

        self.pat.index(idx)?;
        self.expr.index(idx)?;
        Ok(())
    }
}

impl Index for ast::Ident {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("Ident => {:?}", idx.source.source(span));

        let ident = self.resolve(&idx.storage, &*idx.source)?;
        idx.scopes.declare(ident.as_ref(), span)?;
        Ok(())
    }
}

impl Index for ast::Pat {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("Pat => {:?}", idx.source.source(span));

        match self {
            ast::Pat::PatPath(pat_path) => {
                pat_path.path.index(idx)?;

                if let Some(ident) = pat_path.path.try_as_ident_mut() {
                    ident.index(idx)?;
                }
            }
            ast::Pat::PatObject(pat_object) => {
                pat_object.index(idx)?;
            }
            ast::Pat::PatVec(pat_vec) => {
                pat_vec.index(idx)?;
            }
            ast::Pat::PatTuple(pat_tuple) => {
                pat_tuple.index(idx)?;
            }
            ast::Pat::PatBinding(pat_binding) => {
                pat_binding.index(idx)?;
            }
            ast::Pat::PatIgnore(..) => (),
            ast::Pat::PatLit(..) => (),
            ast::Pat::PatRest(..) => (),
        }

        Ok(())
    }
}

impl Index for ast::PatTuple {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("PatTuple => {:?}", idx.source.source(span));

        if let Some(path) = &mut self.path {
            path.index(idx)?;
        }

        for (pat, _) in &mut self.items {
            pat.index(idx)?;
        }

        Ok(())
    }
}

impl Index for ast::PatBinding {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("PatBinding => {:?}", idx.source.source(span));
        self.pat.index(idx)?;
        Ok(())
    }
}

impl Index for ast::PatObject {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("PatObject => {:?}", idx.source.source(span));

        match &mut self.ident {
            ast::ObjectIdent::Anonymous(..) => (),
            ast::ObjectIdent::Named(path) => {
                path.index(idx)?;
            }
        }

        for (pat, _) in &mut self.items {
            pat.index(idx)?;
        }

        Ok(())
    }
}

impl Index for ast::PatVec {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("PatVec => {:?}", idx.source.source(span));

        for (pat, _) in &mut self.items {
            pat.index(idx)?;
        }

        Ok(())
    }
}

impl Index for ast::Expr {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("Expr => {:?}", idx.source.source(span));

        let mut attributes = attrs::Attributes::new(
            self.attributes().to_vec(),
            idx.storage.clone(),
            idx.source.clone(),
        );

        match self {
            ast::Expr::Path(path) => {
                path.index(idx)?;
            }
            ast::Expr::Let(expr_let) => {
                expr_let.index(idx)?;
            }
            ast::Expr::Block(block) => {
                block.index(idx)?;
            }
            ast::Expr::Group(expr) => {
                expr.expr.index(idx)?;
            }
            ast::Expr::If(expr_if) => {
                expr_if.index(idx)?;
            }
            ast::Expr::Assign(expr_assign) => {
                expr_assign.index(idx)?;
            }
            ast::Expr::Binary(expr_binary) => {
                expr_binary.index(idx)?;
            }
            ast::Expr::Match(expr_if) => {
                expr_if.index(idx)?;
            }
            ast::Expr::Item(decl) => {
                decl.index(idx)?;
            }
            ast::Expr::Closure(expr_closure) => {
                expr_closure.index(idx)?;
            }
            ast::Expr::While(expr_while) => {
                expr_while.index(idx)?;
            }
            ast::Expr::Loop(expr_loop) => {
                expr_loop.index(idx)?;
            }
            ast::Expr::For(expr_for) => {
                expr_for.index(idx)?;
            }
            ast::Expr::FieldAccess(expr_field_access) => {
                expr_field_access.index(idx)?;
            }
            ast::Expr::Unary(expr_unary) => {
                expr_unary.index(idx)?;
            }
            ast::Expr::Index(expr_index_get) => {
                expr_index_get.index(idx)?;
            }
            ast::Expr::Break(expr_break) => {
                expr_break.index(idx)?;
            }
            ast::Expr::Continue(expr_continue) => {
                expr_continue.index(idx)?;
            }
            ast::Expr::Yield(expr_yield) => {
                expr_yield.index(idx)?;
            }
            ast::Expr::Return(expr_return) => {
                expr_return.index(idx)?;
            }
            ast::Expr::Await(expr_await) => {
                expr_await.index(idx)?;
            }
            ast::Expr::Try(expr_try) => {
                expr_try.index(idx)?;
            }
            ast::Expr::Select(expr_select) => {
                expr_select.index(idx)?;
            }
            // ignored because they have no effect on indexing.
            ast::Expr::Call(expr_call) => {
                expr_call.index(idx)?;
            }
            ast::Expr::Lit(expr_lit) => {
                expr_lit.index(idx)?;
            }
            ast::Expr::ForceSemi(force_semi) => {
                force_semi.expr.index(idx)?;
            }
            ast::Expr::Tuple(expr_tuple) => {
                expr_tuple.index(idx)?;
            }
            ast::Expr::Vec(expr_vec) => {
                expr_vec.index(idx)?;
            }
            ast::Expr::Object(expr_object) => {
                expr_object.index(idx)?;
            }
            ast::Expr::Range(expr_range) => {
                expr_range.index(idx)?;
            }
            // NB: macros have nothing to index, they don't export language
            // items.
            ast::Expr::MacroCall(macro_call) => {
                // Note: There is a preprocessing step involved with statemetns
                // for which the macro **might** have been expanded to a
                // built-in macro if we end up here. So instead of expanding if
                // the id is set, we just assert that the builtin macro has been
                // added to the query engine.

                if macro_call.id.is_none() {
                    if !idx.try_expand_internal_macro(&mut attributes, macro_call)? {
                        let out = idx.expand_macro::<ast::Expr>(macro_call)?;
                        *self = out;
                        self.index(idx)?;
                    }
                } else {
                    // Assert that the built-in macro has been expanded.
                    idx.query.builtin_macro_for(&**macro_call)?;
                    attributes.drain();
                }
            }
        }

        if let Some(span) = attributes.remaining() {
            return Err(CompileError::msg(span, "unsupported expression attribute"));
        }

        Ok(())
    }
}

impl Index for ast::ExprIf {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprIf => {:?}", idx.source.source(span));

        self.condition.index(idx)?;
        self.block.index(idx)?;

        for expr_else_if in &mut self.expr_else_ifs {
            expr_else_if.condition.index(idx)?;
            expr_else_if.block.index(idx)?;
        }

        if let Some(expr_else) = &mut self.expr_else {
            expr_else.block.index(idx)?;
        }

        Ok(())
    }
}

impl Index for ast::ExprAssign {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprAssign => {:?}", idx.source.source(span));

        self.lhs.index(idx)?;
        self.rhs.index(idx)?;
        Ok(())
    }
}

impl Index for ast::ExprBinary {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprBinary => {:?}", idx.source.source(span));

        self.lhs.index(idx)?;
        self.rhs.index(idx)?;
        Ok(())
    }
}

impl Index for ast::ExprMatch {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprMatch => {:?}", idx.source.source(span));

        self.expr.index(idx)?;

        for (branch, _) in &mut self.branches {
            if let Some((_, condition)) = &mut branch.condition {
                condition.index(idx)?;
            }

            let _guard = idx.scopes.push_scope();
            branch.pat.index(idx)?;
            branch.body.index(idx)?;
        }

        Ok(())
    }
}

impl Index for ast::Condition {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("Condition => {:?}", idx.source.source(span));

        match self {
            ast::Condition::Expr(expr) => {
                expr.index(idx)?;
            }
            ast::Condition::ExprLet(expr_let) => {
                expr_let.index(idx)?;
            }
        }

        Ok(())
    }
}

impl Index for ast::ItemEnum {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();

        if let Some(first) = self.attributes.first() {
            return Err(CompileError::msg(
                first,
                "enum attributes are not supported",
            ));
        }

        let name = self.name.resolve(&idx.storage, &*idx.source)?;
        let _guard = idx.items.push_name(name.as_ref());

        let visibility = ast_to_visibility(&self.visibility)?;
        let enum_item = idx.query.insert_new_item(
            &idx.items,
            idx.source_id,
            span,
            &idx.mod_item,
            visibility,
        )?;

        idx.query.index_enum(&enum_item, &idx.source)?;

        for (variant, _) in &mut self.variants {
            if let Some(first) = variant.attributes.first() {
                return Err(CompileError::msg(
                    first,
                    "variant attributes are not supported yet",
                ));
            }

            for (field, _) in variant.body.fields() {
                if let Some(first) = field.attributes.first() {
                    return Err(CompileError::msg(
                        first,
                        "field attributes are not supported",
                    ));
                }
            }

            let span = variant.name.span();
            let name = variant.name.resolve(&idx.storage, &*idx.source)?;
            let _guard = idx.items.push_name(name.as_ref());

            let item = idx.query.insert_new_item(
                &idx.items,
                idx.source_id,
                span,
                &idx.mod_item,
                Visibility::Public,
            )?;
            variant.id = Some(item.id);

            idx.query
                .index_variant(&item, &idx.source, enum_item.id, variant.clone())?;
        }

        Ok(())
    }
}

impl Index for Box<ast::ItemStruct> {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();

        if let Some(first) = self.attributes.first() {
            return Err(CompileError::msg(
                first,
                "struct attributes are not supported",
            ));
        }

        for (field, _) in self.body.fields() {
            if let Some(first) = field.attributes.first() {
                return Err(CompileError::msg(
                    first,
                    "field attributes are not supported",
                ));
            } else if !field.visibility.is_inherited() {
                return Err(CompileError::msg(
                    &field,
                    "field visibility levels are not supported",
                ));
            }
        }

        let ident = self.ident.resolve(&idx.storage, &*idx.source)?;
        let _guard = idx.items.push_name(ident.as_ref());

        let visibility = ast_to_visibility(&self.visibility)?;
        let item = idx.query.insert_new_item(
            &idx.items,
            idx.source_id,
            span,
            &idx.mod_item,
            visibility,
        )?;
        self.id = Some(item.id);

        idx.query.index_struct(&item, &idx.source, self.clone())?;
        Ok(())
    }
}

impl Index for ast::ItemImpl {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        if let Some(first) = self.attributes.first() {
            return Err(CompileError::msg(
                first,
                "impl attributes are not supported",
            ));
        }

        let mut guards = Vec::new();

        if let Some(global) = &self.path.global {
            return Err(CompileError::msg(
                global,
                "global scopes are not supported yet",
            ));
        }

        for path_segment in self.path.as_components() {
            let ident_segment = path_segment
                .try_as_ident()
                .ok_or_else(|| CompileError::msg(path_segment, "unsupported path segment"))?;
            let ident = ident_segment.resolve(&idx.storage, &*idx.source)?;
            guards.push(idx.items.push_name(ident.as_ref()));
        }

        let new = Arc::new(idx.items.item().clone());
        let old = std::mem::replace(&mut idx.impl_item, Some(new));

        for item_fn in &mut self.functions {
            item_fn.index(idx)?;
        }

        idx.impl_item = old;
        Ok(())
    }
}

impl Index for ast::ItemMod {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        if let Some(first) = self.attributes.first() {
            return Err(CompileError::msg(
                first,
                "module attributes are not supported",
            ));
        }

        let name_span = self.name_span();

        match &mut self.body {
            ast::ItemModBody::EmptyBody(..) => {
                idx.handle_file_mod(self)?;
            }
            ast::ItemModBody::InlineBody(body) => {
                let name = self.name.resolve(&idx.storage, &*idx.source)?;
                let _guard = idx.items.push_name(name.as_ref());

                let visibility = ast_to_visibility(&self.visibility)?;
                let mod_item = idx.query.insert_mod(
                    &idx.items,
                    idx.source_id,
                    name_span,
                    &idx.mod_item,
                    visibility,
                )?;

                self.id = Some(idx.items.id());

                let replaced = std::mem::replace(&mut idx.mod_item, mod_item);
                body.file.index(idx)?;
                idx.mod_item = replaced;
            }
        }

        Ok(())
    }
}

impl Index for Box<ast::ItemConst> {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        if let Some(first) = self.attributes.first() {
            return Err(CompileError::msg(
                first,
                "attributes on constants are not supported",
            ));
        }

        let span = self.span();
        let name = self.name.resolve(&idx.storage, &*idx.source)?;
        let _guard = idx.items.push_name(name.as_ref());

        let item = idx.query.insert_new_item(
            &idx.items,
            idx.source_id,
            span,
            &idx.mod_item,
            ast_to_visibility(&self.visibility)?,
        )?;

        self.id = Some(item.id);

        let last = idx.nested_item.replace(self.descriptive_span());
        self.expr.index(idx)?;
        idx.nested_item = last;

        idx.query.index_const(&item, &idx.source, &self.expr)?;
        Ok(())
    }
}

impl Index for ast::Item {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("Item => {:?}", idx.source.source(span));

        let mut attributes = attrs::Attributes::new(
            self.attributes().to_vec(),
            idx.storage.clone(),
            idx.source.clone(),
        );

        match self {
            ast::Item::Enum(item_enum) => {
                item_enum.index(idx)?;
            }
            ast::Item::Struct(item_struct) => {
                item_struct.index(idx)?;
            }
            ast::Item::Fn(item_fn) => {
                item_fn.index(idx)?;
                attributes.drain();
            }
            ast::Item::Impl(item_impl) => {
                item_impl.index(idx)?;
            }
            ast::Item::Mod(item_mod) => {
                item_mod.index(idx)?;
            }
            ast::Item::Const(item_const) => {
                item_const.index(idx)?;
            }
            ast::Item::MacroCall(macro_call) => {
                // Note: There is a preprocessing step involved with items for
                // which the macro must have been expanded to a built-in macro
                // if we end up here. So instead of expanding here, we just
                // assert that the builtin macro has been added to the query
                // engine.

                // Assert that the built-in macro has been expanded.
                idx.query.builtin_macro_for(&**macro_call)?;

                // NB: macros are handled during pre-processing.
                attributes.drain();
            }
            // NB: imports are ignored during indexing.
            ast::Item::Use(..) => {}
        }

        if let Some(span) = attributes.remaining() {
            return Err(CompileError::msg(span, "unsupported item attribute"));
        }

        Ok(())
    }
}

impl Index for ast::Path {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("Path => {:?}", idx.source.source(span));

        let id = idx
            .query
            .insert_path(&idx.mod_item, idx.impl_item.as_ref(), &*idx.items.item());
        self.id = Some(id);

        match self.as_kind() {
            Some(ast::PathKind::SelfValue) => {
                idx.scopes.mark_use("self");
            }
            Some(ast::PathKind::Ident(ident)) => {
                let ident = ident.resolve(&idx.storage, &*idx.source)?;
                idx.scopes.mark_use(ident.as_ref());
            }
            None => (),
        }

        Ok(())
    }
}

impl Index for ast::ExprWhile {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprWhile => {:?}", idx.source.source(span));

        let _guard = idx.scopes.push_scope();
        self.condition.index(idx)?;
        self.body.index(idx)?;
        Ok(())
    }
}

impl Index for ast::ExprLoop {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprLoop => {:?}", idx.source.source(span));

        let _guard = idx.scopes.push_scope();
        self.body.index(idx)?;
        Ok(())
    }
}

impl Index for ast::ExprFor {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprFor => {:?}", idx.source.source(span));

        // NB: creating the iterator is evaluated in the parent scope.
        self.iter.index(idx)?;

        let _guard = idx.scopes.push_scope();
        self.binding.index(idx)?;
        self.body.index(idx)?;
        Ok(())
    }
}

impl Index for Box<ast::ExprClosure> {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprClosure => {:?}", idx.source.source(span));

        let _guard = idx.items.push_id();

        let kind = match self.async_token {
            Some(..) => IndexFnKind::Async,
            _ => IndexFnKind::None,
        };

        let guard = idx.scopes.push_closure(kind, self.move_token.is_some());
        let span = self.span();

        let item = idx.query.insert_new_item(
            &idx.items,
            idx.source_id,
            span,
            &idx.mod_item,
            Visibility::Inherited,
        )?;

        self.id = Some(idx.items.id());

        for (arg, _) in self.args.as_slice_mut() {
            match arg {
                ast::FnArg::SelfValue(s) => {
                    return Err(CompileError::new(s, CompileErrorKind::UnsupportedSelf));
                }
                ast::FnArg::Pat(pat) => {
                    pat.index_local(idx)?;
                }
            }
        }

        self.body.index(idx)?;

        let c = guard.into_closure(span)?;

        let captures = Arc::from(c.captures);

        let call = match Indexer::call(c.generator, c.kind) {
            Some(call) => call,
            None => {
                return Err(CompileError::new(span, CompileErrorKind::ClosureKind));
            }
        };

        idx.query
            .index_closure(&item, &idx.source, self.clone(), captures, call, c.do_move)?;

        Ok(())
    }
}

impl Index for ast::ExprFieldAccess {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprIndexSet => {:?}", idx.source.source(span));

        self.expr.index(idx)?;
        Ok(())
    }
}

impl Index for ast::ExprUnary {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprUnary => {:?}", idx.source.source(span));

        self.expr.index(idx)?;
        Ok(())
    }
}

impl Index for ast::ExprIndex {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprIndex => {:?}", idx.source.source(span));

        self.index.index(idx)?;
        self.target.index(idx)?;
        Ok(())
    }
}

impl Index for ast::ExprBreak {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprBreak => {:?}", idx.source.source(span));

        if let Some(expr) = &mut self.expr {
            match expr {
                ast::ExprBreakValue::Expr(expr) => {
                    expr.index(idx)?;
                }
                ast::ExprBreakValue::Label(..) => (),
            }
        }

        Ok(())
    }
}

impl Index for ast::ExprContinue {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprContinue => {:?}", idx.source.source(span));
        Ok(())
    }
}

impl Index for ast::ExprYield {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprYield => {:?}", idx.source.source(span));

        let span = self.span();
        idx.scopes.mark_yield(span)?;

        if let Some(expr) = &mut self.expr {
            expr.index(idx)?;
        }

        Ok(())
    }
}

impl Index for ast::ExprReturn {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprReturn => {:?}", idx.source.source(span));

        if let Some(expr) = &mut self.expr {
            expr.index(idx)?;
        }

        Ok(())
    }
}

impl Index for ast::ExprAwait {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprAwait => {:?}", idx.source.source(span));

        let span = self.span();
        idx.scopes.mark_await(span)?;
        self.expr.index(idx)?;
        Ok(())
    }
}

impl Index for ast::ExprTry {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprTry => {:?}", idx.source.source(span));

        self.expr.index(idx)?;
        Ok(())
    }
}

impl Index for ast::ExprSelect {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprSelect => {:?}", idx.source.source(span));

        idx.scopes.mark_await(self.span())?;

        let mut default_branch = None;

        for (branch, _) in &mut self.branches {
            match branch {
                ast::ExprSelectBranch::Pat(pat) => {
                    // NB: expression to evaluate future is evaled in parent scope.
                    pat.expr.index(idx)?;

                    let _guard = idx.scopes.push_scope();
                    pat.pat.index(idx)?;
                    pat.body.index(idx)?;
                }
                ast::ExprSelectBranch::Default(def) => {
                    default_branch = Some(def);
                }
            }
        }

        if let Some(def) = default_branch {
            let _guard = idx.scopes.push_scope();
            def.body.index(idx)?;
        }

        Ok(())
    }
}

impl Index for ast::ExprCall {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprCall => {:?}", idx.source.source(span));

        self.id = Some(idx.items.id());

        for (expr, _) in &mut self.args {
            expr.index(idx)?;
        }

        self.expr.index(idx)?;
        Ok(())
    }
}

impl Index for ast::ExprLit {
    fn index(&mut self, _: &mut Indexer<'_>) -> CompileResult<()> {
        if let Some(first) = self.attributes.first() {
            return Err(CompileError::msg(
                first,
                "literal attributes are not supported",
            ));
        }

        match &mut self.lit {
            // NB: literals have nothing to index, they don't export language
            // items.
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

impl Index for ast::ExprTuple {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprTuple => {:?}", idx.source.source(span));

        for (expr, _) in &mut self.items {
            expr.index(idx)?;
        }

        Ok(())
    }
}

impl Index for ast::ExprVec {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprVec => {:?}", idx.source.source(span));

        for (expr, _) in &mut self.items {
            expr.index(idx)?;
        }

        Ok(())
    }
}

impl Index for ast::ExprObject {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprObject => {:?}", idx.source.source(span));

        match &mut self.ident {
            ast::ObjectIdent::Named(path) => {
                path.index(idx)?;
            }
            ast::ObjectIdent::Anonymous(..) => (),
        }

        for (assign, _) in &mut self.assignments {
            if let Some((_, expr)) = &mut assign.assign {
                expr.index(idx)?;
            }
        }

        Ok(())
    }
}

impl Index for ast::ExprRange {
    fn index(&mut self, idx: &mut Indexer<'_>) -> CompileResult<()> {
        let span = self.span();
        log::trace!("ExprRange => {:?}", idx.source.source(span));

        if let Some(from) = &mut self.from {
            from.index(idx)?;
        }

        if let Some(to) = &mut self.to {
            to.index(idx)?;
        }

        Ok(())
    }
}

/// Construct visibility from ast.
pub(crate) fn ast_to_visibility(vis: &ast::Visibility) -> Result<Visibility, CompileError> {
    let span = match vis {
        ast::Visibility::Inherited => return Ok(Visibility::Inherited),
        ast::Visibility::Public(..) => return Ok(Visibility::Public),
        ast::Visibility::Crate(..) => return Ok(Visibility::Crate),
        ast::Visibility::Super(..) => return Ok(Visibility::Super),
        ast::Visibility::SelfValue(..) => return Ok(Visibility::SelfValue),
        ast::Visibility::In(restrict) => restrict.span(),
    };

    Err(CompileError::new(
        span,
        CompileErrorKind::UnsupportedVisibility,
    ))
}
