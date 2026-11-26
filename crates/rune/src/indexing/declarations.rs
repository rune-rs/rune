//! Signatures extracted from a declaration source (`.d.rn`).
//!
//! A declaration source describes what a host provides natively: functions
//! without bodies, type definitions, modules and `impl` blocks of signatures,
//! with their documentation. It is parsed by the same grammar as any other
//! source, in declaration mode, and what this module extracts from the
//! resulting tree is all that is taken from it. Nothing in it is compiled into
//! a unit.
//!
//! [`register`] turns what [`file`] extracts into items the compiler resolves
//! against, the same way it resolves against what a host installed natively.

use crate::alloc::borrow::Cow;
use crate::alloc::prelude::*;
use crate::alloc::{Box, HashSet, String, Vec};
use crate::ast::{self, Kind, Span};
use crate::compile::{meta, Error, ItemMeta, Location, ModId, Result, Visibility, WithSpan};
use crate::grammar::{ws, Node, Tree};
use crate::parse::ResolveContext;
use crate::query::Query;
use crate::{Diagnostics, SourceId};
use crate::{Hash, Item, ItemBuf};

use Kind::*;

/// A declaration extracted from a declaration source.
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "not all of a declaration is registered as item metadata yet"
)]
pub(crate) struct Declaration {
    /// The span of the whole declaration, attributes included.
    pub(crate) span: Span,
    /// The modules the declaration is nested in, outermost first.
    pub(crate) module: Vec<String>,
    /// The lines of documentation, from doc comments and `#[doc = ..]`.
    pub(crate) docs: Vec<String>,
    /// Attributes other than documentation, as written.
    pub(crate) attributes: Vec<String>,
    /// Whether the declaration is `pub`.
    pub(crate) is_pub: bool,
    /// What is declared.
    pub(crate) kind: DeclarationKind,
}

/// What a [`Declaration`] declares.
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "not all of a declaration is registered as item metadata yet"
)]
pub(crate) enum DeclarationKind {
    /// A function signature.
    Fn(Signature),
    /// A struct definition.
    Struct(TypeDef),
    /// An enum definition, with its variants.
    Enum {
        /// The name of the enum.
        name: String,
        /// The variants of the enum.
        variants: Vec<TypeDef>,
    },
    /// A `use` declaration, as written.
    Use(String),
}

/// A function signature.
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "not all of a declaration is registered as item metadata yet"
)]
pub(crate) struct Signature {
    /// The name of the function.
    pub(crate) name: String,
    /// The path of the `impl` block the function is declared in, as written.
    pub(crate) impl_of: Option<String>,
    /// Whether the function is `async`.
    pub(crate) is_async: bool,
    /// The arguments of the function.
    pub(crate) args: Vec<Arg>,
    /// The return type, as written.
    pub(crate) output: Option<String>,
}

/// An argument in a [`Signature`].
#[derive(Debug)]
pub(crate) struct Arg {
    /// The pattern of the argument, as written.
    pub(crate) pat: String,
    /// The type of the argument, as written.
    pub(crate) ty: Option<String>,
}

/// A struct or a variant.
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "not all of a declaration is registered as item metadata yet"
)]
pub(crate) struct TypeDef {
    /// The name of the type or variant.
    pub(crate) name: String,
    /// The documentation of a variant; that of a struct is on its
    /// declaration.
    pub(crate) docs: Vec<String>,
    /// Its fields, by name for a struct body and by position for a tuple body.
    pub(crate) fields: Vec<Field>,
}

/// A field of a [`TypeDef`].
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "not all of a declaration is registered as item metadata yet"
)]
pub(crate) struct Field {
    /// The name of the field, or its position in a tuple body.
    pub(crate) name: String,
    /// The type of the field, as written.
    pub(crate) ty: Option<String>,
}

/// A body of items still to be walked.
struct Pending<'a> {
    /// The node whose children are items: the root or a block body.
    body: Node<'a>,
    /// The modules the body is nested in.
    module: Vec<String>,
    /// The `impl` block the body belongs to, which only takes functions.
    impl_of: Option<String>,
}

