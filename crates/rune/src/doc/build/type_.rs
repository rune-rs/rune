use core::mem::replace;

use anyhow::{Context, Result};
use relative_path::RelativePathBuf;
use serde::Serialize;

use crate::alloc::borrow::Cow;
use crate::alloc::fmt::TryWrite;
use crate::alloc::prelude::*;
use crate::doc::artifacts::TestKind;
use crate::doc::context::{Assoc, AssocFnKind, Kind, Meta};
use crate::item::ComponentRef;
use crate::{Hash, Item};

use super::{Builder, Ctxt, IndexEntry, IndexKind, ItemKind};

#[derive(Serialize)]
pub(super) struct Protocol<'a> {
    name: &'a str,
    field: Option<&'a str>,
    repr: Option<String>,
    return_type: Option<String>,
    doc: Option<String>,
    deprecated: Option<&'a str>,
}

#[derive(Serialize)]
pub(super) struct Method<'a> {
    is_async: bool,
    deprecated: Option<&'a str>,
    name: &'a str,
    args: String,
    parameters: Option<String>,
    return_type: Option<String>,
    line_doc: Option<String>,
    doc: Option<String>,
}

#[derive(Serialize)]
pub(super) struct Variant<'a> {
    name: &'a str,
    line_doc: Option<String>,
    doc: Option<String>,
}

#[derive(Default, Serialize)]
pub(super) struct Trait<'a> {
    #[serde(serialize_with = "super::serialize_item")]
    pub(super) item: &'a Item,
    pub(super) hash: Hash,
    pub(super) module: String,
    pub(super) name: &'a str,
    pub(super) url: RelativePathBuf,
    pub(super) methods: Vec<Method<'a>>,
    pub(super) protocols: Vec<Protocol<'a>>,
}

pub(super) fn build_assoc_fns<'m>(
    cx: &mut Ctxt<'_, 'm>,
    meta: Meta<'m>,
) -> Result<(
    Vec<Protocol<'m>>,
    Vec<Method<'m>>,
    Vec<Variant<'m>>,
    Vec<IndexEntry<'m>>,
    Vec<Trait<'m>>,
)> {
    let (variants, protocols, methods) = associated_for_hash(cx, meta.hash, meta, true)?;

    let mut index = Vec::new();

    if let Some(name) = cx.state.path.file_name() {
        index.try_reserve(methods.len())?;

        for m in &methods {
            index.try_push(IndexEntry {
                path: cx
                    .state
                    .path
                    .with_file_name(format!("{name}#method.{}", m.name)),
                item: Cow::Owned(meta.item.join([m.name])?),
                kind: IndexKind::Method,
                doc: m.line_doc.try_clone()?,
            })?;
        }

        for m in &variants {
            index.try_push(IndexEntry {
                path: cx
                    .state
                    .path
                    .with_file_name(format!("{name}#variant.{}", m.name)),
                item: Cow::Owned(meta.item.join([m.name])?),
                kind: IndexKind::Variant,
                doc: m.line_doc.try_clone()?,
            })?;
        }
    }

    let mut traits = Vec::new();

    'outer: for hash in cx.context.traits(meta.hash) {
        let item = 'item: {
            for meta in cx.context.meta_by_hash(hash)? {
                match meta.kind {
                    Kind::Trait => break 'item meta.item,
                    _ => continue,
                }
            }

            continue 'outer;
        };

        let (_, protocols, methods) = associated_for_hash(cx, hash, meta, false)?;

        let name = item
            .last()
            .and_then(|c| c.as_str())
            .context("Missing trait name")?;

        let module = cx.module_path_html(meta, false)?;
        let url = cx.item_path(item, ItemKind::Trait)?;

        traits.try_push(Trait {
            item,
            hash,
            module,
            name,
            url,
            methods,
            protocols,
        })?;
    }

    Ok((protocols, methods, variants, index, traits))
}

