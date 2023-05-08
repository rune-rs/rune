use crate::no_std::prelude::*;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::compile::{ComponentRef, Item};
use crate::doc::context::{Assoc, AssocFnKind, Signature, Meta};
use crate::doc::html::{Ctxt, IndexEntry, IndexKind, Builder};

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
    name: &'a str,
    args: String,
    parameters: Option<String>,
    return_type: Option<String>,
    line_doc: Option<String>,
    doc: Option<String>,
}

pub(super) fn build_assoc_fns<'m>(
    cx: &Ctxt<'_, 'm>,
    meta: Meta<'m>,
) -> Result<(Vec<Protocol<'m>>, Vec<Method<'m>>, Vec<IndexEntry>)> {
    let mut protocols = Vec::new();
    let mut methods = Vec::new();

    for assoc in cx.context.associated(meta.hash) {
        let Assoc::Fn(assoc) = assoc else {
            continue;
        };

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
            AssocFnKind::Instance(name) => {
                let line_doc = cx.render_docs(assoc.docs.get(..1).unwrap_or_default())?;
                let doc = cx.render_docs(assoc.docs)?;

                let mut list = Vec::new();

                for &hash in assoc.parameter_types {
                    list.push(cx.link(hash, None)?);
                }

                let parameters = (!list.is_empty()).then(|| list.join(", "));

                methods.push(Method {
                    is_async: assoc.is_async,
                    name,
                    args: cx.args_to_string(
                        assoc.docs_args,
                        Signature::Instance { args: assoc.args },
                        &assoc.argument_types,
                    )?,
                    parameters,
                    return_type: match assoc.return_type {
                        Some(hash) => Some(cx.link(hash, None)?),
                        None => None,
                    },
                    line_doc,
                    doc,
                });

                continue;
            }
        };

        let doc = if assoc.docs.is_empty() {
            cx.render_docs(protocol.doc)?
        } else {
            cx.render_docs(assoc.docs)?
        };

        let repr = if let Some(repr) = protocol.repr {
            Some(cx.render_code([repr.replace("$value", value.as_ref())])?)
        } else {
            None
        };

        protocols.push(Protocol {
            name: protocol.name,
            repr,
            return_type: match assoc.return_type {
                Some(hash) => Some(cx.link(hash, None)?),
                None => None,
            },
            doc,
        });
    }

    let mut index = Vec::new();

    if let Some(name) = cx.path.file_name() {
        index.reserve(methods.len());

        for m in &methods {
            index.push(IndexEntry {
                path: cx.path.with_file_name(format!("{name}#method.{}", m.name)),
                item: cx.item.join([m.name]),
                kind: IndexKind::Method,
                doc: m.line_doc.clone(),
            });
        }
    }

    Ok((protocols, methods, index))
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
}

/// Build an unknown type.
#[tracing::instrument(skip_all)]
pub(super) fn build<'m>(cx: &Ctxt<'_, 'm>, what: &'static str, what_class: &'static str, meta: Meta<'m>) -> Result<(Builder<'m>, Vec<IndexEntry>)> {
    let module = cx.module_path_html(false)?;

    let (protocols, methods, index) = build_assoc_fns(cx, meta)?;

    let builder = Builder::new(cx, move |cx| {
        let name = cx.item.last().context("missing module name")?;

        cx.type_template.render(&Params {
            shared: cx.shared(),
            what,
            what_class,
            module,
            name,
            item: &cx.item,
            methods,
            protocols,
        })
    });

    Ok((builder, index))
}