/// Extract the declarations from the tree of a declaration source, reporting
/// everything which is not a declaration to `diagnostics`.
///
/// Modules nest, so the bodies are walked over an explicit stack.
pub(crate) fn file(
    cx: ResolveContext<'_, '_>,
    source_id: SourceId,
    tree: &Tree,
    diagnostics: &mut Diagnostics,
) -> Result<Vec<Declaration>> {
    let mut x = Extract {
        cx,
        source_id,
        diagnostics,
        out: Vec::new(),
        stack: Vec::new(),
    };

    tree.parse_all(|p| {
        let span = p.span();

        x.stack
            .try_push(Pending {
                body: p.node(),
                module: Vec::new(),
                impl_of: None,
            })
            .with_span(span)?;

        p.ignore();
        Ok(())
    })?;

    while let Some(pending) = x.stack.pop() {
        x.body(pending)?;
    }

    Ok(x.out)
}

struct Extract<'a, 'cx, 's> {
    cx: ResolveContext<'cx, 's>,
    source_id: SourceId,
    diagnostics: &'a mut Diagnostics,
    out: Vec<Declaration>,
    stack: Vec<Pending<'a>>,
}

impl<'a> Extract<'a, '_, '_> {
    fn error(&mut self, span: Span, message: &'static str) -> Result<()> {
        self.diagnostics
            .error(self.source_id, Error::msg(span, message))
            .with_span(span)?;
        Ok(())
    }

    /// The source text a span covers.
    fn text(&self, span: Span) -> Result<String> {
        let text = self
            .cx
            .sources
            .source(self.source_id, span)
            .unwrap_or_default();
        Ok(text.try_to_owned().with_span(span)?)
    }

    fn body(&mut self, pending: Pending<'a>) -> Result<()> {
        for node in pending.body.children() {
            match node.kind() {
                ws!() | K![;] | K!['{'] | K!['}'] => {}
                // Inner doc comments document the source as a whole.
                InnerAttribute => {}
                Item => self.item(node, &pending)?,
                _ => {
                    self.error(
                        node.span(),
                        "only declarations are allowed in a declaration source",
                    )?;
                }
            }
        }

        Ok(())
    }

    fn item(&mut self, item: Node<'a>, pending: &Pending<'a>) -> Result<()> {
        let span = item.span();
        let mut docs = Vec::new();
        let mut attributes = Vec::new();
        for node in item.children() {
            match node.kind() {
                ws!() => {}
                Attribute => {
                    if let Some(doc) = self.doc(&node)? {
                        docs.try_push(doc).with_span(span)?;
                    } else {
                        attributes
                            .try_push(self.text(node.span())?)
                            .with_span(span)?;
                    }
                }
                kind => {
                    if pending.impl_of.is_some() && kind != ItemFn {
                        self.error(
                            node.span(),
                            "an `impl` block in a declaration source may only declare functions",
                        )?;
                        continue;
                    }

                    // The grammar puts the modifiers inside the item they
                    // modify. Those of an item which is rejected outright are
                    // not worth reporting as well.
                    let (is_pub, is_async) = match kind {
                        ItemConst | ItemStatic | ItemFileMod => (false, false),
                        _ => self.modifiers(&node)?,
                    };

                    let kind = match kind {
                        ItemFn => {
                            let Some(signature) = self.item_fn(node, pending, is_async)? else {
                                continue;
                            };

                            DeclarationKind::Fn(signature)
                        }
                        ItemStruct => DeclarationKind::Struct(self.type_def(node)?),
                        ItemEnum => self.item_enum(node)?,
                        ItemUse => DeclarationKind::Use(self.text(node.span())?),
                        ItemMod => {
                            self.nested(node, pending, None)?;
                            continue;
                        }
                        ItemImpl => {
                            let path = node
                                .children()
                                .find(|n| n.kind() == Path)
                                .map(|n| self.text(n.span()))
                                .transpose()?;
                            self.nested(node, pending, path)?;
                            continue;
                        }
                        ItemFileMod => {
                            self.error(
                                node.span(),
                                "a declaration source may only contain inline `mod` blocks",
                            )?;
                            continue;
                        }
                        ItemConst | ItemStatic => {
                            self.error(
                                node.span(),
                                "constants are not allowed in a declaration source",
                            )?;
                            continue;
                        }
                        _ => {
                            self.error(
                                node.span(),
                                "only declarations are allowed in a declaration source",
                            )?;
                            continue;
                        }
                    };

                    if is_async && !matches!(kind, DeclarationKind::Fn(..)) {
                        self.error(span, "only a function may be `async`")?;
                    }

                    let module = clone_module(&pending.module, span)?;

                    self.out
                        .try_push(Declaration {
                            span,
                            module,
                            docs,
                            attributes,
                            is_pub,
                            kind,
                        })
                        .with_span(span)?;

                    return Ok(());
                }
            }
        }

        Ok(())
    }

