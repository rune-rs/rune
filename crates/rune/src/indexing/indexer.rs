use rust_alloc::rc::Rc;

use core::mem::replace;
use core::num::NonZeroUsize;

use crate::alloc::path::Path;
use crate::alloc::prelude::*;
use crate::alloc::{self, HashMap, VecDeque};
use crate::ast::spanned;
use crate::ast::{self, Span, Spanned};
use crate::compile::attrs;
use crate::compile::{
    self, Doc, DynLocation, Error, ErrorKind, ItemId, ItemMeta, ModId, Visibility, WithSpan,
};
use crate::grammar::{Ignore, Node, Tree};
use crate::macros::MacroCompiler;
use crate::parse::{Parse, Parser, Resolve};
use crate::query::{BuiltInFile, BuiltInFormat, BuiltInLine, BuiltInMacro, BuiltInTemplate, Query};
use crate::runtime::{format, Call};
use crate::worker::{LoadFileKind, Task};
use crate::SourceId;

use super::{Guard, Items, Layer, Scopes};

/// Macros are only allowed to expand recursively into other macros 64 times.
const MAX_MACRO_RECURSION: usize = 64;

pub(crate) struct Indexer<'a, 'arena> {
    /// Query engine.
    pub(crate) q: Query<'a, 'arena>,
    pub(crate) source_id: SourceId,
    pub(crate) items: Items,
    /// Helper to calculate details about an indexed scope.
    pub(crate) scopes: Scopes,
    /// The current item state.
    pub(crate) item: IndexItem,
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
    pub(crate) root: Option<&'a Path>,
    /// Imports to process.
    pub(crate) queue: Option<&'a mut VecDeque<Task>>,
    /// Loaded modules.
    pub(crate) loaded: Option<&'a mut HashMap<ModId, (SourceId, Span)>>,
    /// The current tree being processed.
    pub(crate) tree: &'a Rc<Tree>,
}

impl<'a> Ignore<'a> for Indexer<'_, '_> {
    /// Report an error.
    fn error(&mut self, error: Error) -> alloc::Result<()> {
        self.q.diagnostics.error(self.source_id, error)
    }

    fn ignore(&mut self, _: Node<'a>) -> compile::Result<()> {
        Ok(())
    }
}

impl Indexer<'_, '_> {
    /// Push an identifier item.
    pub(super) fn push_id(&mut self) -> alloc::Result<Guard> {
        let id = self.q.pool.next_id(self.item.id);
        self.items.push_id(id)
    }

    /// Insert a new item at the current indexed location.
    pub(crate) fn insert_new_item(
        &mut self,
        span: &dyn Spanned,
        visibility: Visibility,
        docs: &[Doc],
    ) -> compile::Result<ItemMeta> {
        self.q.insert_new_item(
            &self.items,
            self.item.module,
            self.item.impl_item,
            &DynLocation::new(self.source_id, span),
            visibility,
            docs,
        )
    }

    /// Indicate that we've entered an expanded macro context, and ensure that
    /// we don't blow past [`MAX_MACRO_RECURSION`].
    ///
    /// This is used when entering expressions which have been expanded from a
    /// macro - cause those expression might in turn be macros themselves.
    pub(super) fn enter_macro<S>(&mut self, span: &S) -> compile::Result<()>
    where
        S: Spanned,
    {
        self.macro_depth = self.macro_depth.wrapping_add(1);

        if self.macro_depth >= MAX_MACRO_RECURSION {
            return Err(compile::Error::new(
                span,
                ErrorKind::MaxMacroRecursion {
                    depth: self.macro_depth,
                    max: MAX_MACRO_RECURSION,
                },
            ));
        }

        Ok(())
    }

    /// Leave the last macro context.
    pub(super) fn leave_macro(&mut self) {
        self.macro_depth = self.macro_depth.wrapping_sub(1);
    }

