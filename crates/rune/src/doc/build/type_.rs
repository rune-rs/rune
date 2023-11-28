use anyhow::{Context, Result};
use serde::Serialize;

use crate::alloc::borrow::Cow;
use crate::alloc::prelude::*;
use crate::alloc::{String, Vec};
use crate::compile::{ComponentRef, Item};
use crate::doc::build::{Builder, Ctxt, IndexEntry, IndexKind};
use crate::doc::context::{Assoc, AssocFnKind, Meta};

#[derive(Serialize)]
pub(super) struct Protocol<'a> {
    name: &'a str,
    repr: Option<String>,
    return_type: Option<String>,
    doc: Option<String>,
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

pub(super) fn build_assoc_fns<'m>(
    cx: &mut Ctxt<'_, 'm>,
    meta: Meta<'m>,
) -> Result<(
    Vec<Protocol<'m>>,
    Vec<Method<'m>>,
    Vec<Variant<'m>>,
    Vec<IndexEntry<'m>>,
)> {
    let mut protocols = Vec::new();
    let mut methods = Vec::new();
    let mut variants = Vec::new();

    let meta_item = meta.item.context("Missing meta item")?;

    for assoc in cx.context.associated(meta.hash) {
        match assoc {
            Assoc::Variant(variant) => {
                let line_doc =
                    cx.render_line_docs(meta, variant.docs.get(..1).unwrap_or_default())?;
                let doc = cx.render_docs(meta, variant.docs, true)?;

                variants.try_push(Variant {
                    name: variant.name,
                    line_doc,
                    doc,
                })?;
            }
            Assoc::Fn(assoc) => {
                let value;

                let (protocol, value) = match assoc.kind {
                    AssocFnKind::Protocol(protocol) => (protocol, "value"),
                    AssocFnKind::FieldFn(protocol, field) => {
                        value = format!("value.{field}");
                        (protocol, value.as_str())
                    }
                    AssocFnKind::IndexFn(protocol, index) => {
                        value = format!("value.{index}");
                        (protocol, value.as_str())
                    }
                    AssocFnKind::Method(name, args, sig) => {
                        let line_doc =
                            cx.render_line_docs(meta, assoc.docs.get(..1).unwrap_or_default())?;

                        cx.state.item.push(name)?;
                        let doc = cx.render_docs(meta, assoc.docs, true)?;
                        cx.state.item.pop()?;

                        let mut list = Vec::new();

                        for &hash in assoc.parameter_types {
                            if let Some(link) = cx.link(hash, None)? {
                                list.try_push(link)?;
                            } else {
                                list.try_push(hash.try_to_string()?)?;
                            }
                        }

                        let parameters = (!list.is_empty())
                            .then(|| list.iter().try_join(", "))
                            .transpose()?;

                        methods.try_push(Method {
                            is_async: assoc.is_async,
                            deprecated: assoc.deprecated,
                            name,
                            args: cx.args_to_string(
                                assoc.arg_names,
                                args,
                                sig,
                                assoc.argument_types,
                            )?,
                            parameters,
                            return_type: match assoc.return_type {
                                Some(hash) => cx.link(hash, None)?,
                                None => None,
                            },
                            line_doc,
                            doc,
                        })?;

                        continue;
                    }
                };

                let doc = if assoc.docs.is_empty() {
                    cx.render_docs(meta, protocol.doc, false)?
                } else {
                    cx.render_docs(meta, assoc.docs, true)?
                };

                let repr = if let Some(repr) = protocol.repr {
                    Some(cx.render_code([repr.replace("$value", value.as_ref())])?)
                } else {
                    None
                };

                protocols.try_push(Protocol {
                    name: protocol.name,
                    repr,
                    return_type: match assoc.return_type {
                        Some(hash) => cx.link(hash, None)?,
                        None => None,
                    },
                    doc,
                })?;
            }
        }
    }

    let mut index = Vec::new();

    if let Some(name) = cx.state.path.file_name() {
        index.try_reserve(methods.len())?;

        for m in &methods {
            index.try_push(IndexEntry {
                path: cx
                    .state
                    .path
                    .with_file_name(format!("{name}#method.{}", m.name)),
                item: Cow::Owned(meta_item.join([m.name])?),
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
                item: Cow::Owned(meta_item.join([m.name])?),
                kind: IndexKind::Variant,
                doc: m.line_doc.try_clone()?,
            })?;
        }
    }

    Ok((protocols, methods, variants, index))
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

    let (protocols, methods, _, index) = build_assoc_fns(cx, meta)?;
    let item = meta.item.context("Missing type item")?;
    let name = item.last().context("Missing module name")?;

    let doc = cx.render_docs(meta, meta.docs, true)?;

    let builder = Builder::new(cx, move |cx| {
        cx.type_template.render(&Params {
            shared: cx.shared()?,
            what,
            what_class,
            module,
            name,
            item,
            methods,
            protocols,
            doc,
        })
    })?;

    Ok((builder, index))
}