    /// Whether an item is `pub` and whether it is `async`, reporting any
    /// other modifier.
    fn modifiers(&mut self, node: &Node<'a>) -> Result<(bool, bool)> {
        let mut is_pub = false;
        let mut is_async = false;

        let Some(mods) = node.children().find(|n| n.kind() == Modifiers) else {
            return Ok((is_pub, is_async));
        };

        for m in mods.children() {
            match m.kind() {
                ws!() => {}
                K![pub] | ModifierSuper | ModifierSelf | ModifierCrate | ModifierIn => {
                    is_pub = true;
                }
                K![async] => {
                    is_async = true;
                }
                _ => {
                    self.error(
                        m.span(),
                        "only `pub` and `async` are allowed on a declaration",
                    )?;
                }
            }
        }

        Ok((is_pub, is_async))
    }

    /// Queue the block body of a `mod` or `impl` item to be walked.
    fn nested(
        &mut self,
        node: Node<'a>,
        pending: &Pending<'a>,
        impl_of: Option<String>,
    ) -> Result<()> {
        let span = node.span();
        let mut module = clone_module(&pending.module, span)?;

        if impl_of.is_none() {
            if let Some(name) = node.children().find(|n| matches!(n.kind(), K![ident])) {
                module.try_push(self.text(name.span())?).with_span(span)?;
            }
        }

        let body = node
            .children()
            .find(|n| n.kind() == Block)
            .and_then(|n| n.children().find(|n| n.kind() == BlockBody));

        if let Some(body) = body {
            self.stack
                .try_push(Pending {
                    body,
                    module,
                    impl_of,
                })
                .with_span(span)?;
        }

        Ok(())
    }

    fn item_fn(
        &mut self,
        node: Node<'a>,
        pending: &Pending<'a>,
        is_async: bool,
    ) -> Result<Option<Signature>> {
        let span = node.span();
        let mut name = None;
        let mut args = Vec::new();
        let mut output = None;
        let mut has_body = false;

        for n in node.children() {
            match n.kind() {
                K![ident] => {
                    name = Some(self.text(n.span())?);
                }
                FnArgs => {
                    // A type follows the pattern it annotates as `: Type`.
                    let mut annotated = false;

                    for c in n.children() {
                        match c.kind() {
                            Pat => {
                                let pat = self.text(c.span())?;
                                args.try_push(Arg { pat, ty: None }).with_span(span)?;
                                annotated = false;
                            }
                            K![:] => {
                                annotated = true;
                            }
                            Type if annotated => {
                                if let Some(arg) = args.last_mut() {
                                    arg.ty = Some(self.text(c.span())?);
                                }

                                annotated = false;
                            }
                            _ => {}
                        }
                    }
                }
                Type => {
                    output = Some(self.text(n.span())?);
                }
                Block => {
                    has_body = true;
                    self.error(n.span(), "a function in a declaration source has no body")?;
                }
                _ => {}
            }
        }

        let Some(name) = name else {
            self.error(span, "expected function name")?;
            return Ok(None);
        };

        if has_body {
            return Ok(None);
        }

        let impl_of = match &pending.impl_of {
            Some(path) => Some(path.try_clone().with_span(span)?),
            None => None,
        };

        Ok(Some(Signature {
            name,
            impl_of,
            is_async,
            args,
            output,
        }))
    }

