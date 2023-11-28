use anyhow::{Context, Result};
use serde::Serialize;

use crate::alloc::{String, Vec};
use crate::compile::{ComponentRef, Item};
use crate::doc::build::{self, Builder, Ctxt, IndexEntry};
use crate::doc::context::Meta;

#[derive(Serialize)]
struct Params<'a> {
    #[serde(flatten)]
    shared: build::Shared<'a>,
    module: String,
    #[serde(serialize_with = "super::serialize_component_ref")]
    name: ComponentRef<'a>,
    #[serde(serialize_with = "super::serialize_item")]
    item: &'a Item,
    variants: Vec<build::type_::Variant<'a>>,
    methods: Vec<build::type_::Method<'a>>,
    protocols: Vec<build::type_::Protocol<'a>>,
    doc: Option<String>,
}

/// Build an enumeration.
#[tracing::instrument(skip_all)]
pub(crate) fn build<'m>(
    cx: &mut Ctxt<'_, 'm>,
    meta: Meta<'m>,
) -> Result<(Builder<'m>, Vec<IndexEntry<'m>>)> {
    let module = cx.module_path_html(meta, false)?;

    let (protocols, methods, variants, index) = build::type_::build_assoc_fns(cx, meta)?;
    let item = meta.item.context("Missing enum item")?;
    let name = item.last().context("Missing enum name")?;

    let doc = cx.render_docs(meta, meta.docs, true)?;

    let builder = Builder::new(cx, move |cx| {
        cx.enum_template.render(&Params {
            shared: cx.shared()?,
            module,
            name,
            item,
            variants,
            methods,
            protocols,
            doc,
        })
    })?;

    Ok((builder, index))
}
