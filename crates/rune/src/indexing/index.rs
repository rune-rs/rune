use crate::ast;
use crate::ast::{OptionSpanned, Span, Spanned};
use crate::collections::HashMap;
use crate::compile::attrs;
use crate::compile::ir;
use crate::compile::{
    CompileError, CompileErrorKind, CompileResult, Item, Location, ModMeta, Options, PrivMeta,
    PrivMetaKind, SourceLoader, SourceMeta, Visibility,
};
use crate::indexing::locals;
use crate::indexing::{IndexFnKind, IndexScopes};
use crate::macros::MacroCompiler;
use crate::parse::{Parse, ParseError, ParseErrorKind, Parser, Resolve};
use crate::query::{
    Build, BuildEntry, BuiltInFile, BuiltInFormat, BuiltInLine, BuiltInMacro, BuiltInTemplate,
    Function, Indexed, IndexedEntry, InstanceFunction, Query, Used,
};
use crate::runtime::format;
use crate::runtime::Call;
use crate::shared::Items;
use crate::worker::{Import, ImportKind, LoadFileKind, Task};
use crate::{Context, Diagnostics, Hash, SourceId};
use rune_macros::__instrument_ast as instrument;
use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;

/// `self` variable.
const SELF: &str = "self";

/// Indicates whether the thing being indexed should be marked as used to
/// determine whether they capture a variable from an outside scope (like a
/// closure) or not.
#[derive(Debug, Clone, Copy)]
struct IsUsed(bool);

const IS_USED: IsUsed = IsUsed(true);
const NOT_USED: IsUsed = IsUsed(true);