fn associated_for_hash<'m>(
    cx: &mut Ctxt<'_, 'm>,
    hash: Hash,
    meta: Meta<'m>,
    capture_tests: bool,
) -> Result<(Vec<Variant<'m>>, Vec<Protocol<'m>>, Vec<Method<'m>>)> {
    let mut variants = Vec::new();
    let mut protocols = Vec::new();
    let mut methods = Vec::new();

    for hash in cx.context.associated(hash) {
        for assoc in cx.context.associated_meta(hash) {
            match assoc {
                Assoc::Variant(variant) => {
                    let line_doc =
                        cx.render_line_docs(meta, variant.docs.get(..1).unwrap_or_default())?;

                    let doc = cx.render_docs(meta, variant.docs, capture_tests)?;

                    variants.try_push(Variant {
                        name: variant.name,
                        line_doc,
                        doc,
                    })?;
                }
                Assoc::Fn(assoc) => {
                    let value;

                    let (protocol, value, field) = match assoc.kind {
                        AssocFnKind::Protocol(protocol) => (protocol, "value", None),
                        AssocFnKind::FieldFn(protocol, field) => {
                            value = format!("value.{field}");
                            (protocol, value.as_str(), Some(field))
                        }
                        AssocFnKind::IndexFn(protocol, index) => {
                            value = format!("value.{index}");
                            (protocol, value.as_str(), None)
                        }
                        AssocFnKind::Method(item, name, sig) => {
                            // NB: Regular associated functions are documented by the trait itself.
                            if assoc.trait_hash.is_some() {
                                continue;
                            }

                            let line_doc =
                                cx.render_line_docs(meta, assoc.docs.get(..1).unwrap_or_default())?;

                            let old = replace(&mut cx.state.item, item);
                            let doc = cx.render_docs(meta, assoc.docs, capture_tests)?;
                            cx.state.item = old;

                            let parameters = if !assoc.parameter_types.is_empty() {
                                let mut s = String::new();
                                let mut it = assoc.parameter_types.iter().peekable();

                                while let Some(hash) = it.next() {
                                    cx.write_link(&mut s, *hash, None, &[])?;

                                    if it.peek().is_some() {
                                        write!(s, ", ")?;
                                    }
                                }

                                Some(s)
                            } else {
                                None
                            };

                            let method = Method {
                                is_async: assoc.is_async,
                                deprecated: assoc.deprecated,
                                name,
                                args: cx.args_to_string(sig, assoc.arguments)?,
                                parameters,
                                return_type: cx.return_type(assoc.return_type)?,
                                line_doc,
                                doc,
                            };

                            methods.try_push(method)?;
                            continue;
                        }
                    };

                    let kind = replace(&mut cx.state.kind, TestKind::Protocol(protocol));

                    let doc = if assoc.docs.is_empty() {
                        cx.render_docs(meta, protocol.doc, false)?
                    } else {
                        cx.render_docs(meta, assoc.docs, capture_tests)?
                    };

                    cx.state.kind = kind;

                    let repr = if let Some(repr) = protocol.repr {
                        Some(cx.render_code([repr.replace("$value", value.as_ref())])?)
                    } else {
                        None
                    };

                    let protocol = Protocol {
                        name: protocol.name,
                        field,
                        repr,
                        return_type: cx.return_type(assoc.return_type)?,
                        doc,
                        deprecated: assoc.deprecated,
                    };

                    protocols.try_push(protocol)?;
                }
            }
        }
    }
    Ok((variants, protocols, methods))
}

#[derive(Serialize)]
struct Params<'a> {
    #[serde(flatten)]
    shared: super::Shared<'a>,
    what: &'a str,
    what_class: &'a str,
    module: String,
    #[serde(serialize_with = "super::serialize_component_ref")]
    name: ComponentRef<'a>,
    #[serde(serialize_with = "super::serialize_item")]
    item: &'a Item,
    methods: Vec<Method<'a>>,
    protocols: Vec<Protocol<'a>>,
    traits: Vec<Trait<'a>>,
    doc: Option<String>,
}

/// Build an unknown type.
#[tracing::instrument(skip_all)]
pub(crate) fn build<'m>(
    cx: &mut Ctxt<'_, 'm>,
    what: &'static str,
    what_class: &'static str,
    meta: Meta<'m>,
) -> Result<(Builder<'m>, Vec<IndexEntry<'m>>)> {
    let module = cx.module_path_html(meta, false)?;

    let (protocols, methods, _, index, traits) = build_assoc_fns(cx, meta)?;
    let name = meta.item.last().context("Missing module name")?;

    let doc = cx.render_docs(meta, meta.docs, true)?;

    let builder = Builder::new(cx, move |cx| {
        cx.type_template.render(&Params {
            shared: cx.shared()?,
            what,
            what_class,
            module,
            name,
            item: meta.item,
            methods,
            protocols,
            traits,
            doc,
        })
    })?;

    Ok((builder, index))
}
