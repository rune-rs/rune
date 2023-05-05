use crate::no_std::prelude::*;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::compile::{ComponentRef, Item};
use crate::doc::context::{AssociatedKind, Kind, Signature};
use crate::hash::Hash;

use super::Ctxt;

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
    doc: Option<String>,
}

pub(super) fn build_assoc<'a>(
    cx: &'a Ctxt<'a>,
    hash: Hash,
) -> Result<(Vec<Protocol<'a>>, Vec<Method<'a>>)> {
    let mut protocols = Vec::new();
    let mut methods = Vec::new();

    for name in cx.context.iter_components(&cx.item) {
        let item = cx.item.join([name]);

        let meta = match cx.context.meta(&item) {
            Some(meta) => meta,
            _ => continue,
        };

        let name = match name {
            ComponentRef::Str(name) => name,
            _ => continue,
        };

        match meta.kind {
            Kind::Function(f) => {
                if !matches!(f.signature, Signature::Instance { .. }) {
                    methods.push(Method {
                        is_async: f.is_async,
                        name,
                        args: super::args_to_string(f.args, f.signature)?,
                        parameters: None,
                        return_type: match f.return_type {
                            Some(hash) => Some(cx.hash_to_link(hash)?),
                            None => None,
                        },
                        doc: cx.render_docs(meta.docs)?,
                    });
                }
            }
            _ => {
                continue;
            }
        }
    }

    for assoc in cx.context.associated(hash) {
        let value;

        let (protocol, value) = match assoc.kind {
            AssociatedKind::Protocol(protocol) => (protocol, "value"),
            AssociatedKind::FieldFn(protocol, field) => {
                value = format!("value.{field}");
                (protocol, value.as_str())
            }
            AssociatedKind::IndexFn(protocol, index) => {
                value = format!("value.{index}");
                (protocol, value.as_str())
            }
            AssociatedKind::Instance(name) => {
                let doc = cx.render_docs(assoc.docs)?;

                let mut list = Vec::new();

                for hash in assoc.parameter_types {
                    list.push(cx.hash_to_link(*hash)?);
                }

                let parameters = (!list.is_empty()).then(|| list.join(", "));

                methods.push(Method {
                    is_async: assoc.is_async,
                    name,
                    args: super::args_to_string(
                        assoc.docs_args,
                        Signature::Instance { args: assoc.args },
                    )?,
                    parameters,
                    return_type: match assoc.return_type {
                        Some(hash) => Some(cx.hash_to_link(hash)?),
                        None => None,
                    },
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

        let repr = protocol
            .repr
            .map(|line| cx.render_code([line.replace("$value", value.as_ref())]));

        protocols.push(Protocol {
            name: protocol.name,
            repr,
            return_type: match assoc.return_type {
                Some(hash) => Some(cx.hash_to_link(hash)?),
                None => None,
            },
            doc,
        });
    }

    Ok((protocols, methods))
}

#[derive(Serialize)]
struct Params<'a> {
    #[serde(flatten)]
    shared: super::Shared,
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
pub(super) fn build(cx: &Ctxt<'_>, what: &str, what_class: &str, hash: Hash) -> Result<()> {
    let module = cx.module_path_html(false);
    let name = cx.item.last().context("missing module name")?;

    let (protocols, methods) = build_assoc(cx, hash)?;

    cx.write_file(|cx| {
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
    })
}