    /// A struct, or a variant of an enum.
    fn type_def(&mut self, node: Node<'a>) -> Result<TypeDef> {
        let span = node.span();
        let mut name = String::new();
        let mut docs = Vec::new();
        let mut fields = Vec::new();

        for n in node.children() {
            match n.kind() {
                K![ident] => {
                    name = self.text(n.span())?;
                }
                Attribute => {
                    if let Some(doc) = self.doc(&n)? {
                        docs.try_push(doc).with_span(span)?;
                    }
                }
                StructBody | TupleBody => {
                    let tuple = n.kind() == TupleBody;

                    for (index, field) in n.children().filter(|n| n.kind() == Field).enumerate() {
                        let mut name = None;
                        let mut ty = None;

                        for f in field.children() {
                            match f.kind() {
                                K![ident] if name.is_none() => {
                                    name = Some(self.text(f.span())?);
                                }
                                Type => {
                                    ty = Some(self.text(f.span())?);
                                }
                                _ => {}
                            }
                        }

                        let name = match (tuple, name) {
                            (false, Some(name)) => name,
                            _ => index.try_to_string().with_span(span)?,
                        };

                        fields.try_push(Field { name, ty }).with_span(span)?;
                    }
                }
                _ => {}
            }
        }

        Ok(TypeDef { name, docs, fields })
    }

    fn item_enum(&mut self, node: Node<'a>) -> Result<DeclarationKind> {
        let span = node.span();
        let mut name = String::new();
        let mut variants = Vec::new();

        for n in node.children() {
            match n.kind() {
                K![ident] => {
                    name = self.text(n.span())?;
                }
                Variant => {
                    variants.try_push(self.type_def(n)?).with_span(span)?;
                }
                _ => {}
            }
        }

        Ok(DeclarationKind::Enum { name, variants })
    }

    /// The documentation an attribute carries, if it is `#[doc = ..]`, which
    /// is also what the lexer turns a doc comment into.
    fn doc(&mut self, node: &Node<'a>) -> Result<Option<String>> {
        let Some(stream) = node.children().find(|n| n.kind() == TokenStream) else {
            return Ok(None);
        };

        let mut tokens = stream
            .children()
            .filter(|n| !matches!(n.kind(), ws!()))
            .map(|n| n.token());

        let (Some(ident), Some(eq), Some(lit), None) =
            (tokens.next(), tokens.next(), tokens.next(), tokens.next())
        else {
            return Ok(None);
        };

        // The lexer names the attribute it makes out of a doc comment with a
        // built-in identifier, since there is no `doc` in the source for it.
        let is_doc = match ident.kind {
            ast::Kind::Ident(ast::LitSource::BuiltIn(builtin)) => {
                matches!(builtin, ast::BuiltIn::Doc)
            }
            K![ident] => self.cx.sources.source(self.source_id, ident.span) == Some("doc"),
            _ => false,
        };

        if !is_doc || eq.kind != K![=] {
            return Ok(None);
        }

        let ast::Kind::Str(source) = lit.kind else {
            return Ok(None);
        };

        let lit = ast::LitStr {
            span: lit.span,
            source,
        };

        let doc = lit.resolve_string(self.cx)?;
        Ok(Some(doc.as_ref().try_to_owned().with_span(lit.span)?))
    }
}

fn clone_module(module: &[String], span: Span) -> Result<Vec<String>> {
    let mut out = Vec::new();

    for m in module {
        out.try_push(m.try_clone().with_span(span)?)
            .with_span(span)?;
    }

    Ok(out)
}

