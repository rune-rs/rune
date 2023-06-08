use core::num::NonZeroUsize;

use core::mem::replace;

use crate::no_std::collections::{HashMap, VecDeque};
use crate::no_std::path::PathBuf;
use crate::no_std::prelude::*;
use crate::no_std::sync::Arc;

use crate::ast::{self, OptionSpanned, Span, Spanned};
use crate::compile::attrs::Attributes;
use crate::compile::{
    self, attrs, CompileErrorKind, Doc, ItemId, Location, ModId, ParseErrorKind, Visibility,
    WithSpan,
};
use crate::indexing::locals;
use crate::indexing::{self, Indexed};
use crate::indexing::{IndexFnKind, IndexScopes};
use crate::macros::MacroCompiler;
use crate::parse::{Parse, Parser, Resolve};
use crate::query::{BuiltInFile, BuiltInFormat, BuiltInLine, BuiltInMacro, BuiltInTemplate, Query};
use crate::runtime::format;
use crate::runtime::Call;
use crate::shared::Items;
use crate::worker::{Import, ImportKind, LoadFileKind, Task};
use crate::SourceId;

use rune_macros::instrument;

/// `self` variable.
const SELF: &str = "self";

/// Macros are only allowed to expand recursively into other macros 64 times.
const MAX_MACRO_RECURSION: usize = 64;

/// Indicates whether the thing being indexed should be marked as used to
/// determine whether they capture a variable from an outside scope (like a
/// closure) or not.
#[derive(Debug, Clone, Copy)]
pub(crate) struct IsUsed(bool);

pub(crate) const IS_USED: IsUsed = IsUsed(true);
pub(crate) const NOT_USED: IsUsed = IsUsed(false);

pub(crate) struct Indexer<'a> {
    /// Query engine.
    pub(crate) q: Query<'a>,
    pub(crate) source_id: SourceId,
    pub(crate) items: Items<'a>,
    pub(crate) scopes: IndexScopes,
    /// The current module being indexed.
    pub(crate) mod_item: ModId,
    /// Set if we are inside of an impl self.
    pub(crate) impl_item: Option<ItemId>,
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
    /// Depth of expression macro expansion that we're currently in.
    pub(crate) macro_depth: usize,
    /// The root URL that the indexed file originated from.
    pub(crate) root: Option<PathBuf>,
    /// Imports to process.
    pub(crate) queue: Option<&'a mut VecDeque<Task>>,
    /// Loaded modules.
    pub(crate) loaded: Option<&'a mut HashMap<ModId, (SourceId, Span)>>,
}

impl<'a> Indexer<'a> {
    /// Indicate that we've entered an expanded macro context, and ensure that
    /// we don't blow past [`MAX_MACRO_RECURSION`].
    ///
    /// This is used when entering expressions which have been expanded from a
    /// macro - cause those expression might in turn be macros themselves.
    fn enter_macro<S>(&mut self, spanned: &S) -> compile::Result<()>
    where
        S: Spanned,
    {
        self.macro_depth = self.macro_depth.wrapping_add(1);

        if self.macro_depth >= MAX_MACRO_RECURSION {
            return Err(compile::Error::new(
                spanned,
                CompileErrorKind::MaxMacroRecursion {
                    depth: self.macro_depth,
                    max: MAX_MACRO_RECURSION,
                },
            ));
        }

        Ok(())
    }

    /// Leave the last macro context.
    fn leave_macro(&mut self) {
        self.macro_depth = self.macro_depth.wrapping_sub(1);
    }