    /// Try to expand an internal macro.
    pub(super) fn try_expand_internal_macro(
        &mut self,
        p: &mut attrs::Parser,
        ast: &mut ast::MacroCall,
    ) -> compile::Result<bool> {
        let Some((_, builtin)) =
            p.try_parse::<attrs::BuiltIn>(resolve_context!(self.q), &ast.attributes)?
        else {
            return Ok(false);
        };

        let args = builtin.args(resolve_context!(self.q))?;

        // NB: internal macros are
        let Some(ident) = ast.path.try_as_ident() else {
            return Err(compile::Error::new(
                &ast.path,
                ErrorKind::NoSuchBuiltInMacro {
                    name: ast.path.resolve(resolve_context!(self.q))?,
                },
            ));
        };

        let ident = ident.resolve(resolve_context!(self.q))?;

        let mut internal_macro = match ident {
            "template" => self.expand_template_macro(ast, &args)?,
            "format" => self.expand_format_macro(ast, &args)?,
            "file" => self.expand_file_macro(ast)?,
            "line" => self.expand_line_macro(ast)?,
            _ => {
                return Err(compile::Error::new(
                    &ast.path,
                    ErrorKind::NoSuchBuiltInMacro {
                        name: ast.path.resolve(resolve_context!(self.q))?,
                    },
                ))
            }
        };

        match &mut internal_macro {
            BuiltInMacro::Template(template) => {
                for e in &mut template.exprs {
                    super::index::expr(self, e)?;
                }
            }
            BuiltInMacro::Format(format) => {
                super::index::expr(self, &mut format.value)?;
            }

            BuiltInMacro::Line(_) | BuiltInMacro::File(_) => { /* Nothing to index */ }
        }

        let id = self.q.insert_new_builtin_macro(internal_macro)?;
        ast.id = Some(id);
        Ok(true)
    }

