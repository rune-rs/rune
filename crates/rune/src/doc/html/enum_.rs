use crate::no_std::prelude::*;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::compile::{ComponentRef, Item};
use crate::doc::context::Meta;
use crate::doc::html::{Builder, Ctxt, IndexEntry};

/// Build an enumeration.
#[tracing::instrument(skip_all)]
pub(super) fn build<'m>(cx: &Ctxt<'_, 'm>, meta: Meta<'m>) -> Result<(Builder<'m>, Vec<IndexEntry<'m>>)> {
    #[derive(Serialize)]
    struct Params<'a> {
        #[serde(flatten)]
        shared: super::Shared<'a>,
        module: String,
        #[serde(serialize_with = "super::serialize_component_ref")]
        name: ComponentRef<'a>,
        #[serde(serialize_with = "super::serialize_item")]
        item: &'a Item,
        methods: Vec<super::type_::Method<'a>>,
        protocols: Vec<super::type_::Protocol<'a>>,
    }

    let module = cx.module_path_html(meta, false)?;

    let (protocols, methods, index) = super::type_::build_assoc_fns(cx, meta)?;
    let name = meta.item.last().context("Missing enum name")?;

    let builder = Builder::new(cx, move |cx| {
        cx.enum_template.render(&Params {
            shared: cx.shared(),
            module,
            name,
            item: meta.item,
            methods,
            protocols,
        })
    });

    Ok((builder, index))
}
