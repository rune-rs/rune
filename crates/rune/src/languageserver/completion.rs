use std::borrow::ToOwned;

use anyhow::Result;
use lsp::CompletionItem;
use lsp::CompletionItemKind;
use lsp::CompletionItemLabelDetails;
use lsp::CompletionTextEdit;
use lsp::Documentation;
use lsp::MarkupContent;
use lsp::MarkupKind;
use lsp::TextEdit;

use crate::alloc::fmt::TryWrite;
use crate::alloc::prelude::*;
use crate::alloc::{String, Vec};
use crate::compile::meta;
use crate::doc::Visitor;
use crate::runtime::debug::DebugArgs;
use crate::{Context, Hash, Unit};

use super::state::ServerSource;

pub(super) fn complete_for_unit(
    workspace_source: &ServerSource,
    unit: &Unit,
    symbol: &str,
    start: lsp::Position,
    position: lsp::Position,
    results: &mut Vec<CompletionItem>,
) -> Result<()> {
    let Some(debug_info) = unit.debug_info() else {
        return Ok(());
    };

    for (hash, function) in debug_info.functions.iter() {
        let func_name = function.try_to_string()?;

        if !func_name.starts_with(symbol) {
            continue;
        }

        let Some(last) = function.path.base_name() else {
            continue;
        };

        let args = match &function.args {
            DebugArgs::EmptyArgs => None,
            DebugArgs::TupleArgs(n) => Some({
                let mut o = String::new();

                let mut it = 0..*n;
                let last = it.next_back();

                for n in it {
                    write!(o, "_{n}, ")?;
                }

                if let Some(n) = last {
                    write!(o, "_{n}")?;
                }

                o
            }),
            DebugArgs::Named(names) => Some(names.iter().map(|s| s.as_ref()).try_join(", ")?),
        };

        let docs = workspace_source
            .get_docs_by_hash(*hash)
            .map(|docs| docs.docs.join("\n"));

        let detail = args.map(|a| format!("({a:}) -> ?"));

        results.try_push(CompletionItem {
            label: last.to_owned(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: detail.clone(),
            documentation: docs.map(|d| {
                Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: d,
                })
            }),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                range: lsp::Range {
                    start,
                    end: position,
                },
                new_text: format!("{}", function.path),
            })),
            label_details: Some(CompletionItemLabelDetails {
                detail,
                description: None,
            }),
            commit_characters: Some(vec!["(".into()]),
            ..Default::default()
        })?;
    }

    Ok(())
}

pub(super) fn complete_native_instance_data(
    context: &Context,
    symbol: &str,
    start: lsp::Position,
    position: lsp::Position,
    results: &mut Vec<CompletionItem>,
) -> Result<()> {
    for (meta, signature) in context.iter_functions() {
        let (prefix, kind, n) = match (&meta.item, &meta.kind) {
            (
                Some(item),
                meta::Kind::Function {
                    associated: Some(meta::AssociatedKind::Instance(name)),
                    ..
                },
            ) => (item, CompletionItemKind::FUNCTION, name),
            _ => continue,
        };

        if n.starts_with(symbol) {
            let return_type = signature
                .return_type
                .base
                .as_non_empty()
                .and_then(|hash| context.lookup_meta_by_hash(hash.get()).next())
                .and_then(|r| r.item.as_deref());

            let docs = meta.docs.lines().join("\n");
            let args = meta.docs.args().join(", ");

            let detail = return_type.map(|r| format!("({args}) -> {r}"));

            let Ok(data) = serde_json::to_value(meta.hash.into_inner()) else {
                continue;
            };

            results.try_push(CompletionItem {
                label: n.try_to_string()?.into_std(),
                kind: Some(kind),
                detail,
                documentation: Some(lsp::Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: docs,
                })),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                    range: lsp::Range {
                        start,
                        end: position,
                    },
                    new_text: n.try_to_string()?.into_std(),
                })),
                label_details: Some(CompletionItemLabelDetails {
                    detail: None,
                    description: Some(prefix.try_to_string()?.into_std()),
                }),
                data: Some(data),
                ..Default::default()
            })?;
        }
    }

    Ok(())
}