    /// Expand the template macro.
    fn expand_template_macro(
        &mut self,
        ast: &ast::MacroCall,
        args: &attrs::BuiltInArgs,
    ) -> compile::Result<BuiltInMacro> {
        let mut p = Parser::from_token_stream(&ast.input, ast.span());
        let mut exprs = Vec::new();

        while !p.is_eof()? {
            exprs.try_push(p.parse::<ast::Expr>()?)?;

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
        ast: &ast::MacroCall,
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
                            key,
                            "Multiple `format!(.., fill = ..)`",
                        ));
                    }

                    let arg = p.parse::<ast::LitChar>()?;
                    let f = arg.resolve(resolve_context!(self.q))?;

                    fill = Some(f);
                }
                "align" => {
                    if align.is_some() {
                        return Err(compile::Error::unsupported(
                            key,
                            "Multiple `format!(.., align = ..)`",
                        ));
                    }

                    let arg = p.parse::<ast::Ident>()?;
                    let a = arg.resolve(resolve_context!(self.q))?;

                    let Ok(a) = str::parse::<format::Alignment>(a) else {
                        return Err(compile::Error::unsupported(
                            key,
                            "`format!(.., align = ..)`",
                        ));
                    };

                    align = Some(a);
                }
                "flags" => {
                    if flags.is_some() {
                        return Err(compile::Error::unsupported(
                            key,
                            "Multiple `format!(.., flags = ..)`",
                        ));
                    }

                    let arg = p.parse::<ast::LitNumber>()?;

                    let Some(f) = arg.resolve(resolve_context!(self.q))?.as_u32(false) else {
                        return Err(compile::Error::msg(arg, "Argument out-of-bounds"));
                    };

                    let f = format::Flags::from(f);
                    flags = Some(f);
                }
                "width" => {
                    if width.is_some() {
                        return Err(compile::Error::unsupported(
                            key,
                            "Multiple `format!(.., width = ..)`",
                        ));
                    }

                    let arg = p.parse::<ast::LitNumber>()?;

                    let Some(f) = arg.resolve(resolve_context!(self.q))?.as_usize(false) else {
                        return Err(compile::Error::msg(arg, "Argument out-of-bounds"));
                    };

                    width = NonZeroUsize::new(f);
                }
                "precision" => {
                    if precision.is_some() {
                        return Err(compile::Error::unsupported(
                            key,
                            "Multiple `format!(.., precision = ..)`",
                        ));
                    }

                    let arg = p.parse::<ast::LitNumber>()?;

                    let Some(f) = arg.resolve(resolve_context!(self.q))?.as_usize(false) else {
                        return Err(compile::Error::msg(arg, "Argument out-of-bounds"));
                    };

                    precision = NonZeroUsize::new(f);
                }
                "type" => {
                    if format_type.is_some() {
                        return Err(compile::Error::unsupported(
                            key,
                            "Multiple `format!(.., type = ..)`",
                        ));
                    }

                    let arg = p.parse::<ast::Ident>()?;
                    let a = arg.resolve(resolve_context!(self.q))?;

                    format_type = Some(match str::parse::<format::Type>(a) {
                        Ok(format_type) => format_type,
                        _ => {
                            return Err(compile::Error::unsupported(
                                key,
                                "`format!(.., type = ..)`",
                            ));
                        }
                    });
                }
                _ => {
                    return Err(compile::Error::unsupported(key, "`format!(.., <key>)`"));
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
    fn expand_file_macro(&mut self, ast: &ast::MacroCall) -> compile::Result<BuiltInMacro> {
        let name = self.q.sources.name(self.source_id).ok_or_else(|| {
            compile::Error::new(
                ast,
                ErrorKind::MissingSourceId {
                    source_id: self.source_id,
                },
            )
        })?;
        let id = self.q.storage.insert_str(name)?;
        let source = ast::StrSource::Synthetic(id);
        let value = ast::Lit::Str(ast::LitStr {
            span: ast.span(),
            source,
        });

        Ok(BuiltInMacro::File(BuiltInFile { value }))
    }

    /// Expand a macro returning the current line for where the macro invocation begins
    fn expand_line_macro(&mut self, ast: &ast::MacroCall) -> compile::Result<BuiltInMacro> {
        let (l, _) = self
            .q
            .sources
            .get(self.source_id)
            .map(|s| s.find_line_column(ast.open.span.start.into_usize()))
            .unwrap_or_default();

        // 1-indexed as that is what most editors will use
        let id = self.q.storage.insert_number(l + 1)?;
        let source = ast::NumberSource::Synthetic(id);

        Ok(BuiltInMacro::Line(BuiltInLine {
            value: ast::Lit::Number(ast::LitNumber {
                span: ast.span(),
                source,
            }),
        }))
    }

    /// Perform a macro expansion.
    pub(super) fn expand_macro<T>(&mut self, ast: &mut ast::MacroCall) -> compile::Result<T>
    where
        T: Parse,
    {
        ast.path.id = self.item.id;

        let item = self.q.item_for("macro", self.item.id).with_span(&ast)?;

        let mut compiler = MacroCompiler {
            item_meta: item,
            idx: self,
        };

        compiler.eval_macro::<T>(ast)
    }

    /// Perform an attribute macro expansion.
    pub(super) fn expand_attribute_macro<T>(
        &mut self,
        attr: &mut ast::Attribute,
        item: &ast::Item,
    ) -> compile::Result<Option<T>>
    where
        T: Parse,
    {
        attr.path.id = self.item.id;

        let containing = self
            .q
            .item_for("attribute macro", self.item.id)
            .with_span(&*attr)?;

        let mut compiler = MacroCompiler {
            item_meta: containing,
            idx: self,
        };

        compiler.eval_attribute_macro::<T>(attr, item)
    }

    /// Handle a filesystem module.
    pub(super) fn handle_file_mod(
        &mut self,
        ast: &mut ast::ItemMod,
        docs: &[Doc],
    ) -> compile::Result<()> {
        let name = ast.name.resolve(resolve_context!(self.q))?;
        let visibility = ast_to_visibility(&ast.visibility)?;
        let guard = self.items.push_name(name.as_ref())?;

        let (mod_item, mod_item_id) = self.q.insert_mod(
            &self.items,
            &DynLocation::new(self.source_id, spanned::from_fn(|| ast.name_span())),
            self.item.module,
            visibility,
            docs,
        )?;

        self.items.pop(guard).with_span(&*ast)?;

        ast.id = mod_item_id;

        let Some(root) = &self.root else {
            return Err(compile::Error::new(
                &*ast,
                ErrorKind::UnsupportedModuleSource,
            ));
        };

        let source = self
            .q
            .source_loader
            .load(root, self.q.pool.module_item(mod_item), &*ast)?;

        if let Some(loaded) = self.loaded.as_mut() {
            if let Some(_existing) = loaded.try_insert(mod_item, (self.source_id, ast.span()))? {
                return Err(compile::Error::new(
                    &*ast,
                    ErrorKind::ModAlreadyLoaded {
                        item: self.q.pool.module_item(mod_item).try_to_owned()?,
                        #[cfg(feature = "emit")]
                        existing: _existing,
                    },
                ));
            }
        }

        let source_id = self.q.sources.insert(source)?;

        self.q
            .visitor
            .visit_mod(&DynLocation::new(source_id, &*ast))
            .with_span(&*ast)?;

        if let Some(queue) = self.queue.as_mut() {
            queue.try_push_back(Task::LoadFile {
                kind: LoadFileKind::Module {
                    root: self.root.map(|p| p.try_to_owned()).transpose()?,
                },
                source_id,
                span: ast.span(),
                mod_item,
                mod_item_id,
            })?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct IndexItem {
    /// The current module being indexed.
    pub(crate) module: ModId,
    /// Whether the item has been inserted or not.
    pub(crate) id: ItemId,
    /// Set if we are inside of an impl self.
    pub(crate) impl_item: Option<ItemId>,
}

impl IndexItem {
    pub(crate) fn new(module: ModId, id: ItemId) -> Self {
        Self {
            module,
            id,
            impl_item: None,
        }
    }

    pub(crate) fn with_impl_item(module: ModId, id: ItemId, impl_item: ItemId) -> Self {
        Self {
            module,
            id,
            impl_item: Some(impl_item),
        }
    }

    /// Replace item we're currently in.
    #[tracing::instrument(skip(self), fields(self.module = ?self.module, self.id = ?self.id, self.impl_item = ?self.impl_item))]
    pub(super) fn replace(&mut self, id: ItemId) -> IndexItem {
        tracing::debug!("replacing item");

        IndexItem {
            module: self.module,
            id: replace(&mut self.id, id),
            impl_item: self.impl_item,
        }
    }

    /// Replace module id.
    pub(super) fn replace_module(&mut self, module: ModId, id: ItemId) -> IndexItem {
        IndexItem {
            module: replace(&mut self.module, module),
            id: replace(&mut self.id, id),
            impl_item: self.impl_item,
        }
    }
}

/// Construct visibility from ast.
pub(super) fn ast_to_visibility(vis: &ast::Visibility) -> compile::Result<Visibility> {
    let span = match vis {
        ast::Visibility::Inherited => return Ok(Visibility::Inherited),
        ast::Visibility::Public(..) => return Ok(Visibility::Public),
        ast::Visibility::Crate(..) => return Ok(Visibility::Crate),
        ast::Visibility::Super(..) => return Ok(Visibility::Super),
        ast::Visibility::SelfValue(..) => return Ok(Visibility::SelfValue),
        ast::Visibility::In(restrict) => restrict.span(),
    };

    Err(compile::Error::new(span, ErrorKind::UnsupportedVisibility))
}

/// Construct the calling convention based on the parameters.
pub(super) fn validate_call(
    is_const: bool,
    is_async: bool,
    layer: &Layer,
) -> compile::Result<Option<Call>> {
    for span in &layer.awaits {
        if is_const {
            return Err(compile::Error::new(span, ErrorKind::AwaitInConst));
        }

        if !is_async {
            return Err(compile::Error::new(span, ErrorKind::AwaitOutsideAsync));
        }
    }

    for span in &layer.yields {
        if is_const {
            return Err(compile::Error::new(span, ErrorKind::YieldInConst));
        }
    }

    if is_const {
        return Ok(None);
    }

    Ok(match (!layer.yields.is_empty(), is_async) {
        (true, false) => Some(Call::Generator),
        (false, false) => Some(Call::Immediate),
        (true, true) => Some(Call::Stream),
        (false, true) => Some(Call::Async),
    })
}