/// Register what a declaration source declares as items the compiler
/// resolves against, as if a host had installed them natively.
///
/// A declaration describes something the host provides, so an item of the
/// same name which the native context already has wins and the declaration
/// is skipped rather than reported as a conflict: the context is what will
/// be called at run time, and a declaration source written for an older
/// version of the host should not stop a script compiling against a newer
/// one.
///
/// Every declaration is public. It stands for a native item, and native
/// items have no visibility of their own.
pub(crate) fn register(
    q: &mut Query<'_, '_>,
    mod_item: ModId,
    source_id: SourceId,
    declarations: &[Declaration],
) -> Result<()> {
    let types = types(q, mod_item, declarations)?;

    for d in declarations {
        let span = d.span;

        let mut base = q
            .pool
            .module_item(mod_item)
            .try_to_owned()
            .with_span(span)?;

        for m in &d.module {
            base.push(m.as_str()).with_span(span)?;
        }

        let types = Types {
            hashes: types.hashes.try_clone().with_span(span)?,
            modules: modules(q, mod_item, &d.module, span)?,
        };

        match &d.kind {
            DeclarationKind::Fn(f) => {
                let mut item = base;
                let mut container = None;

                if let Some(impl_of) = &f.impl_of {
                    for part in impl_of.split("::").map(str::trim) {
                        item.push(part).with_span(span)?;
                    }

                    container = Some(Hash::type_hash(&item));
                }

                item.push(f.name.as_str()).with_span(span)?;

                let is_instance = f
                    .args
                    .first()
                    .is_some_and(|a| a.pat == "self" || a.pat == "mut self");

                let kind = meta::Kind::Function {
                    associated: match is_instance {
                        true => Some(meta::AssociatedKind::Instance(Cow::Owned(
                            f.name.try_clone().with_span(span)?,
                        ))),
                        false => None,
                    },
                    trait_hash: None,
                    signature: signature(q, &types, f, span)?,
                    is_test: false,
                    is_bench: false,
                    parameters: Hash::EMPTY,
                    #[cfg(feature = "doc")]
                    container,
                    #[cfg(feature = "doc")]
                    parameter_types: Vec::new(),
                };

                #[cfg(not(feature = "doc"))]
                let _ = container;

                insert(q, mod_item, source_id, d, &d.docs, &item, kind, true)?;
            }
            DeclarationKind::Struct(st) => {
                let mut item = base;
                item.push(st.name.as_str()).with_span(span)?;
                let kind = meta::Kind::Type {
                    parameters: Hash::EMPTY,
                };
                insert(q, mod_item, source_id, d, &d.docs, &item, kind, false)?;
            }
            DeclarationKind::Enum { name, variants } => {
                let mut item = base;
                item.push(name.as_str()).with_span(span)?;
                let enum_hash = Hash::type_hash(&item);
                let kind = meta::Kind::Enum {
                    parameters: Hash::EMPTY,
                };

                if !insert(q, mod_item, source_id, d, &d.docs, &item, kind, false)? {
                    continue;
                }

                // Variants are installed the way a native enum's are: as
                // structs belonging to the enum, constructed by a function
                // the host provides.
                for v in variants {
                    let mut variant = item.try_to_owned().with_span(span)?;
                    variant.push(v.name.as_str()).with_span(span)?;

                    let named = v
                        .fields
                        .first()
                        .is_some_and(|f| f.name.as_str().parse::<usize>().is_err());

                    let fields = if v.fields.is_empty() {
                        meta::Fields::Empty
                    } else if named {
                        let mut fields = Vec::new();

                        for (position, f) in v.fields.iter().enumerate() {
                            fields
                                .try_push(meta::FieldMeta {
                                    name: Box::try_from(f.name.as_str()).with_span(span)?,
                                    position,
                                })
                                .with_span(span)?;
                        }

                        let fields = Box::try_from(fields).with_span(span)?;
                        meta::Fields::Named(meta::FieldsNamed { fields })
                    } else {
                        meta::Fields::Unnamed(v.fields.len())
                    };

                    let kind = meta::Kind::Struct {
                        fields,
                        constructor: Some(constructor(q, &types, &v.fields, enum_hash, span)?),
                        parameters: Hash::EMPTY,
                        enum_hash,
                    };

                    insert(q, mod_item, source_id, d, &v.docs, &variant, kind, true)?;
                }
            }
            // Imports in a declaration source only matter to the
            // declarations they would let refer to each other, which have
            // no types the compiler checks.
            DeclarationKind::Use(..) => {}
        }
    }

    Ok(())
}