pub(super) fn complete_native_loose_data(
    context: &Context,
    symbol: &str,
    start: lsp::Position,
    position: lsp::Position,
    results: &mut Vec<CompletionItem>,
) -> Result<()> {
    for (meta, signature) in context.iter_functions() {
        let (item, kind) = match (&meta.item, &meta.kind) {
            (Some(item), meta::Kind::Function { .. }) => (item, CompletionItemKind::FUNCTION),
            _ => continue,
        };

        let func_name = item
            .try_to_string()?
            .trim_start_matches("::")
            .try_to_owned()?;

        if func_name.starts_with(symbol) {
            let return_type = signature
                .return_type
                .base
                .as_non_empty()
                .and_then(|hash| context.lookup_meta_by_hash(hash.get()).next())
                .and_then(|r| r.item.as_deref());

            let docs = meta.docs.lines().join("\n");
            let args = meta.docs.args().join(", ");

            let detail = return_type.map(|r| format!("({args}) -> {r}"));

            let Ok(data) = serde_json::to_value(meta.hash.into_inner()) else {
                continue;
            };

            results.try_push(CompletionItem {
                label: func_name.try_clone()?.into_std(),
                kind: Some(kind),
                detail,
                documentation: Some(lsp::Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: docs,
                })),
                text_edit: Some(lsp::CompletionTextEdit::Edit(TextEdit {
                    range: lsp::Range {
                        start,
                        end: position,
                    },
                    new_text: func_name.into_std(),
                })),
                data: Some(data),
                ..Default::default()
            })?;
        }
    }

    Ok(())
}

/// What the compiled sources of a build declare or define which the unit does
/// not hold the code of - the items of a declaration source, and everything
/// when the unit failed to build, which it does whenever a name is half typed.
pub(super) fn complete_for_docs(
    docs: &Visitor,
    unit: Option<&Unit>,
    context: &Context,
    symbol: &str,
    start: lsp::Position,
    position: lsp::Position,
    results: &mut Vec<CompletionItem>,
) -> Result<()> {
    let functions = unit
        .and_then(|unit| unit.debug_info())
        .map(|debug| &debug.functions);

    for data in docs.data.values() {
        let (kind, detail) = match &data.kind {
            Some(meta::Kind::Function {
                associated: None,
                signature,
                ..
            }) => (
                CompletionItemKind::FUNCTION,
                Some(signature_detail(docs, context, signature)?),
            ),
            Some(meta::Kind::Type { .. }) => (CompletionItemKind::STRUCT, None),
            Some(meta::Kind::Enum { .. }) => (CompletionItemKind::ENUM, None),
            _ => continue,
        };

        if functions.is_some_and(|f| f.contains_key(&data.hash)) {
            continue;
        }

        let path = data.item.try_to_string()?;

        if !path.starts_with(symbol) {
            continue;
        }

        let Some(last) = data.item.base_name() else {
            continue;
        };

        let documentation = doc_lines(&data.docs)?.map(|value| {
            Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: value.into_std(),
            })
        });

        let detail = detail.map(String::into_std);

        results.try_push(CompletionItem {
            label: last.to_owned(),
            kind: Some(kind),
            detail: detail.clone(),
            documentation,
            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                range: lsp::Range {
                    start,
                    end: position,
                },
                new_text: path.into_std(),
            })),
            label_details: Some(CompletionItemLabelDetails {
                detail,
                description: None,
            }),
            ..Default::default()
        })?;
    }

    Ok(())
}

/// The arguments and return type of a signature as `(value: u64) -> u64`,
/// leaving out the types nothing named.
pub(super) fn signature_detail(
    docs: &Visitor,
    context: &Context,
    signature: &meta::Signature,
) -> Result<String> {
    let mut o = String::new();
    o.try_push('(')?;

    for (n, arg) in signature
        .arguments
        .iter()
        .flat_map(|a| a.iter())
        .enumerate()
    {
        if n > 0 {
            o.try_push_str(", ")?;
        }

        match &arg.name {
            meta::DocName::Name(name) => o.try_push_str(name.as_ref())?,
            meta::DocName::Index(n) => write!(o, "_{n}")?,
        }

        if let Some(ty) = type_name(docs, context, arg.base) {
            write!(o, ": {ty}")?;
        }
    }

    o.try_push(')')?;

    if let Some(ty) = type_name(docs, context, signature.return_type.base) {
        write!(o, " -> {ty}")?;
    }

    Ok(o)
}

/// The name of the type with the given hash, declared beside what names it
/// or native.
fn type_name<'a>(docs: &'a Visitor, context: &'a Context, hash: Hash) -> Option<&'a str> {
    if hash == Hash::EMPTY {
        return None;
    }

    let item = match docs.get_by_hash(hash) {
        Some(data) => &data.item,
        None => context.lookup_meta_by_hash(hash).next()?.item.as_ref()?,
    };

    item.base_name()
}

/// Documentation comments as the markdown they are, without the space a
/// comment puts after its `///`. None if there are none.
pub(super) fn doc_lines(docs: &[String]) -> Result<Option<String>> {
    if docs.is_empty() {
        return Ok(None);
    }

    let text = docs
        .iter()
        .map(|d| d.strip_prefix(' ').unwrap_or(d))
        .try_join("\n")?;

    Ok(Some(text))
}