    /// Try to expand an internal macro.
    fn try_expand_internal_macro(
        &mut self,
        attributes: &mut attrs::Attributes,
        ast: &mut ast::MacroCall,
    ) -> compile::Result<bool> {
        let (_, builtin) = match attributes.try_parse::<attrs::BuiltIn>(resolve_context!(self.q))? {
            Some(builtin) => builtin,
            None => return Ok(false),
        };

        let args = builtin.args(resolve_context!(self.q))?;

        // NB: internal macros are
        let ident = match ast.path.try_as_ident() {
            Some(ident) => ident,
            None => {
                return Err(compile::Error::new(
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
                return Err(compile::Error::new(
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
                    expr(self, e, IS_USED)?;
                }
            }
            BuiltInMacro::Format(format) => {
                expr(self, &mut format.value, IS_USED)?;
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
    ) -> compile::Result<BuiltInMacro> {
        let mut p = Parser::from_token_stream(&ast.input, ast.span());
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
    ) -> compile::Result<BuiltInMacro> {
        let mut p = Parser::from_token_stream(&ast.input, ast.span());

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
                        return Err(compile::Error::unsupported(
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
                        return Err(compile::Error::unsupported(
                            key.span(),
                            "multiple `format!(.., align = ..)`",
                        ));
                    }

                    let arg = p.parse::<ast::Ident>()?;
                    let a = arg.resolve(resolve_context!(self.q))?;

                    align = Some(match str::parse::<format::Alignment>(a) {
                        Ok(a) => (arg, a),
                        _ => {
                            return Err(compile::Error::unsupported(
                                key.span(),
                                "`format!(.., align = ..)`",
                            ));
                        }
                    });
                }
                "flags" => {
                    if flags.is_some() {
                        return Err(compile::Error::unsupported(
                            key.span(),
                            "multiple `format!(.., flags = ..)`",
                        ));
                    }

                    let arg = p.parse::<ast::LitNumber>()?;

                    let f = arg
                        .resolve(resolve_context!(self.q))?
                        .as_u32(false)
                        .with_span(arg)?;

                    let f = format::Flags::from(f);
                    flags = Some((arg, f));
                }
                "width" => {
                    if width.is_some() {
                        return Err(compile::Error::unsupported(
                            key.span(),
                            "multiple `format!(.., width = ..)`",
                        ));
                    }

                    let arg = p.parse::<ast::LitNumber>()?;

                    let f = arg
                        .resolve(resolve_context!(self.q))?
                        .as_usize(false)
                        .with_span(arg)?;

                    width = Some((arg, NonZeroUsize::new(f)));
                }
                "precision" => {
                    if precision.is_some() {
                        return Err(compile::Error::unsupported(
                            key.span(),
                            "multiple `format!(.., precision = ..)`",
                        ));
                    }

                    let arg = p.parse::<ast::LitNumber>()?;

                    let f = arg
                        .resolve(resolve_context!(self.q))?
                        .as_usize(false)
                        .with_span(arg)?;

                    precision = Some((arg, NonZeroUsize::new(f)));
                }
                "type" => {
                    if format_type.is_some() {
                        return Err(compile::Error::unsupported(
                            key.span(),
                            "multiple `format!(.., type = ..)`",
                        ));
                    }

                    let arg = p.parse::<ast::Ident>()?;
                    let a = arg.resolve(resolve_context!(self.q))?;

                    format_type = Some(match str::parse::<format::Type>(a) {
                        Ok(format_type) => (arg, format_type),
                        _ => {
                            return Err(compile::Error::unsupported(
                                key.span(),
                                "`format!(.., type = ..)`",
                            ));
                        }
                    });
                }
                _ => {
                    return Err(compile::Error::unsupported(
                        key.span(),
                        "`format!(.., <key>)`",
                    ));
                }
            }
        }

        p.eof()?;

        Ok(BuiltInMacro::Format(BuiltInFormat {
            span: ast.span(),
            fill,
            align,
            width,
            precision,
            flags,
            format_type,
            value,
        }))
    }

    /// Expand a macro returning the current file
    fn expand_file_macro(&mut self, ast: &mut ast::MacroCall) -> compile::Result<BuiltInMacro> {
        let name = self.q.sources.name(self.source_id).ok_or_else(|| {
            compile::Error::new(
                ast.span(),
                ParseErrorKind::MissingSourceId {
                    source_id: self.source_id,
                },
            )
        })?;
        let id = self.q.storage.insert_str(name);
        let source = ast::StrSource::Synthetic(id);
        let value = ast::Lit::Str(ast::LitStr {
            span: ast.span(),
            source,
        });

        Ok(BuiltInMacro::File(BuiltInFile {
            span: ast.span(),
            value,
        }))
    }

    /// Expand a macro returning the current line for where the macro invocation begins
    fn expand_line_macro(&mut self, ast: &mut ast::MacroCall) -> compile::Result<BuiltInMacro> {
        let (l, _) = self
            .q
            .sources
            .get(self.source_id)
            .map(|s| s.pos_to_utf16cu_linecol(ast.open.span.start.into_usize()))
            .unwrap_or_default();

        let id = self.q.storage.insert_number(l + 1); // 1-indexed as that is what most editors will use
        let source = ast::NumberSource::Synthetic(id);

        Ok(BuiltInMacro::Line(BuiltInLine {
            span: ast.span(),
            value: ast::Lit::Number(ast::LitNumber {
                span: ast.span(),
                source,
            }),
        }))
    }

    /// Perform a macro expansion.
    fn expand_macro<T>(&mut self, ast: &mut ast::MacroCall) -> compile::Result<T>
    where
        T: Parse,
    {
        let id = self
            .q
            .insert_path(self.mod_item, self.impl_item, &self.items.item());
        ast.path.id.set(id);
        let id = self.items.id().with_span(ast.span())?;

        let item = self.q.item_for((ast.span(), id))?;

        let mut compiler = MacroCompiler {
            item_meta: item,
            query: self.q.borrow(),
        };

        let expanded = compiler.eval_macro::<T>(ast)?;
        self.q.remove_path_by_id(ast.path.id);
        Ok(expanded)
    }

    /// Perform an attribute macro expansion.
    fn expand_attribute_macro<T>(
        &mut self,
        attr: &mut ast::Attribute,
        item: &mut ast::Item,
    ) -> compile::Result<Option<T>>
    where
        T: Parse,
    {
        let id = self
            .q
            .insert_path(self.mod_item, self.impl_item, &self.items.item());
        attr.path.id.set(id);
        let id = self.items.id().with_span(attr.span())?;

        let containing = self.q.item_for((attr.span(), id))?;

        let mut compiler = MacroCompiler {
            item_meta: containing,
            query: self.q.borrow(),
        };

        let expanded = compiler.eval_attribute_macro::<T>(attr, item)?;
        self.q.remove_path_by_id(attr.path.id);
        Ok(expanded)
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
    fn handle_file_mod(
        &mut self,
        item_mod: &mut ast::ItemMod,
        docs: &[Doc],
    ) -> compile::Result<()> {
        let span = item_mod.span();

        let (Some(loaded), Some(queue)) = (self.loaded.as_mut(), self.queue.as_mut()) else {
            return Err(compile::Error::msg(span, "File modules are not supported"));
        };

        let name = item_mod.name.resolve(resolve_context!(self.q))?;
        let _guard = self.items.push_name(name.as_ref());

        let root = match &self.root {
            Some(root) => root,
            None => {
                return Err(compile::Error::new(
                    span,
                    CompileErrorKind::UnsupportedModuleSource,
                ));
            }
        };

        let visibility = ast_to_visibility(&item_mod.visibility)?;

        let mod_item_id = self.items.id().with_span(span)?;

        let mod_item = self.q.insert_mod(
            &self.items,
            Location::new(self.source_id, item_mod.name_span()),
            self.mod_item,
            visibility,
            docs,
        )?;

        item_mod.id.set(mod_item_id);

        let source = self
            .q
            .source_loader
            .load(root, self.q.pool.module_item(mod_item), span)?;

        if let Some(existing) = loaded.insert(mod_item, (self.source_id, span)) {
            return Err(compile::Error::new(
                span,
                CompileErrorKind::ModAlreadyLoaded {
                    item: self.q.pool.module_item(mod_item).to_owned(),
                    existing,
                },
            ));
        }

        let source_id = self.q.sources.insert(source);
        self.q.visitor.visit_mod(source_id, span);

        queue.push_back(Task::LoadFile {
            kind: LoadFileKind::Module {
                root: self.root.clone(),
            },
            source_id,
            mod_item,
            mod_item_id,
        });

        Ok(())
    }
}

/// Index the contents of a module known by its AST as a "file".
pub(crate) fn file(idx: &mut Indexer<'_>, ast: &mut ast::File) -> compile::Result<()> {
    let mut attrs = Attributes::new(ast.attributes.to_vec());
    let docs = attrs.try_parse_collect::<attrs::Doc>(resolve_context!(idx.q))?;

    let ctx = resolve_context!(idx.q);

    // This part catches comments interior to the module of the form `//!`.
    for (span, doc) in docs {
        idx.q.visitor.visit_doc_comment(
            Location::new(idx.source_id, span),
            idx.q.pool.module_item(idx.mod_item),
            idx.q.pool.module_item_hash(idx.mod_item),
            &doc.doc_string.resolve(ctx)?,
        );
    }

    if let Some(first) = attrs.remaining() {
        return Err(compile::Error::msg(
            first,
            "file attributes are not supported",
        ));
    }

    // Items take priority.
    let mut head = VecDeque::new();

    // Macros and items with attributes are expanded as they are encountered, but after regular items have
    // been processed.
    let mut queue = VecDeque::new();

    for (item, semi) in ast.items.drain(..) {
        match item {
            i @ ast::Item::MacroCall(_) => {
                queue.push_back((0, i, Vec::new(), semi));
            }
            i if !i.attributes().is_empty() => {
                queue.push_back((0, i, Vec::new(), semi));
            }
            i => {
                head.push_back((i, semi));
            }
        }
    }

    'uses: while !head.is_empty() || !queue.is_empty() {
        while let Some((i, semi)) = head.pop_front() {
            if let Some(semi) = semi {
                if !i.needs_semi_colon() {
                    idx.q
                        .diagnostics
                        .unnecessary_semi_colon(idx.source_id, semi.span());
                }
            }

            item(idx, i)?;
        }

        while let Some((depth, mut item, mut skipped_attributes, semi)) = queue.pop_front() {
            if depth >= MAX_MACRO_RECURSION {
                return Err(compile::Error::new(
                    item.span(),
                    CompileErrorKind::MaxMacroRecursion {
                        depth,
                        max: MAX_MACRO_RECURSION,
                    },
                ));
            }

            // Before furthor processing all attributes are either expanded, or if unknown put in
            // `skipped_attributes`, to either be reinserted for the `item` handler or to be used
            // by the macro_call expansion below.
            if let Some(mut attr) = item.remove_first_attribute() {
                match idx.expand_attribute_macro::<ast::File>(&mut attr, &mut item)? {
                    Some(file) => {
                        for (item, semi) in file.items.into_iter().rev() {
                            match item {
                                item @ ast::Item::MacroCall(_) => {
                                    queue.push_back((
                                        depth.wrapping_add(1),
                                        item,
                                        Vec::new(),
                                        semi,
                                    ));
                                }
                                item if !item.attributes().is_empty() => {
                                    queue.push_back((
                                        depth.wrapping_add(1),
                                        item,
                                        Vec::new(),
                                        semi,
                                    ));
                                }
                                item => {
                                    head.push_front((item, semi));
                                }
                            }
                        }
                    }
                    None => {
                        skipped_attributes.push(attr);
                        if !matches!(item, ast::Item::MacroCall(_)) && item.attributes().is_empty()
                        {
                            // For all we know only non macro attributes remain, which will be
                            // handled by the item handler.
                            *item.attributes_mut() = skipped_attributes;
                            head.push_front((item, semi));
                        } else {
                            // items with remaining attributes and macro calls will be dealt with by
                            // reinserting in the queue.
                            queue.push_back((depth, item, skipped_attributes, semi))
                        }
                    }
                };
                continue;
            }

            let ast::Item::MacroCall(mut macro_call) = item else {
                unreachable!("non macro items would have had attributes")
            };

            macro_call.attributes = skipped_attributes.clone();

            let mut attributes = attrs::Attributes::new(skipped_attributes);

            if idx.try_expand_internal_macro(&mut attributes, &mut macro_call)? {
                // Macro call must be added to output to make sure its instructions are assembled.
                ast.items.push((ast::Item::MacroCall(macro_call), semi));
            } else {
                let file = idx.expand_macro::<ast::File>(&mut macro_call)?;

                for (item, semi) in file.items.into_iter().rev() {
                    match item {
                        item @ ast::Item::MacroCall(_) => {
                            queue.push_back((depth.wrapping_add(1), item, Vec::new(), semi));
                        }
                        item if !item.attributes().is_empty() => {
                            queue.push_back((depth.wrapping_add(1), item, Vec::new(), semi));
                        }
                        item => {
                            head.push_front((item, semi));
                        }
                    }
                }
            }

            if let Some(span) = attributes.remaining() {
                return Err(compile::Error::msg(span, "unsupported item attribute"));
            }

            if !head.is_empty() {
                continue 'uses;
            }
        }
    }

    Ok(())
}

#[instrument(span = ast)]
fn item_fn(idx: &mut Indexer<'_>, mut ast: ast::ItemFn) -> compile::Result<()> {
    let span = ast.span();

    let name = ast.name.resolve(resolve_context!(idx.q))?;
    let _guard = idx.items.push_name(name.as_ref());

    let visibility = ast_to_visibility(&ast.visibility)?;
    let mut attributes = attrs::Attributes::new(ast.attributes.clone());
    let docs = Doc::collect_from(resolve_context!(idx.q), &mut attributes)?;

    let item_meta = idx.q.insert_new_item(
        &idx.items,
        Location::new(idx.source_id, span),
        idx.mod_item,
        visibility,
        &docs,
    )?;

    let kind = match (ast.const_token, ast.async_token) {
        (Some(const_token), Some(async_token)) => {
            return Err(compile::Error::new(
                const_token.span().join(async_token.span()),
                CompileErrorKind::FnConstAsyncConflict,
            ));
        }
        (Some(..), _) => IndexFnKind::Const,
        (_, Some(..)) => IndexFnKind::Async,
        _ => IndexFnKind::None,
    };

    if let (Some(const_token), Some(async_token)) = (ast.const_token, ast.async_token) {
        return Err(compile::Error::new(
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
                locals::pat(idx, p)?;
            }
        }
    }

    // Take and restore item nesting.
    let last = idx.nested_item.replace(ast.descriptive_span());
    block(idx, &mut ast.body)?;
    idx.nested_item = last;

    let f = guard.into_function(span)?;
    ast.id = item_meta.id;

    let call = match Indexer::call(f.generator, f.kind) {
        Some(call) => call,
        // const function.
        None => {
            if f.generator {
                return Err(compile::Error::new(
                    span,
                    CompileErrorKind::FnConstNotGenerator,
                ));
            }

            idx.q.index_const_fn(item_meta, Box::new(ast))?;
            return Ok(());
        }
    };

    // NB: it's only a public item in the sense of exporting it if it's not
    // inside of a nested item.
    let is_public = item_meta.is_public(idx.q.pool) && idx.nested_item.is_none();

    let is_test = match attributes.try_parse::<attrs::Test>(resolve_context!(idx.q))? {
        Some((span, _)) => {
            if let Some(nested_span) = idx.nested_item {
                let span = span.join(ast.descriptive_span());

                return Err(compile::Error::new(
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

                return Err(compile::Error::new(
                    span,
                    CompileErrorKind::NestedBench { nested_span },
                ));
            }

            true
        }
        _ => false,
    };

    if let Some(attrs) = attributes.remaining() {
        return Err(compile::Error::msg(
            attrs,
            "unrecognized function attribute",
        ));
    }

    if ast.is_instance() {
        if is_test {
            return Err(compile::Error::msg(
                span,
                "#[test] is not supported on member functions",
            ));
        }

        if is_bench {
            return Err(compile::Error::msg(
                span,
                "#[bench] is not supported on member functions",
            ));
        }

        let impl_item = idx.impl_item.ok_or_else(|| {
            compile::Error::new(span, CompileErrorKind::InstanceFunctionOutsideImpl)
        })?;

        idx.q.index_and_build(indexing::Entry {
            item_meta,
            indexed: Indexed::InstanceFunction(indexing::InstanceFunction {
                ast: Box::new(ast),
                call,
                impl_item,
                instance_span: span,
            }),
        });
    } else {
        let entry = indexing::Entry {
            item_meta,
            indexed: Indexed::Function(indexing::Function {
                ast: Box::new(ast),
                call,
                is_test,
                is_bench,
            }),
        };

        if is_public || is_test || is_bench {
            idx.q.index_and_build(entry);
        } else {
            idx.q.index(entry);
        }
    }

    Ok(())
}

#[instrument(span = ast)]
fn expr_block(idx: &mut Indexer<'_>, ast: &mut ast::ExprBlock) -> compile::Result<()> {
    let span = ast.span();

    if let Some(span) = ast.attributes.option_span() {
        return Err(compile::Error::msg(
            span,
            "block attributes are not supported yet",
        ));
    }

    if ast.async_token.is_none() && ast.const_token.is_none() {
        if let Some(span) = ast.move_token.option_span() {
            return Err(compile::Error::msg(
                span,
                "move modifier not support on blocks",
            ));
        }

        return block(idx, &mut ast.block);
    }

    let _guard = idx.items.push_id();

    let item_meta = idx.q.insert_new_item(
        &idx.items,
        Location::new(idx.source_id, span),
        idx.mod_item,
        Visibility::default(),
        &[],
    )?;

    ast.block.id = item_meta.id;

    if ast.const_token.is_some() {
        if let Some(async_token) = ast.async_token {
            return Err(compile::Error::new(
                async_token.span(),
                CompileErrorKind::BlockConstAsyncConflict,
            ));
        }

        block(idx, &mut ast.block)?;
        idx.q.index_const_block(item_meta, &ast.block)?;
        return Ok(());
    }

    let guard = idx
        .scopes
        .push_closure(IndexFnKind::Async, ast.move_token.is_some());

    block(idx, &mut ast.block)?;

    let c = guard.into_closure(span)?;

    let captures = Arc::from(c.captures);

    let call = match Indexer::call(c.generator, c.kind) {
        Some(call) => call,
        None => {
            return Err(compile::Error::new(span, CompileErrorKind::ClosureKind));
        }
    };

    idx.q
        .index_async_block(item_meta, ast.block.clone(), captures, call, c.do_move)?;

    Ok(())
}

#[instrument(span = ast)]
fn block(idx: &mut Indexer<'_>, ast: &mut ast::Block) -> compile::Result<()> {
    let span = ast.span();

    let _guard = idx.items.push_id();
    let _guard = idx.scopes.push_scope();

    idx.q.insert_new_item(
        &idx.items,
        Location::new(idx.source_id, span),
        idx.mod_item,
        Visibility::Inherited,
        &[],
    )?;

    let mut statements = Vec::new();

    for stmt in ast.statements.drain(..) {
        match stmt {
            ast::Stmt::Item(i, semi) => {
                if let Some(semi) = semi {
                    if !i.needs_semi_colon() {
                        idx.q
                            .diagnostics
                            .unnecessary_semi_colon(idx.source_id, semi.span());
                    }
                }

                item(idx, i)?;
            }
            stmt => {
                statements.push(stmt);
            }
        }
    }

    let mut must_be_last = None;

    for stmt in &mut statements {
        if let Some(span) = must_be_last {
            return Err(compile::Error::new(
                span,
                CompileErrorKind::ExpectedBlockSemiColon {
                    followed_span: stmt.span(),
                },
            ));
        }

        match stmt {
            ast::Stmt::Local(l) => {
                local(idx, l)?;
            }
            ast::Stmt::Expr(e) => {
                if e.needs_semi() {
                    must_be_last = Some(e.span());
                }

                expr(idx, e, IS_USED)?;
            }
            ast::Stmt::Semi(semi) => {
                if !semi.needs_semi() {
                    idx.q
                        .diagnostics
                        .unnecessary_semi_colon(idx.source_id, semi.span());
                }

                expr(idx, &mut semi.expr, IS_USED)?;
            }
            ast::Stmt::Item(i, ..) => {
                return Err(compile::Error::msg(i, "Unexpected item in this stage"));
            }
        }
    }

    ast.statements = statements;
    Ok(())
}

#[instrument(span = ast)]
fn local(idx: &mut Indexer<'_>, ast: &mut ast::Local) -> compile::Result<()> {
    if let Some(span) = ast.attributes.option_span() {
        return Err(compile::Error::msg(span, "attributes are not supported"));
    }

    // We index the rhs expression first so that it doesn't see it's own
    // declaration and use that instead of capturing from the outside.
    expr(idx, &mut ast.expr, IS_USED)?;
    pat(idx, &mut ast.pat, NOT_USED)?;
    Ok(())
}

#[instrument(span = ast)]
fn expr_let(idx: &mut Indexer<'_>, ast: &mut ast::ExprLet) -> compile::Result<()> {
    pat(idx, &mut ast.pat, NOT_USED)?;
    expr(idx, &mut ast.expr, IS_USED)?;
    Ok(())
}

#[instrument(span = ast)]
fn declare(idx: &mut Indexer<'_>, ast: &mut ast::Ident) -> compile::Result<()> {
    let span = ast.span();

    let ident = ast.resolve(resolve_context!(idx.q))?;
    idx.scopes.declare(ident, span)?;
    Ok(())
}

#[instrument(span = ast)]
fn pat(idx: &mut Indexer<'_>, ast: &mut ast::Pat, is_used: IsUsed) -> compile::Result<()> {
    match ast {
        ast::Pat::Path(pat) => {
            path(idx, &mut pat.path, is_used)?;

            if let Some(i) = pat.path.try_as_ident_mut() {
                // Treat as a variable declaration going lexically forward.
                declare(idx, i)?;
            }
        }
        ast::Pat::Object(pat) => {
            pat_object(idx, pat)?;
        }
        ast::Pat::Vec(pat) => {
            pat_vec(idx, pat)?;
        }
        ast::Pat::Tuple(pat) => {
            pat_tuple(idx, pat)?;
        }
        ast::Pat::Binding(pat) => {
            pat_binding(idx, pat)?;
        }
        ast::Pat::Ignore(..) => (),
        ast::Pat::Lit(..) => (),
        ast::Pat::Rest(..) => (),
    }

    Ok(())
}

#[instrument(span = ast)]
fn pat_tuple(idx: &mut Indexer<'_>, ast: &mut ast::PatTuple) -> compile::Result<()> {
    if let Some(p) = &mut ast.path {
        // Not a variable use - just the name of the tuple.
        path(idx, p, NOT_USED)?;
    }

    for (p, _) in &mut ast.items {
        pat(idx, p, NOT_USED)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn pat_binding(idx: &mut Indexer<'_>, ast: &mut ast::PatBinding) -> compile::Result<()> {
    pat(idx, &mut ast.pat, NOT_USED)?;
    Ok(())
}

#[instrument(span = ast)]
fn pat_object(idx: &mut Indexer<'_>, ast: &mut ast::PatObject) -> compile::Result<()> {
    match &mut ast.ident {
        ast::ObjectIdent::Anonymous(..) => (),
        ast::ObjectIdent::Named(p) => {
            // Not a variable use - just a name in a pattern match.
            path(idx, p, NOT_USED)?;
        }
    }

    for (p, _) in &mut ast.items {
        pat(idx, p, NOT_USED)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn pat_vec(idx: &mut Indexer<'_>, ast: &mut ast::PatVec) -> compile::Result<()> {
    for (p, _) in &mut ast.items {
        pat(idx, p, NOT_USED)?;
    }

    Ok(())
}

#[instrument(span = ast)]
pub(crate) fn expr(
    idx: &mut Indexer<'_>,
    ast: &mut ast::Expr,
    is_used: IsUsed,
) -> compile::Result<()> {
    let mut attributes = attrs::Attributes::new(ast.attributes().to_vec());

    match ast {
        ast::Expr::Path(e) => {
            path(idx, e, is_used)?;
        }
        ast::Expr::Let(e) => {
            expr_let(idx, e)?;
        }
        ast::Expr::Block(e) => {
            expr_block(idx, e)?;
        }
        ast::Expr::Group(e) => {
            expr(idx, &mut e.expr, is_used)?;
        }
        ast::Expr::Empty(e) => {
            expr(idx, &mut e.expr, is_used)?;
        }
        ast::Expr::If(e) => {
            expr_if(idx, e)?;
        }
        ast::Expr::Assign(e) => {
            expr_assign(idx, e)?;
        }
        ast::Expr::Binary(e) => {
            expr_binary(idx, e)?;
        }
        ast::Expr::Match(e) => {
            expr_match(idx, e)?;
        }
        ast::Expr::Closure(e) => {
            expr_closure(idx, e)?;
        }
        ast::Expr::While(e) => {
            expr_while(idx, e)?;
        }
        ast::Expr::Loop(e) => {
            expr_loop(idx, e)?;
        }
        ast::Expr::For(e) => {
            expr_for(idx, e)?;
        }
        ast::Expr::FieldAccess(e) => {
            expr_field_access(idx, e)?;
        }
        ast::Expr::Unary(e) => {
            expr_unary(idx, e)?;
        }
        ast::Expr::Index(e) => {
            expr_index(idx, e)?;
        }
        ast::Expr::Break(e) => {
            expr_break(idx, e)?;
        }
        ast::Expr::Continue(e) => {
            expr_continue(idx, e)?;
        }
        ast::Expr::Yield(e) => {
            expr_yield(idx, e)?;
        }
        ast::Expr::Return(e) => {
            expr_return(idx, e)?;
        }
        ast::Expr::Await(e) => {
            expr_await(idx, e)?;
        }
        ast::Expr::Try(e) => {
            expr_try(idx, e)?;
        }
        ast::Expr::Select(e) => {
            expr_select(idx, e)?;
        }
        // ignored because they have no effect on indexing.
        ast::Expr::Call(e) => {
            expr_call(idx, e)?;
        }
        ast::Expr::Lit(e) => {
            expr_lit(idx, e)?;
        }
        ast::Expr::Tuple(e) => {
            expr_tuple(idx, e)?;
        }
        ast::Expr::Vec(e) => {
            expr_vec(idx, e)?;
        }
        ast::Expr::Object(e) => {
            expr_object(idx, e)?;
        }
        ast::Expr::Range(e) => {
            expr_range(idx, e)?;
        }
        // NB: macros have nothing to index, they don't export language
        // items.
        ast::Expr::MacroCall(macro_call) => {
            // Note: There is a preprocessing step involved with statements for
            // which the macro **might** have been expanded to a built-in macro
            // if we end up here. So instead of expanding if the id is set, we
            // just assert that the builtin macro has been added to the query
            // engine.

            if !macro_call.id.is_set() {
                if !idx.try_expand_internal_macro(&mut attributes, macro_call)? {
                    let out = idx.expand_macro::<ast::Expr>(macro_call)?;
                    idx.enter_macro(&macro_call)?;
                    *ast = out;
                    expr(idx, ast, is_used)?;
                    idx.leave_macro();
                }
            } else {
                // Assert that the built-in macro has been expanded.
                idx.q.builtin_macro_for(&*macro_call)?;
                attributes.drain();
            }
        }
    }

    if let Some(span) = attributes.remaining() {
        return Err(compile::Error::msg(
            span,
            "unsupported expression attribute",
        ));
    }

    Ok(())
}

#[instrument(span = ast)]
fn expr_if(idx: &mut Indexer<'_>, ast: &mut ast::ExprIf) -> compile::Result<()> {
    condition(idx, &mut ast.condition)?;
    block(idx, &mut ast.block)?;

    for expr_else_if in &mut ast.expr_else_ifs {
        condition(idx, &mut expr_else_if.condition)?;
        block(idx, &mut expr_else_if.block)?;
    }

    if let Some(expr_else) = &mut ast.expr_else {
        block(idx, &mut expr_else.block)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn expr_assign(idx: &mut Indexer<'_>, ast: &mut ast::ExprAssign) -> compile::Result<()> {
    expr(idx, &mut ast.lhs, IS_USED)?;
    expr(idx, &mut ast.rhs, IS_USED)?;
    Ok(())
}

#[instrument(span = ast)]
fn expr_binary(idx: &mut Indexer<'_>, ast: &mut ast::ExprBinary) -> compile::Result<()> {
    expr(idx, &mut ast.lhs, IS_USED)?;
    expr(idx, &mut ast.rhs, IS_USED)?;
    Ok(())
}

#[instrument(span = ast)]
fn expr_match(idx: &mut Indexer<'_>, ast: &mut ast::ExprMatch) -> compile::Result<()> {
    expr(idx, &mut ast.expr, IS_USED)?;

    for (branch, _) in &mut ast.branches {
        if let Some((_, condition)) = &mut branch.condition {
            expr(idx, condition, IS_USED)?;
        }

        let _guard = idx.scopes.push_scope();
        pat(idx, &mut branch.pat, NOT_USED)?;
        expr(idx, &mut branch.body, IS_USED)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn condition(idx: &mut Indexer<'_>, ast: &mut ast::Condition) -> compile::Result<()> {
    match ast {
        ast::Condition::Expr(e) => {
            expr(idx, e, IS_USED)?;
        }
        ast::Condition::ExprLet(e) => {
            expr_let(idx, e)?;
        }
    }

    Ok(())
}

#[instrument(span = ast)]
fn item_enum(idx: &mut Indexer<'_>, ast: ast::ItemEnum) -> compile::Result<()> {
    let span = ast.span();
    let mut attrs = Attributes::new(ast.attributes.to_vec());
    let docs = Doc::collect_from(resolve_context!(idx.q), &mut attrs)?;

    if let Some(first) = attrs.remaining() {
        return Err(compile::Error::msg(
            first,
            "enum attributes are not supported",
        ));
    }

    let name = ast.name.resolve(resolve_context!(idx.q))?;
    let _guard = idx.items.push_name(name.as_ref());

    let visibility = ast_to_visibility(&ast.visibility)?;
    let enum_item = idx.q.insert_new_item(
        &idx.items,
        Location::new(idx.source_id, span),
        idx.mod_item,
        visibility,
        &docs,
    )?;

    idx.q.index_enum(enum_item)?;

    for (index, (mut variant, _)) in ast.variants.into_iter().enumerate() {
        let mut attrs = Attributes::new(variant.attributes.to_vec());
        let docs = Doc::collect_from(resolve_context!(idx.q), &mut attrs)?;

        if let Some(first) = attrs.remaining() {
            return Err(compile::Error::msg(
                first,
                "variant attributes are not supported yet",
            ));
        }

        let span = variant.name.span();
        let name = variant.name.resolve(resolve_context!(idx.q))?;
        let _guard = idx.items.push_name(name.as_ref());

        let item_meta = idx.q.insert_new_item(
            &idx.items,
            Location::new(idx.source_id, span),
            idx.mod_item,
            Visibility::Public,
            &docs,
        )?;
        variant.id = item_meta.id;

        let ctx = resolve_context!(idx.q);

        for (field, _) in variant.body.fields() {
            let mut attrs = Attributes::new(field.attributes.to_vec());
            let docs = Doc::collect_from(ctx, &mut attrs)?;
            let name = field.name.resolve(ctx)?;

            for doc in docs {
                idx.q.visitor.visit_field_doc_comment(
                    Location::new(idx.source_id, doc.span),
                    idx.q.pool.item(item_meta.item),
                    idx.q.pool.item_type_hash(item_meta.item),
                    name,
                    doc.doc_string.resolve(ctx)?.as_ref(),
                );
            }

            if let Some(first) = attrs.remaining() {
                return Err(compile::Error::msg(
                    first,
                    "field attributes are not supported",
                ));
            }
        }

        idx.q
            .index_variant(item_meta, enum_item.id, variant, index)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn item_struct(idx: &mut Indexer<'_>, mut ast: ast::ItemStruct) -> compile::Result<()> {
    let span = ast.span();
    let mut attrs = Attributes::new(ast.attributes.to_vec());

    let docs = Doc::collect_from(resolve_context!(idx.q), &mut attrs)?;

    if let Some(first) = attrs.remaining() {
        return Err(compile::Error::msg(
            first,
            "struct attributes are not supported",
        ));
    }

    let ident = ast.ident.resolve(resolve_context!(idx.q))?;
    let _guard = idx.items.push_name(ident);

    let visibility = ast_to_visibility(&ast.visibility)?;
    let item_meta = idx.q.insert_new_item(
        &idx.items,
        Location::new(idx.source_id, span),
        idx.mod_item,
        visibility,
        &docs,
    )?;
    ast.id = item_meta.id;

    let ctx = resolve_context!(idx.q);

    for (field, _) in ast.body.fields() {
        let mut attrs = Attributes::new(field.attributes.to_vec());
        let docs = Doc::collect_from(ctx, &mut attrs)?;
        let name = field.name.resolve(ctx)?;

        for doc in docs {
            idx.q.visitor.visit_field_doc_comment(
                Location::new(idx.source_id, doc.span),
                idx.q.pool.item(item_meta.item),
                idx.q.pool.item_type_hash(item_meta.item),
                name,
                doc.doc_string.resolve(ctx)?.as_ref(),
            );
        }

        if let Some(first) = attrs.remaining() {
            return Err(compile::Error::msg(
                first,
                "field attributes are not supported",
            ));
        } else if !field.visibility.is_inherited() {
            return Err(compile::Error::msg(
                field,
                "field visibility levels are not supported",
            ));
        }
    }

    idx.q.index_struct(item_meta, Box::new(ast))?;
    Ok(())
}

#[instrument(span = ast)]
fn item_impl(idx: &mut Indexer<'_>, ast: ast::ItemImpl) -> compile::Result<()> {
    if let Some(first) = ast.attributes.first() {
        return Err(compile::Error::msg(
            first,
            "impl attributes are not supported",
        ));
    }

    let mut guards = Vec::new();

    if let Some(global) = &ast.path.global {
        return Err(compile::Error::msg(
            global,
            "global scopes are not supported yet",
        ));
    }

    for path_segment in ast.path.as_components() {
        let ident_segment = path_segment
            .try_as_ident()
            .ok_or_else(|| compile::Error::msg(path_segment, "unsupported path segment"))?;
        let ident = ident_segment.resolve(resolve_context!(idx.q))?;
        guards.push(idx.items.push_name(ident));
    }

    let new = idx.q.pool.alloc_item(&*idx.items.item());
    let old = replace(&mut idx.impl_item, Some(new));

    for i in ast.functions {
        item_fn(idx, i)?;
    }

    idx.impl_item = old;
    Ok(())
}

#[instrument(span = ast)]
fn item_mod(idx: &mut Indexer<'_>, mut ast: ast::ItemMod) -> compile::Result<()> {
    let mut attrs = Attributes::new(ast.attributes.clone());
    let docs = Doc::collect_from(resolve_context!(idx.q), &mut attrs)?;

    if let Some(first) = attrs.remaining() {
        return Err(compile::Error::msg(
            first,
            "module attributes are not supported",
        ));
    }

    let name_span = ast.name_span();

    match &mut ast.body {
        ast::ItemModBody::EmptyBody(..) => {
            idx.handle_file_mod(&mut ast, &docs)?;
        }
        ast::ItemModBody::InlineBody(body) => {
            let name = ast.name.resolve(resolve_context!(idx.q))?;
            let _guard = idx.items.push_name(name.as_ref());

            let visibility = ast_to_visibility(&ast.visibility)?;
            let mod_item = idx.q.insert_mod(
                &idx.items,
                Location::new(idx.source_id, name_span),
                idx.mod_item,
                visibility,
                &docs,
            )?;

            ast.id.set(idx.items.id().with_span(name_span)?);

            let replaced = replace(&mut idx.mod_item, mod_item);
            file(idx, &mut body.file)?;
            idx.mod_item = replaced;
        }
    }

    Ok(())
}

#[instrument(span = ast)]
fn item_const(idx: &mut Indexer<'_>, mut ast: ast::ItemConst) -> compile::Result<()> {
    let mut attrs = Attributes::new(ast.attributes.to_vec());
    let docs = Doc::collect_from(resolve_context!(idx.q), &mut attrs)?;

    if let Some(first) = attrs.remaining() {
        return Err(compile::Error::msg(
            first,
            "attributes on constants are not supported",
        ));
    }

    let span = ast.span();
    let name = ast.name.resolve(resolve_context!(idx.q))?;
    let _guard = idx.items.push_name(name.as_ref());

    let item_meta = idx.q.insert_new_item(
        &idx.items,
        Location::new(idx.source_id, span),
        idx.mod_item,
        ast_to_visibility(&ast.visibility)?,
        &docs,
    )?;

    ast.id = item_meta.id;

    let last = idx.nested_item.replace(ast.descriptive_span());
    expr(idx, &mut ast.expr, IS_USED)?;
    idx.nested_item = last;
    idx.q.index_const_expr(item_meta, &ast.expr)?;
    Ok(())
}

#[instrument(span = ast)]
fn item(idx: &mut Indexer<'_>, ast: ast::Item) -> compile::Result<()> {
    let mut attributes = attrs::Attributes::new(ast.attributes().to_vec());

    match ast {
        ast::Item::Enum(item) => {
            item_enum(idx, item)?;
        }
        ast::Item::Struct(item) => {
            item_struct(idx, item)?;
        }
        ast::Item::Fn(item) => {
            item_fn(idx, item)?;
            attributes.drain();
        }
        ast::Item::Impl(item) => {
            item_impl(idx, item)?;
        }
        ast::Item::Mod(item) => {
            item_mod(idx, item)?;
        }
        ast::Item::Const(item) => {
            item_const(idx, item)?;
        }
        ast::Item::MacroCall(macro_call) => {
            // Note: There is a preprocessing step involved with items for
            // which the macro must have been expanded to a built-in macro
            // if we end up here. So instead of expanding here, we just
            // assert that the builtin macro has been added to the query
            // engine.

            // Assert that the built-in macro has been expanded.
            idx.q.builtin_macro_for(&macro_call)?;

            // NB: macros are handled during pre-processing.
            attributes.drain();
        }
        // NB: imports are ignored during indexing.
        ast::Item::Use(item_use) => {
            let Some(queue) = idx.queue.as_mut() else {
                return Err(compile::Error::msg(&item_use, "Imports are not supported in this context"));
            };

            let visibility = ast_to_visibility(&item_use.visibility)?;

            let import = Import {
                kind: ImportKind::Global,
                visibility,
                module: idx.mod_item,
                item: idx.items.item().clone(),
                source_id: idx.source_id,
                ast: Box::new(item_use),
            };

            import.process(&mut idx.q, &mut |task| {
                queue.push_back(task);
            })?;
        }
    }

    attributes.try_parse_collect::<attrs::Doc>(resolve_context!(idx.q))?;
    if let Some(span) = attributes.remaining() {
        return Err(compile::Error::msg(span, "unsupported item attribute"));
    }

    Ok(())
}

#[instrument(span = ast)]
fn path(idx: &mut Indexer<'_>, ast: &mut ast::Path, is_used: IsUsed) -> compile::Result<()> {
    let id = idx
        .q
        .insert_path(idx.mod_item, idx.impl_item, &idx.items.item());
    ast.id.set(id);

    path_segment(idx, &mut ast.first)?;

    for (_, segment) in &mut ast.rest {
        path_segment(idx, segment)?;
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

#[instrument(span = ast)]
fn path_segment(idx: &mut Indexer<'_>, ast: &mut ast::PathSegment) -> compile::Result<()> {
    if let ast::PathSegment::Generics(generics) = ast {
        for (param, _) in generics {
            // This is a special case where the expression of a generic
            // statement does not count as "used". Since they do not capture
            // the outside environment.
            expr(idx, &mut param.expr, NOT_USED)?;
        }
    }

    Ok(())
}

#[instrument(span = ast)]
fn expr_while(idx: &mut Indexer<'_>, ast: &mut ast::ExprWhile) -> compile::Result<()> {
    let _guard = idx.scopes.push_scope();
    condition(idx, &mut ast.condition)?;
    block(idx, &mut ast.body)?;
    Ok(())
}

#[instrument(span = ast)]
fn expr_loop(idx: &mut Indexer<'_>, ast: &mut ast::ExprLoop) -> compile::Result<()> {
    let _guard = idx.scopes.push_scope();
    block(idx, &mut ast.body)?;
    Ok(())
}

#[instrument(span = ast)]
fn expr_for(idx: &mut Indexer<'_>, ast: &mut ast::ExprFor) -> compile::Result<()> {
    // NB: creating the iterator is evaluated in the parent scope.
    expr(idx, &mut ast.iter, IS_USED)?;

    let _guard = idx.scopes.push_scope();
    pat(idx, &mut ast.binding, NOT_USED)?;
    block(idx, &mut ast.body)?;
    Ok(())
}

#[instrument(span = ast)]
fn expr_closure(idx: &mut Indexer<'_>, ast: &mut ast::ExprClosure) -> compile::Result<()> {
    let _guard = idx.items.push_id();

    let kind = match ast.async_token {
        Some(..) => IndexFnKind::Async,
        _ => IndexFnKind::None,
    };

    let guard = idx.scopes.push_closure(kind, ast.move_token.is_some());
    let span = ast.span();

    let item_meta = idx.q.insert_new_item(
        &idx.items,
        Location::new(idx.source_id, span),
        idx.mod_item,
        Visibility::Inherited,
        &[],
    )?;

    ast.id.set(idx.items.id().with_span(ast.span())?);

    for (arg, _) in ast.args.as_slice_mut() {
        match arg {
            ast::FnArg::SelfValue(s) => {
                return Err(compile::Error::new(s, CompileErrorKind::UnsupportedSelf));
            }
            ast::FnArg::Pat(p) => {
                locals::pat(idx, p)?;
            }
        }
    }

    expr(idx, &mut ast.body, IS_USED)?;

    let c = guard.into_closure(span)?;

    let captures = Arc::from(c.captures);

    let call = match Indexer::call(c.generator, c.kind) {
        Some(call) => call,
        None => {
            return Err(compile::Error::new(span, CompileErrorKind::ClosureKind));
        }
    };

    idx.q
        .index_closure(item_meta, Box::new(ast.clone()), captures, call, c.do_move)?;

    Ok(())
}

#[instrument(span = ast)]
fn expr_field_access(idx: &mut Indexer<'_>, ast: &mut ast::ExprFieldAccess) -> compile::Result<()> {
    expr(idx, &mut ast.expr, IS_USED)?;

    match &mut ast.expr_field {
        ast::ExprField::Path(p) => {
            path(idx, p, IS_USED)?;
        }
        ast::ExprField::LitNumber(..) => {}
    }

    Ok(())
}

#[instrument(span = ast)]
fn expr_unary(idx: &mut Indexer<'_>, ast: &mut ast::ExprUnary) -> compile::Result<()> {
    expr(idx, &mut ast.expr, IS_USED)?;
    Ok(())
}

#[instrument(span = ast)]
fn expr_index(idx: &mut Indexer<'_>, ast: &mut ast::ExprIndex) -> compile::Result<()> {
    expr(idx, &mut ast.index, IS_USED)?;
    expr(idx, &mut ast.target, IS_USED)?;
    Ok(())
}

#[instrument(span = ast)]
fn expr_break(idx: &mut Indexer<'_>, ast: &mut ast::ExprBreak) -> compile::Result<()> {
    if let Some(e) = ast.expr.as_deref_mut() {
        match e {
            ast::ExprBreakValue::Expr(e) => {
                expr(idx, e, IS_USED)?;
            }
            ast::ExprBreakValue::Label(..) => (),
        }
    }

    Ok(())
}

#[instrument(span = ast)]
fn expr_continue(idx: &mut Indexer<'_>, ast: &mut ast::ExprContinue) -> compile::Result<()> {
    Ok(())
}

#[instrument(span = ast)]
fn expr_yield(idx: &mut Indexer<'_>, ast: &mut ast::ExprYield) -> compile::Result<()> {
    let span = ast.span();
    idx.scopes.mark_yield(span)?;

    if let Some(e) = &mut ast.expr {
        expr(idx, e, IS_USED)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn expr_return(idx: &mut Indexer<'_>, ast: &mut ast::ExprReturn) -> compile::Result<()> {
    if let Some(e) = &mut ast.expr {
        expr(idx, e, IS_USED)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn expr_await(idx: &mut Indexer<'_>, ast: &mut ast::ExprAwait) -> compile::Result<()> {
    let span = ast.span();
    idx.scopes.mark_await(span)?;
    expr(idx, &mut ast.expr, IS_USED)?;
    Ok(())
}

#[instrument(span = ast)]
fn expr_try(idx: &mut Indexer<'_>, ast: &mut ast::ExprTry) -> compile::Result<()> {
    expr(idx, &mut ast.expr, IS_USED)?;
    Ok(())
}

#[instrument(span = ast)]
fn expr_select(idx: &mut Indexer<'_>, ast: &mut ast::ExprSelect) -> compile::Result<()> {
    idx.scopes.mark_await(ast.span())?;

    let mut default_branch = None;

    for (branch, _) in &mut ast.branches {
        match branch {
            ast::ExprSelectBranch::Pat(p) => {
                // NB: expression to evaluate future is evaled in parent scope.
                expr(idx, &mut p.expr, IS_USED)?;

                let _guard = idx.scopes.push_scope();
                pat(idx, &mut p.pat, NOT_USED)?;
                expr(idx, &mut p.body, IS_USED)?;
            }
            ast::ExprSelectBranch::Default(def) => {
                default_branch = Some(def);
            }
        }
    }

    if let Some(def) = default_branch {
        let _guard = idx.scopes.push_scope();
        expr(idx, &mut def.body, IS_USED)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn expr_call(idx: &mut Indexer<'_>, ast: &mut ast::ExprCall) -> compile::Result<()> {
    ast.id.set(idx.items.id().with_span(ast.span())?);

    for (e, _) in &mut ast.args {
        expr(idx, e, IS_USED)?;
    }

    expr(idx, &mut ast.expr, IS_USED)?;
    Ok(())
}

#[instrument(span = ast)]
fn expr_lit(idx: &mut Indexer<'_>, ast: &mut ast::ExprLit) -> compile::Result<()> {
    if let Some(first) = ast.attributes.first() {
        return Err(compile::Error::msg(
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

#[instrument(span = ast)]
fn expr_tuple(idx: &mut Indexer<'_>, ast: &mut ast::ExprTuple) -> compile::Result<()> {
    for (e, _) in &mut ast.items {
        expr(idx, e, IS_USED)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn expr_vec(idx: &mut Indexer<'_>, ast: &mut ast::ExprVec) -> compile::Result<()> {
    for (e, _) in &mut ast.items {
        expr(idx, e, IS_USED)?;
    }

    Ok(())
}

#[instrument(span = ast)]
fn expr_object(idx: &mut Indexer<'_>, ast: &mut ast::ExprObject) -> compile::Result<()> {
    match &mut ast.ident {
        ast::ObjectIdent::Named(p) => {
            // Not a variable use: Name of the object.
            path(idx, p, NOT_USED)?;
        }
        ast::ObjectIdent::Anonymous(..) => (),
    }

    for (assign, _) in &mut ast.assignments {
        if let Some((_, e)) = &mut assign.assign {
            expr(idx, e, IS_USED)?;
        }
    }

    Ok(())
}

#[instrument(span = ast)]
fn expr_range(idx: &mut Indexer<'_>, ast: &mut ast::ExprRange) -> compile::Result<()> {
    if let Some(from) = &mut ast.from {
        expr(idx, from, IS_USED)?;
    }

    if let Some(to) = &mut ast.to {
        expr(idx, to, IS_USED)?;
    }

    Ok(())
}

/// Construct visibility from ast.
fn ast_to_visibility(vis: &ast::Visibility) -> compile::Result<Visibility> {
    let span = match vis {
        ast::Visibility::Inherited => return Ok(Visibility::Inherited),
        ast::Visibility::Public(..) => return Ok(Visibility::Public),
        ast::Visibility::Crate(..) => return Ok(Visibility::Crate),
        ast::Visibility::Super(..) => return Ok(Visibility::Super),
        ast::Visibility::SelfValue(..) => return Ok(Visibility::SelfValue),
        ast::Visibility::In(restrict) => restrict.span(),
    };

    Err(compile::Error::new(
        span,
        CompileErrorKind::UnsupportedVisibility,
    ))
}