/// The types declared by `declarations`.
fn types(q: &Query<'_, '_>, mod_item: ModId, declarations: &[Declaration]) -> Result<Types> {
    let mut hashes = HashSet::new();

    for d in declarations {
        let span = d.span;

        let name = match &d.kind {
            DeclarationKind::Struct(st) => st.name.as_str(),
            DeclarationKind::Enum { name, .. } => name.as_str(),
            _ => continue,
        };

        let mut item = q
            .pool
            .module_item(mod_item)
            .try_to_owned()
            .with_span(span)?;

        for m in &d.module {
            item.push(m.as_str()).with_span(span)?;
        }

        item.push(name).with_span(span)?;
        hashes.try_insert(Hash::type_hash(&item)).with_span(span)?;
    }

    Ok(Types {
        hashes,
        modules: Vec::new(),
    })
}

/// The module a declaration is in, and every module enclosing it, outermost
/// first.
fn modules(
    q: &Query<'_, '_>,
    mod_item: ModId,
    module: &[String],
    span: Span,
) -> Result<Vec<ItemBuf>> {
    let mut item = q
        .pool
        .module_item(mod_item)
        .try_to_owned()
        .with_span(span)?;
    let mut out = Vec::new();
    out.try_push(item.try_clone().with_span(span)?)
        .with_span(span)?;

    for m in module {
        item.push(m.as_str()).with_span(span)?;
        out.try_push(item.try_clone().with_span(span)?)
            .with_span(span)?;
    }

    Ok(out)
}

/// Resolve a type as written in a declaration to the hash documentation
/// refers to it by: a declared type of the same source, a prelude name, or an
/// item of the native context. Anything else, a tuple or a type nobody
/// declared, is left unknown; types are documentation only and not checked.
#[cfg(feature = "doc")]
fn type_hash(q: &Query<'_, '_>, types: &Types, ty: &str) -> Result<Hash, crate::alloc::Error> {
    let ty = ty.trim();

    // Generic parameters are not part of the base type.
    let base = match ty.find("::<").or_else(|| ty.find('<')) {
        Some(n) => ty[..n].trim(),
        None => ty,
    };

    if base.is_empty() || !base.split("::").all(|p| is_ident(p.trim())) {
        return Ok(Hash::EMPTY);
    }

    let mut path = ItemBuf::new();

    for part in base.split("::").map(str::trim) {
        path.push(part)?;
    }

    for module in types.modules.iter().rev() {
        let mut item = module.try_clone()?;

        for part in base.split("::").map(str::trim) {
            item.push(part)?;
        }

        let hash = Hash::type_hash(&item);

        if types.hashes.contains(&hash) {
            return Ok(hash);
        }
    }

    if !base.contains("::") {
        if let Some(item) = q.prelude.get(base) {
            return Ok(Hash::type_hash(item));
        }
    }

    if q.context.lookup_meta(&path).is_some() {
        return Ok(Hash::type_hash(&path));
    }

    Ok(Hash::EMPTY)
}

#[cfg(feature = "doc")]
fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();

    chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// The types a declaration source declares, and the modules the type of an
/// argument is looked up in, innermost last.
#[cfg_attr(not(feature = "doc"), allow(dead_code))]
struct Types {
    hashes: HashSet<Hash>,
    modules: Vec<ItemBuf>,
}

