use crate::no_std::prelude::*;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::compile::{ComponentRef, Item};
use crate::hash::Hash;

use super::Ctxt;

/// Build an enumeration.
#[tracing::instrument(skip_all)]
pub(super) fn build(cx: &Ctxt<'_>, hash: Hash) -> Result<()> {
    #[derive(Serialize)]
    struct Params<'a> {
        #[serde(flatten)]
        shared: super::Shared,
        module: String,
        #[serde(serialize_with = "super::serialize_component_ref")]
        name: ComponentRef<'a>,
        #[serde(serialize_with = "super::serialize_item")]
        item: &'a Item,
        methods: Vec<super::type_::Method<'a>>,
        protocols: Vec<super::type_::Protocol<'a>>,
    }

    let module = cx.module_path_html(false);
    let name = cx.item.last().context("missing module name")?;

    let (protocols, methods) = super::type_::build_assoc_fns(cx, hash)?;

    cx.write_file(|cx| {
        cx.enum_template.render(&Params {
            shared: cx.shared(),
            module,
            name,
            item: &cx.item,
            methods,
            protocols,
        })
    })
}