pub(crate) struct Indexer<'a> {
    /// The root URL that the indexed file originated from.
    pub(crate) root: Option<PathBuf>,
    /// Loaded modules.
    pub(crate) loaded: &'a mut HashMap<Item, (SourceId, Span)>,
    /// Query engine.
    pub(crate) q: Query<'a>,
    /// Imports to process.
    pub(crate) queue: &'a mut VecDeque<Task>,
    /// Native context.
    pub(crate) context: &'a Context,
    pub(crate) options: &'a Options,
    pub(crate) source_id: SourceId,
    pub(crate) diagnostics: &'a mut Diagnostics,
    pub(crate) items: Items<'a>,
    pub(crate) scopes: IndexScopes,
    /// The current module being indexed.
    pub(crate) mod_item: Arc<ModMeta>,
    /// Set if we are inside of an impl self.
    pub(crate) impl_item: Option<Arc<Item>>,
    /// Source loader to use.
    pub(crate) source_loader: &'a mut dyn SourceLoader,
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
        let (_, builtin) = match attributes.try_parse::<attrs::BuiltIn>(resolve_context!(self.q))? {
            Some(builtin) => builtin,
            None => return Ok(false),
        };

        let args = builtin.args(resolve_context!(self.q))?;

        // NB: internal macros are
        let ident = match ast.path.try_as_ident() {
            Some(ident) => ident,
            None => {
                return Err(CompileError::new(
                    ast.path.span(),
                    CompileErrorKind::NoSuchBuiltInMacro {
                        name: ast.path.resolve(resolve_context!(self.q))?,
                    },
                ))
            }
        };

        let ident = ident.resolve(resolve_context!(self.q))?;

        let mut internal_macro = match ident {
            "template" => self.expand_template_macro(ast, &args)?,
            "format" => self.expand_format_macro(ast, &args)?,
            "file" => self.expand_file_macro(ast)?,
            "line" => self.expand_line_macro(ast)?,
            _ => {
                return Err(CompileError::new(
                    ast.path.span(),
                    CompileErrorKind::NoSuchBuiltInMacro {
                        name: ast.path.resolve(resolve_context!(self.q))?,
                    },
                ))
            }
        };

        match &mut internal_macro {
            BuiltInMacro::Template(template) => {
                for e in &mut template.exprs {
                    expr(e, self, IS_USED)?;
                }
            }
            BuiltInMacro::Format(format) => {
                expr(&mut format.value, self, IS_USED)?;
            }

            BuiltInMacro::Line(_) | BuiltInMacro::File(_) => { /* Nothing to index */ }
        }

        let id = self.q.insert_new_builtin_macro(internal_macro)?;
        ast.id.set(id);
        Ok(true)
    }

    /// Expand the template macro.
    fn expand_template_macro(
        &mut self,
        ast: &mut ast::MacroCall,
        args: &attrs::BuiltInArgs,
    ) -> Result<BuiltInMacro, ParseError> {
        let mut p = Parser::from_token_stream(&ast.stream, ast.span());
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
        let mut p = Parser::from_token_stream(&ast.stream, ast.span());

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

            let k = key.resolve(resolve_context!(self.q))?;

            match k {
                "fill" => {
                    if fill.is_some() {
                        return Err(ParseError::unsupported(
                            key.span(),
                            "multiple `format!(.., fill = ..)`",
                        ));
                    }

                    let arg = p.parse::<ast::LitChar>()?;
                    let f = arg.resolve(resolve_context!(self.q))?;

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
                    let a = arg.resolve(resolve_context!(self.q))?;

                    align = Some(match str::parse::<format::Alignment>(a) {
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
                        .resolve(resolve_context!(self.q))?
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
                        .resolve(resolve_context!(self.q))?
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
                        .resolve(resolve_context!(self.q))?
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
                    let a = arg.resolve(resolve_context!(self.q))?;

                    format_type = Some(match str::parse::<format::Type>(a) {
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
        let name = self.q.sources.name(self.source_id).ok_or_else(|| {
            ParseError::new(
                ast.span(),
                ParseErrorKind::MissingSourceId {
                    source_id: self.source_id,
                },
            )
        })?;
        let id = self.q.storage.insert_str(name);
        let source = ast::StrSource::Synthetic(id);
        let node = ast::LitStr {
            span: ast.span(),
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
            .q
            .sources
            .get(self.source_id)
            .and_then(|s| s.position_to_utf16cu_line_char(ast.open.span.start.into_usize()))
            .unwrap_or((0, 0));

        let id = self.q.storage.insert_number(l + 1); // 1-indexed as that is what most editors will use
        let source = ast::NumberSource::Synthetic(id);

        Ok(BuiltInMacro::Line(BuiltInLine {
            span: ast.span(),
            value: ast::LitNumber {
                span: ast.span(),
                source,
            },
        }))
    }

    /// Perform a macro expansion.
    fn expand_macro<T>(&mut self, ast: &mut ast::MacroCall) -> Result<T, CompileError>
    where
        T: Parse,
    {
        let id = self
            .q
            .insert_path(&self.mod_item, self.impl_item.as_ref(), &*self.items.item());
        ast.path.id.set(id);

        let item = self.q.get_item(ast.span(), self.items.id())?;

        let mut compiler = MacroCompiler {
            item,
            options: self.options,
            context: self.context,
            query: self.q.borrow(),
        };

        let expanded = compiler.eval_macro::<T>(ast)?;
        self.q.remove_path_by_id(ast.path.id);
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
                        source_id: self.source_id,
                        ast: Box::new(item_use),
                    };

                    let queue = &mut self.queue;

                    import.process(self.context, &mut self.q, &mut |task| {
                        queue.push_back(task);
                    })?;
                }
                ast::Item::MacroCall(mut macro_call) => {
                    let mut attributes = attrs::Attributes::new(macro_call.attributes.to_vec());

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
                        source_id: self.source_id,
                        ast: Box::new(item_use),
                    };

                    let queue = &mut self.queue;

                    import.process(self.context, &mut self.q, &mut |task| {
                        queue.push_back(task);
                    })?;
                }
                ast::Stmt::Item(ast::Item::MacroCall(mut macro_call), semi) => {
                    let mut attributes = attrs::Attributes::new(macro_call.attributes.to_vec());

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
                ast::Stmt::Item(mut i, semi) => {
                    item(&mut i, self)?;
                    stmts.push(ast::Stmt::Item(i, semi));
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
        let name = item_mod.name.resolve(resolve_context!(self.q))?;
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

        let mod_item = self.q.insert_mod(
            &self.items,
            self.source_id,
            item_mod.name_span(),
            &self.mod_item,
            visibility,
        )?;

        item_mod.id.set(self.items.id());

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

        let source_id = self.q.sources.insert(source);
        self.q.visitor.visit_mod(source_id, span);

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

pub(crate) fn file(ast: &mut ast::File, idx: &mut Indexer<'_>) -> CompileResult<()> {
    if let Some(first) = ast.attributes.first() {
        return Err(CompileError::msg(
            first,
            "file attributes are not supported",
        ));
    }

    idx.preprocess_items(&mut ast.items)?;

    for (i, semi_colon) in &mut ast.items {
        if let Some(semi_colon) = semi_colon {
            if !i.needs_semi_colon() {
                idx.diagnostics
                    .uneccessary_semi_colon(idx.source_id, semi_colon.span());
            }
        }

        item(i, idx)?;
    }

    Ok(())
}

#[instrument]
fn item_fn(ast: &mut ast::ItemFn, idx: &mut Indexer<'_>) -> CompileResult<()> {
    let span = ast.span();

    let name = ast.name.resolve(resolve_context!(idx.q))?;
    let _guard = idx.items.push_name(name.as_ref());

    let visibility = ast_to_visibility(&ast.visibility)?;
    let item = idx
        .q
        .insert_new_item(&idx.items, idx.source_id, span, &idx.mod_item, visibility)?;

    let kind = match (ast.const_token, ast.async_token) {
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

    if let (Some(const_token), Some(async_token)) = (ast.const_token, ast.async_token) {
        return Err(CompileError::new(
            const_token.span().join(async_token.span()),
            CompileErrorKind::FnConstAsyncConflict,
        ));
    }

    let guard = idx.scopes.push_function(kind);

    for (arg, _) in &mut ast.args {
        match arg {
            ast::FnArg::SelfValue(s) => {
                let span = s.span();
                idx.scopes.declare(SELF, span)?;
            }
            ast::FnArg::Pat(p) => {
                locals::pat(p, idx)?;
            }
        }
    }

    // Take and restore item nesting.
    let last = idx.nested_item.replace(ast.descriptive_span());
    block(&mut ast.body, idx)?;
    idx.nested_item = last;

    let f = guard.into_function(span)?;
    ast.id = item.id;

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

            idx.q.index_const_fn(&item, Box::new(ast.clone()))?;

            return Ok(());
        }
    };

    let fun = Function {
        ast: Box::new(ast.clone()),
        call,
    };

    // NB: it's only a public item in the sense of exporting it if it's not
    // inside of a nested item.
    let is_public = item.is_public() && idx.nested_item.is_none();

    let mut attributes = attrs::Attributes::new(ast.attributes.clone());

    let is_test = match attributes.try_parse::<attrs::Test>(resolve_context!(idx.q))? {
        Some((span, _)) => {
            if let Some(nested_span) = idx.nested_item {
                let span = span.join(ast.descriptive_span());

                return Err(CompileError::new(
                    span,
                    CompileErrorKind::NestedTest { nested_span },
                ));
            }

            true
        }
        _ => false,
    };

    let is_bench = match attributes.try_parse::<attrs::Bench>(resolve_context!(idx.q))? {
        Some((span, _)) => {
            if let Some(nested_span) = idx.nested_item {
                let span = span.join(ast.descriptive_span());

                return Err(CompileError::new(
                    span,
                    CompileErrorKind::NestedBench { nested_span },
                ));
            }

            true
        }
        _ => false,
    };

    if let Some(attrs) = attributes.remaining() {
        return Err(CompileError::msg(attrs, "unrecognized function attribute"));
    }

    if ast.is_instance() {
        if is_test {
            return Err(CompileError::msg(
                span,
                "#[test] is not supported on member functions",
            ));
        }

        if is_bench {
            return Err(CompileError::msg(
                span,
                "#[bench] is not supported on member functions",
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
        idx.q.push_build_entry(BuildEntry {
            location: Location::new(idx.source_id, f.ast.span()),
            item: item.clone(),
            build: Build::InstanceFunction(f),
            used: Used::Used,
        });

        let kind = PrivMetaKind::Function {
            type_hash: Hash::type_hash(&item.item),
            is_test: false,
            is_bench: false,
        };

        let meta = PrivMeta {
            item,
            kind,
            source: Some(SourceMeta {
                location: Location::new(idx.source_id, span),
                path: idx.q.sources.path(idx.source_id).map(Into::into),
            }),
        };

        idx.q.insert_meta(span, meta)?;
    } else if is_public || is_test || is_bench {
        // NB: immediately compile all toplevel functions.
        idx.q.push_build_entry(BuildEntry {
            location: Location::new(idx.source_id, fun.ast.descriptive_span()),
            item: item.clone(),
            build: Build::Function(fun),
            used: Used::Used,
        });

        let kind = PrivMetaKind::Function {
            type_hash: Hash::type_hash(&item.item),
            is_test,
            is_bench,
        };

        let meta = PrivMeta {
            item,
            kind,
            source: Some(SourceMeta {
                location: Location::new(idx.source_id, span),
                path: idx.q.sources.path(idx.source_id).map(Into::into),
            }),
        };

        idx.q.insert_meta(span, meta)?;
    } else {
        idx.q.index(IndexedEntry {
            item,
            indexed: Indexed::Function(fun),
        });
    }

    Ok(())
}

#[instrument]
fn expr_block(ast: &mut ast::ExprBlock, idx: &mut Indexer<'_>) -> CompileResult<()> {
    let span = ast.span();

    if let Some(span) = ast.attributes.option_span() {
        return Err(CompileError::msg(
            span,
            "block attributes are not supported yet",
        ));
    }

    if ast.async_token.is_none() && ast.const_token.is_none() {
        if let Some(span) = ast.move_token.option_span() {
            return Err(CompileError::msg(
                span,
                "move modifier not support on blocks",
            ));
        }

        return block(&mut ast.block, idx);
    }

    let _guard = idx.items.push_id();

    let item = idx.q.insert_new_item(
        &idx.items,
        idx.source_id,
        span,
        &idx.mod_item,
        Visibility::default(),
    )?;

    ast.block.id = item.id;

    if ast.const_token.is_some() {
        if let Some(async_token) = ast.async_token {
            return Err(CompileError::new(
                async_token.span(),
                CompileErrorKind::BlockConstAsyncConflict,
            ));
        }

        block(&mut ast.block, idx)?;
        idx.q.index_const(&item, ast, ir::compile::expr_block)?;
        return Ok(());
    }

    let guard = idx
        .scopes
        .push_closure(IndexFnKind::Async, ast.move_token.is_some());

    block(&mut ast.block, idx)?;

    let c = guard.into_closure(span)?;

    let captures = Arc::from(c.captures);

    let call = match Indexer::call(c.generator, c.kind) {
        Some(call) => call,
        None => {
            return Err(CompileError::new(span, CompileErrorKind::ClosureKind));
        }
    };

    idx.q
        .index_async_block(&item, ast.block.clone(), captures, call, c.do_move)?;

    Ok(())
}

#[instrument]
fn block(ast: &mut ast::Block, idx: &mut Indexer<'_>) -> CompileResult<()> {
    let span = ast.span();

    let _guard = idx.items.push_id();
    let _guard = idx.scopes.push_scope();

    idx.q.insert_new_item(
        &idx.items,
        idx.source_id,
        span,
        &idx.mod_item,
        Visibility::Inherited,
    )?;

    idx.preprocess_stmts(&mut ast.statements)?;
    let mut must_be_last = None;

    for stmt in &mut ast.statements {
        if let Some(span) = must_be_last {
            return Err(CompileError::new(
                span,
                CompileErrorKind::ExpectedBlockSemiColon {
                    followed_span: stmt.span(),
                },
            ));
        }

        match stmt {
            ast::Stmt::Local(l) => {
                local(l, idx)?;
            }
            ast::Stmt::Expr(e, None) => {
                if e.needs_semi() {
                    must_be_last = Some(e.span());
                }

                expr(e, idx, IS_USED)?;
            }
            ast::Stmt::Expr(e, Some(semi)) => {
                if !e.needs_semi() {
                    idx.diagnostics
                        .uneccessary_semi_colon(idx.source_id, semi.span());
                }

                expr(e, idx, IS_USED)?;
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

#[instrument]
fn local(ast: &mut ast::Local, idx: &mut Indexer<'_>) -> CompileResult<()> {
    if let Some(span) = ast.attributes.option_span() {
        return Err(CompileError::msg(span, "attributes are not supported"));
    }

    // We index the rhs expression first so that it doesn't see it's own
    // declaration and use that instead of capturing from the outside.
    expr(&mut ast.expr, idx, IS_USED)?;
    pat(&mut ast.pat, idx, NOT_USED)?;
    Ok(())
}

#[instrument]
fn expr_let(ast: &mut ast::ExprLet, idx: &mut Indexer<'_>) -> CompileResult<()> {
    pat(&mut ast.pat, idx, NOT_USED)?;
    expr(&mut ast.expr, idx, IS_USED)?;
    Ok(())
}

#[instrument]
fn declare(ast: &mut ast::Ident, idx: &mut Indexer<'_>) -> CompileResult<()> {
    let span = ast.span();

    let ident = ast.resolve(resolve_context!(idx.q))?;
    idx.scopes.declare(ident, span)?;
    Ok(())
}

#[instrument]
fn pat(ast: &mut ast::Pat, idx: &mut Indexer<'_>, is_used: IsUsed) -> CompileResult<()> {
    match ast {
        ast::Pat::PatPath(pat) => {
            path(&mut pat.path, idx, is_used)?;

            if let Some(i) = pat.path.try_as_ident_mut() {
                // Treat as a variable declaration going lexically forward.
                declare(i, idx)?;
            }
        }
        ast::Pat::PatObject(pat) => {
            pat_object(pat, idx)?;
        }
        ast::Pat::PatVec(pat) => {
            pat_vec(pat, idx)?;
        }
        ast::Pat::PatTuple(pat) => {
            pat_tuple(pat, idx)?;
        }
        ast::Pat::PatBinding(pat) => {
            pat_binding(pat, idx)?;
        }
        ast::Pat::PatIgnore(..) => (),
        ast::Pat::PatLit(..) => (),
        ast::Pat::PatRest(..) => (),
    }

    Ok(())
}

#[instrument]
fn pat_tuple(ast: &mut ast::PatTuple, idx: &mut Indexer<'_>) -> CompileResult<()> {
    if let Some(p) = &mut ast.path {
        // Not a variable use - just the name of the tuple.
        path(p, idx, NOT_USED)?;
    }

    for (p, _) in &mut ast.items {
        pat(p, idx, NOT_USED)?;
    }

    Ok(())
}

#[instrument]
fn pat_binding(ast: &mut ast::PatBinding, idx: &mut Indexer<'_>) -> CompileResult<()> {
    pat(&mut ast.pat, idx, NOT_USED)?;
    Ok(())
}

#[instrument]
fn pat_object(ast: &mut ast::PatObject, idx: &mut Indexer<'_>) -> CompileResult<()> {
    match &mut ast.ident {
        ast::ObjectIdent::Anonymous(..) => (),
        ast::ObjectIdent::Named(p) => {
            // Not a variable use - just a name in a pattern match.
            path(p, idx, NOT_USED)?;
        }
    }

    for (p, _) in &mut ast.items {
        pat(p, idx, NOT_USED)?;
    }

    Ok(())
}

#[instrument]
fn pat_vec(ast: &mut ast::PatVec, idx: &mut Indexer<'_>) -> CompileResult<()> {
    for (p, _) in &mut ast.items {
        pat(p, idx, NOT_USED)?;
    }

    Ok(())
}

#[instrument]
fn expr(ast: &mut ast::Expr, idx: &mut Indexer<'_>, is_used: IsUsed) -> CompileResult<()> {
    let mut attributes = attrs::Attributes::new(ast.attributes().to_vec());

    match ast {
        ast::Expr::Path(e) => {
            path(e, idx, is_used)?;
        }
        ast::Expr::Let(e) => {
            expr_let(e, idx)?;
        }
        ast::Expr::Block(e) => {
            expr_block(e, idx)?;
        }
        ast::Expr::Group(e) => {
            expr(&mut e.expr, idx, is_used)?;
        }
        ast::Expr::Empty(e) => {
            expr(&mut e.expr, idx, is_used)?;
        }
        ast::Expr::If(e) => {
            expr_if(e, idx)?;
        }
        ast::Expr::Assign(e) => {
            expr_assign(e, idx)?;
        }
        ast::Expr::Binary(e) => {
            expr_binary(e, idx)?;
        }
        ast::Expr::Match(e) => {
            expr_match(e, idx)?;
        }
        ast::Expr::Closure(e) => {
            expr_closure(e, idx)?;
        }
        ast::Expr::While(e) => {
            expr_while(e, idx)?;
        }
        ast::Expr::Loop(e) => {
            expr_loop(e, idx)?;
        }
        ast::Expr::For(e) => {
            expr_for(e, idx)?;
        }
        ast::Expr::FieldAccess(e) => {
            expr_field_access(e, idx)?;
        }
        ast::Expr::Unary(e) => {
            expr_unary(e, idx)?;
        }
        ast::Expr::Index(e) => {
            expr_index(e, idx)?;
        }
        ast::Expr::Break(e) => {
            expr_break(e, idx)?;
        }
        ast::Expr::Continue(e) => {
            expr_continue(e, idx)?;
        }
        ast::Expr::Yield(e) => {
            expr_yield(e, idx)?;
        }
        ast::Expr::Return(e) => {
            expr_return(e, idx)?;
        }
        ast::Expr::Await(e) => {
            expr_await(e, idx)?;
        }
        ast::Expr::Try(e) => {
            expr_try(e, idx)?;
        }
        ast::Expr::Select(e) => {
            expr_select(e, idx)?;
        }
        // ignored because they have no effect on indexing.
        ast::Expr::Call(e) => {
            expr_call(e, idx)?;
        }
        ast::Expr::Lit(e) => {
            expr_lit(e, idx)?;
        }
        ast::Expr::ForceSemi(e) => {
            expr(&mut e.expr, idx, is_used)?;
        }
        ast::Expr::Tuple(e) => {
            expr_tuple(e, idx)?;
        }
        ast::Expr::Vec(e) => {
            expr_vec(e, idx)?;
        }
        ast::Expr::Object(e) => {
            expr_object(e, idx)?;
        }
        ast::Expr::Range(e) => {
            expr_range(e, idx)?;
        }
        // NB: macros have nothing to index, they don't export language
        // items.
        ast::Expr::MacroCall(macro_call) => {
            // Note: There is a preprocessing step involved with statemetns
            // for which the macro **might** have been expanded to a
            // built-in macro if we end up here. So instead of expanding if
            // the id is set, we just assert that the builtin macro has been
            // added to the query engine.

            if !macro_call.id.is_set() {
                if !idx.try_expand_internal_macro(&mut attributes, macro_call)? {
                    let out = idx.expand_macro::<ast::Expr>(macro_call)?;
                    *ast = out;
                    expr(ast, idx, is_used)?;
                }
            } else {
                // Assert that the built-in macro has been expanded.
                idx.q.builtin_macro_for(&*macro_call)?;
                attributes.drain();
            }
        }
    }

    if let Some(span) = attributes.remaining() {
        return Err(CompileError::msg(span, "unsupported expression attribute"));
    }

    Ok(())
}

#[instrument]
fn expr_if(ast: &mut ast::ExprIf, idx: &mut Indexer<'_>) -> CompileResult<()> {
    condition(&mut ast.condition, idx)?;
    block(&mut ast.block, idx)?;

    for expr_else_if in &mut ast.expr_else_ifs {
        condition(&mut expr_else_if.condition, idx)?;
        block(&mut expr_else_if.block, idx)?;
    }

    if let Some(expr_else) = &mut ast.expr_else {
        block(&mut expr_else.block, idx)?;
    }

    Ok(())
}

#[instrument]
fn expr_assign(ast: &mut ast::ExprAssign, idx: &mut Indexer<'_>) -> CompileResult<()> {
    expr(&mut ast.lhs, idx, IS_USED)?;
    expr(&mut ast.rhs, idx, IS_USED)?;
    Ok(())
}

#[instrument]
fn expr_binary(ast: &mut ast::ExprBinary, idx: &mut Indexer<'_>) -> CompileResult<()> {
    expr(&mut ast.lhs, idx, IS_USED)?;
    expr(&mut ast.rhs, idx, IS_USED)?;
    Ok(())
}

#[instrument]
fn expr_match(ast: &mut ast::ExprMatch, idx: &mut Indexer<'_>) -> CompileResult<()> {
    expr(&mut ast.expr, idx, IS_USED)?;

    for (branch, _) in &mut ast.branches {
        if let Some((_, condition)) = &mut branch.condition {
            expr(condition, idx, IS_USED)?;
        }

        let _guard = idx.scopes.push_scope();
        pat(&mut branch.pat, idx, NOT_USED)?;
        expr(&mut branch.body, idx, IS_USED)?;
    }

    Ok(())
}

#[instrument]
fn condition(ast: &mut ast::Condition, idx: &mut Indexer<'_>) -> CompileResult<()> {
    match ast {
        ast::Condition::Expr(e) => {
            expr(e, idx, IS_USED)?;
        }
        ast::Condition::ExprLet(e) => {
            expr_let(e, idx)?;
        }
    }

    Ok(())
}

#[instrument]
fn item_enum(ast: &mut ast::ItemEnum, idx: &mut Indexer<'_>) -> CompileResult<()> {
    let span = ast.span();

    if let Some(first) = ast.attributes.first() {
        return Err(CompileError::msg(
            first,
            "enum attributes are not supported",
        ));
    }

    let name = ast.name.resolve(resolve_context!(idx.q))?;
    let _guard = idx.items.push_name(name.as_ref());

    let visibility = ast_to_visibility(&ast.visibility)?;
    let enum_item =
        idx.q
            .insert_new_item(&idx.items, idx.source_id, span, &idx.mod_item, visibility)?;

    idx.q.index_enum(&enum_item)?;

    for (index, (variant, _)) in ast.variants.iter_mut().enumerate() {
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
        let name = variant.name.resolve(resolve_context!(idx.q))?;
        let _guard = idx.items.push_name(name.as_ref());

        let item = idx.q.insert_new_item(
            &idx.items,
            idx.source_id,
            span,
            &idx.mod_item,
            Visibility::Public,
        )?;
        variant.id = item.id;

        idx.q
            .index_variant(&item, enum_item.id, variant.clone(), index)?;
    }

    Ok(())
}

#[instrument]
fn item_struct(ast: &mut ast::ItemStruct, idx: &mut Indexer<'_>) -> CompileResult<()> {
    let span = ast.span();

    if let Some(first) = ast.attributes.first() {
        return Err(CompileError::msg(
            first,
            "struct attributes are not supported",
        ));
    }

    for (field, _) in ast.body.fields() {
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

    let ident = ast.ident.resolve(resolve_context!(idx.q))?;
    let _guard = idx.items.push_name(ident);

    let visibility = ast_to_visibility(&ast.visibility)?;
    let item = idx
        .q
        .insert_new_item(&idx.items, idx.source_id, span, &idx.mod_item, visibility)?;
    ast.id = item.id;

    idx.q.index_struct(&item, Box::new(ast.clone()))?;
    Ok(())
}

#[instrument]
fn item_impl(ast: &mut ast::ItemImpl, idx: &mut Indexer<'_>) -> CompileResult<()> {
    if let Some(first) = ast.attributes.first() {
        return Err(CompileError::msg(
            first,
            "impl attributes are not supported",
        ));
    }

    let mut guards = Vec::new();

    if let Some(global) = &ast.path.global {
        return Err(CompileError::msg(
            global,
            "global scopes are not supported yet",
        ));
    }

    for path_segment in ast.path.as_components() {
        let ident_segment = path_segment
            .try_as_ident()
            .ok_or_else(|| CompileError::msg(path_segment, "unsupported path segment"))?;
        let ident = ident_segment.resolve(resolve_context!(idx.q))?;
        guards.push(idx.items.push_name(ident));
    }

    let new = Arc::new(idx.items.item().clone());
    let old = std::mem::replace(&mut idx.impl_item, Some(new));

    for i in &mut ast.functions {
        item_fn(i, idx)?;
    }

    idx.impl_item = old;
    Ok(())
}

#[instrument]
fn item_mod(ast: &mut ast::ItemMod, idx: &mut Indexer<'_>) -> CompileResult<()> {
    if let Some(first) = ast.attributes.first() {
        return Err(CompileError::msg(
            first,
            "module attributes are not supported",
        ));
    }

    let name_span = ast.name_span();

    match &mut ast.body {
        ast::ItemModBody::EmptyBody(..) => {
            idx.handle_file_mod(ast)?;
        }
        ast::ItemModBody::InlineBody(body) => {
            let name = ast.name.resolve(resolve_context!(idx.q))?;
            let _guard = idx.items.push_name(name.as_ref());

            let visibility = ast_to_visibility(&ast.visibility)?;
            let mod_item = idx.q.insert_mod(
                &idx.items,
                idx.source_id,
                name_span,
                &idx.mod_item,
                visibility,
            )?;

            ast.id.set(idx.items.id());

            let replaced = std::mem::replace(&mut idx.mod_item, mod_item);
            file(&mut body.file, idx)?;
            idx.mod_item = replaced;
        }
    }

    Ok(())
}

#[instrument]
fn item_const(ast: &mut ast::ItemConst, idx: &mut Indexer<'_>) -> CompileResult<()> {
    if let Some(first) = ast.attributes.first() {
        return Err(CompileError::msg(
            first,
            "attributes on constants are not supported",
        ));
    }

    let span = ast.span();
    let name = ast.name.resolve(resolve_context!(idx.q))?;
    let _guard = idx.items.push_name(name.as_ref());

    let item = idx.q.insert_new_item(
        &idx.items,
        idx.source_id,
        span,
        &idx.mod_item,
        ast_to_visibility(&ast.visibility)?,
    )?;

    ast.id = item.id;

    let last = idx.nested_item.replace(ast.descriptive_span());
    expr(&mut ast.expr, idx, IS_USED)?;
    idx.nested_item = last;

    idx.q.index_const(&item, &ast.expr, ir::compile::expr)?;
    Ok(())
}

#[instrument]
fn item(ast: &mut ast::Item, idx: &mut Indexer<'_>) -> CompileResult<()> {
    let mut attributes = attrs::Attributes::new(ast.attributes().to_vec());

    match ast {
        ast::Item::Enum(item) => {
            item_enum(item, idx)?;
        }
        ast::Item::Struct(item) => {
            item_struct(item, idx)?;
        }
        ast::Item::Fn(item) => {
            item_fn(item, idx)?;
            attributes.drain();
        }
        ast::Item::Impl(item) => {
            item_impl(item, idx)?;
        }
        ast::Item::Mod(item) => {
            item_mod(item, idx)?;
        }
        ast::Item::Const(item) => {
            item_const(item, idx)?;
        }
        ast::Item::MacroCall(macro_call) => {
            // Note: There is a preprocessing step involved with items for
            // which the macro must have been expanded to a built-in macro
            // if we end up here. So instead of expanding here, we just
            // assert that the builtin macro has been added to the query
            // engine.

            // Assert that the built-in macro has been expanded.
            idx.q.builtin_macro_for(&*macro_call)?;

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

#[instrument]
fn path(ast: &mut ast::Path, idx: &mut Indexer<'_>, is_used: IsUsed) -> CompileResult<()> {
    let id = idx
        .q
        .insert_path(&idx.mod_item, idx.impl_item.as_ref(), &*idx.items.item());
    ast.id.set(id);

    path_segment(&mut ast.first, idx)?;

    for (_, segment) in &mut ast.rest {
        path_segment(segment, idx)?;
    }

    if is_used.0 {
        match ast.as_kind() {
            Some(ast::PathKind::SelfValue) => {
                idx.scopes.mark_use(SELF);
            }
            Some(ast::PathKind::Ident(ident)) => {
                let ident = ident.resolve(resolve_context!(idx.q))?;
                idx.scopes.mark_use(ident);
            }
            None => (),
        }
    }

    Ok(())
}

#[instrument]
fn path_segment(ast: &mut ast::PathSegment, idx: &mut Indexer<'_>) -> CompileResult<()> {
    if let ast::PathSegment::Generics(generics) = ast {
        for (param, _) in generics {
            // This is a special case where the expression of a generic
            // statement does not count as "used". Since they do not capture
            // the outside environment.
            expr(&mut param.expr, idx, NOT_USED)?;
        }
    }

    Ok(())
}

#[instrument]
fn expr_while(ast: &mut ast::ExprWhile, idx: &mut Indexer<'_>) -> CompileResult<()> {
    let _guard = idx.scopes.push_scope();
    condition(&mut ast.condition, idx)?;
    block(&mut ast.body, idx)?;
    Ok(())
}

#[instrument]
fn expr_loop(ast: &mut ast::ExprLoop, idx: &mut Indexer<'_>) -> CompileResult<()> {
    let _guard = idx.scopes.push_scope();
    block(&mut ast.body, idx)?;
    Ok(())
}

#[instrument]
fn expr_for(ast: &mut ast::ExprFor, idx: &mut Indexer<'_>) -> CompileResult<()> {
    // NB: creating the iterator is evaluated in the parent scope.
    expr(&mut ast.iter, idx, IS_USED)?;

    let _guard = idx.scopes.push_scope();
    pat(&mut ast.binding, idx, NOT_USED)?;
    block(&mut ast.body, idx)?;
    Ok(())
}

#[instrument]
fn expr_closure(ast: &mut ast::ExprClosure, idx: &mut Indexer<'_>) -> CompileResult<()> {
    let _guard = idx.items.push_id();

    let kind = match ast.async_token {
        Some(..) => IndexFnKind::Async,
        _ => IndexFnKind::None,
    };

    let guard = idx.scopes.push_closure(kind, ast.move_token.is_some());
    let span = ast.span();

    let item = idx.q.insert_new_item(
        &idx.items,
        idx.source_id,
        span,
        &idx.mod_item,
        Visibility::Inherited,
    )?;

    ast.id.set(idx.items.id());

    for (arg, _) in ast.args.as_slice_mut() {
        match arg {
            ast::FnArg::SelfValue(s) => {
                return Err(CompileError::new(s, CompileErrorKind::UnsupportedSelf));
            }
            ast::FnArg::Pat(p) => {
                locals::pat(p, idx)?;
            }
        }
    }

    expr(&mut ast.body, idx, IS_USED)?;

    let c = guard.into_closure(span)?;

    let captures = Arc::from(c.captures);

    let call = match Indexer::call(c.generator, c.kind) {
        Some(call) => call,
        None => {
            return Err(CompileError::new(span, CompileErrorKind::ClosureKind));
        }
    };

    idx.q
        .index_closure(&item, Box::new(ast.clone()), captures, call, c.do_move)?;

    Ok(())
}

#[instrument]
fn expr_field_access(ast: &mut ast::ExprFieldAccess, idx: &mut Indexer<'_>) -> CompileResult<()> {
    expr(&mut ast.expr, idx, IS_USED)?;

    match &mut ast.expr_field {
        ast::ExprField::Path(p) => {
            path(p, idx, IS_USED)?;
        }
        ast::ExprField::LitNumber(..) => {}
    }

    Ok(())
}

#[instrument]
fn expr_unary(ast: &mut ast::ExprUnary, idx: &mut Indexer<'_>) -> CompileResult<()> {
    expr(&mut ast.expr, idx, IS_USED)?;
    Ok(())
}

#[instrument]
fn expr_index(ast: &mut ast::ExprIndex, idx: &mut Indexer<'_>) -> CompileResult<()> {
    expr(&mut ast.index, idx, IS_USED)?;
    expr(&mut ast.target, idx, IS_USED)?;
    Ok(())
}

#[instrument]
fn expr_break(ast: &mut ast::ExprBreak, idx: &mut Indexer<'_>) -> CompileResult<()> {
    if let Some(e) = ast.expr.as_deref_mut() {
        match e {
            ast::ExprBreakValue::Expr(e) => {
                expr(e, idx, IS_USED)?;
            }
            ast::ExprBreakValue::Label(..) => (),
        }
    }

    Ok(())
}

#[instrument]
fn expr_continue(ast: &mut ast::ExprContinue, idx: &mut Indexer<'_>) -> CompileResult<()> {
    Ok(())
}

#[instrument]
fn expr_yield(ast: &mut ast::ExprYield, idx: &mut Indexer<'_>) -> CompileResult<()> {
    let span = ast.span();
    idx.scopes.mark_yield(span)?;

    if let Some(e) = &mut ast.expr {
        expr(e, idx, IS_USED)?;
    }

    Ok(())
}

#[instrument]
fn expr_return(ast: &mut ast::ExprReturn, idx: &mut Indexer<'_>) -> CompileResult<()> {
    if let Some(e) = &mut ast.expr {
        expr(e, idx, IS_USED)?;
    }

    Ok(())
}

#[instrument]
fn expr_await(ast: &mut ast::ExprAwait, idx: &mut Indexer<'_>) -> CompileResult<()> {
    let span = ast.span();
    idx.scopes.mark_await(span)?;
    expr(&mut ast.expr, idx, IS_USED)?;
    Ok(())
}

#[instrument]
fn expr_try(ast: &mut ast::ExprTry, idx: &mut Indexer<'_>) -> CompileResult<()> {
    expr(&mut ast.expr, idx, IS_USED)?;
    Ok(())
}

#[instrument]
fn expr_select(ast: &mut ast::ExprSelect, idx: &mut Indexer<'_>) -> CompileResult<()> {
    idx.scopes.mark_await(ast.span())?;

    let mut default_branch = None;

    for (branch, _) in &mut ast.branches {
        match branch {
            ast::ExprSelectBranch::Pat(p) => {
                // NB: expression to evaluate future is evaled in parent scope.
                expr(&mut p.expr, idx, IS_USED)?;

                let _guard = idx.scopes.push_scope();
                pat(&mut p.pat, idx, NOT_USED)?;
                expr(&mut p.body, idx, IS_USED)?;
            }
            ast::ExprSelectBranch::Default(def) => {
                default_branch = Some(def);
            }
        }
    }

    if let Some(def) = default_branch {
        let _guard = idx.scopes.push_scope();
        expr(&mut def.body, idx, IS_USED)?;
    }

    Ok(())
}

#[instrument]
fn expr_call(ast: &mut ast::ExprCall, idx: &mut Indexer<'_>) -> CompileResult<()> {
    ast.id.set(idx.items.id());

    for (e, _) in &mut ast.args {
        expr(e, idx, IS_USED)?;
    }

    expr(&mut ast.expr, idx, IS_USED)?;
    Ok(())
}

#[instrument]
fn expr_lit(ast: &mut ast::ExprLit, _: &mut Indexer<'_>) -> CompileResult<()> {
    if let Some(first) = ast.attributes.first() {
        return Err(CompileError::msg(
            first,
            "literal attributes are not supported",
        ));
    }

    match &mut ast.lit {
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

#[instrument]
fn expr_tuple(ast: &mut ast::ExprTuple, idx: &mut Indexer<'_>) -> CompileResult<()> {
    for (e, _) in &mut ast.items {
        expr(e, idx, IS_USED)?;
    }

    Ok(())
}

#[instrument]
fn expr_vec(ast: &mut ast::ExprVec, idx: &mut Indexer<'_>) -> CompileResult<()> {
    for (e, _) in &mut ast.items {
        expr(e, idx, IS_USED)?;
    }

    Ok(())
}

#[instrument]
fn expr_object(ast: &mut ast::ExprObject, idx: &mut Indexer<'_>) -> CompileResult<()> {
    match &mut ast.ident {
        ast::ObjectIdent::Named(p) => {
            // Not a variable use: Name of the object.
            path(p, idx, NOT_USED)?;
        }
        ast::ObjectIdent::Anonymous(..) => (),
    }

    for (assign, _) in &mut ast.assignments {
        if let Some((_, e)) = &mut assign.assign {
            expr(e, idx, IS_USED)?;
        }
    }

    Ok(())
}

#[instrument]
fn expr_range(ast: &mut ast::ExprRange, idx: &mut Indexer<'_>) -> CompileResult<()> {
    if let Some(from) = &mut ast.from {
        expr(from, idx, IS_USED)?;
    }

    if let Some(to) = &mut ast.to {
        expr(to, idx, IS_USED)?;
    }

    Ok(())
}

/// Construct visibility from ast.
fn ast_to_visibility(vis: &ast::Visibility) -> Result<Visibility, CompileError> {
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