#[cfg(feature = "doc")]
fn doc_type(q: &Query<'_, '_>, types: &Types, ty: Option<&String>, span: Span) -> Result<Hash> {
    match ty {
        Some(ty) => Ok(type_hash(q, types, ty.as_str()).with_span(span)?),
        None => Ok(Hash::EMPTY),
    }
}

fn signature(
    q: &Query<'_, '_>,
    types: &Types,
    f: &Signature,
    span: Span,
) -> Result<meta::Signature> {
    #[cfg(not(feature = "doc"))]
    let _ = (q, types, f, span);

    Ok(meta::Signature {
        #[cfg(feature = "doc")]
        is_async: f.is_async,
        #[cfg(feature = "doc")]
        arguments: {
            let mut args = Vec::new();

            for a in &f.args {
                args.try_push(meta::DocArgument {
                    name: meta::DocName::Name(Box::try_from(a.pat.as_str()).with_span(span)?),
                    base: doc_type(q, types, a.ty.as_ref(), span)?,
                    generics: Box::default(),
                })
                .with_span(span)?;
            }

            Some(Box::try_from(args).with_span(span)?)
        },
        #[cfg(feature = "doc")]
        return_type: meta::DocType::new(doc_type(q, types, f.output.as_ref(), span)?),
    })
}

/// The signature of the constructor of a variant, which returns its enum.
fn constructor(
    q: &Query<'_, '_>,
    types: &Types,
    fields: &[Field],
    enum_hash: Hash,
    span: Span,
) -> Result<meta::Signature> {
    #[cfg(not(feature = "doc"))]
    let _ = (q, types, fields, enum_hash, span);

    Ok(meta::Signature {
        #[cfg(feature = "doc")]
        is_async: false,
        #[cfg(feature = "doc")]
        arguments: {
            let mut args = Vec::new();

            for (n, f) in fields.iter().enumerate() {
                let name = match f.name.as_str().parse::<usize>() {
                    Ok(..) => meta::DocName::Index(n),
                    Err(..) => meta::DocName::Name(Box::try_from(f.name.as_str()).with_span(span)?),
                };

                args.try_push(meta::DocArgument {
                    name,
                    base: doc_type(q, types, f.ty.as_ref(), span)?,
                    generics: Box::default(),
                })
                .with_span(span)?;
            }

            Some(Box::try_from(args).with_span(span)?)
        },
        #[cfg(feature = "doc")]
        return_type: meta::DocType::new(enum_hash),
    })
}

/// Insert the metadata of a declared item, unless the native context already
/// has an item of the same name. Returns whether it was inserted.
///
/// A `callable` item is called by hash at run time, so the host is expected to
/// provide a function for it.
#[allow(clippy::too_many_arguments)]
fn insert(
    q: &mut Query<'_, '_>,
    mod_item: ModId,
    source_id: SourceId,
    d: &Declaration,
    docs: &[String],
    item: &Item,
    kind: meta::Kind,
    callable: bool,
) -> Result<bool> {
    let span = d.span;

    if q.context.lookup_meta(item).is_some() {
        return Ok(false);
    }
    let hash = Hash::type_hash(item);
    let id = q.pool.alloc_item(item).with_span(span)?;
    let location = Location::new(source_id, span);

    let meta = meta::Meta {
        context: false,
        hash,
        item_meta: ItemMeta {
            location,
            item: id,
            visibility: Visibility::Public,
            module: mod_item,
            impl_item: None,
        },
        kind,
        source: Some(meta::SourceMeta {
            location,
            #[cfg(feature = "std")]
            path: q
                .sources
                .path(source_id)
                .map(|p| p.try_into())
                .transpose()
                .with_span(span)?,
        }),
        parameters: Hash::EMPTY,
    };

    q.insert_meta(meta).with_span(span)?;

    for line in docs {
        q.visitor
            .visit_doc_comment(&location, item, hash, line.as_str())
            .with_span(span)?;
    }

    if callable {
        q.unit.insert_declared(hash, item).with_span(span)?;
    }

    Ok(true)
}
