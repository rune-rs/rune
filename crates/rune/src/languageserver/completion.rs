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
use crate::runtime::debug::DebugArgs;
use crate::Context;
use crate::Unit;

use super::state::Source;

pub(super) fn complete_for_unit(
    workspace_source: &Source,
    unit: &Unit,
    symbol: &str,
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
            label: format!("{}", function.path.last().unwrap()),
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
                    start: lsp::Position {
                        line: position.line,
                        character: position.character - symbol.len() as u32,
                    },
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
                .and_then(|hash| context.lookup_meta_by_hash(hash).next())
                .and_then(|r| r.item.as_deref());

            let docs = meta.docs.lines().join("\n");
            let args = meta.docs.args().unwrap_or_default().join(", ");

            let detail = return_type.map(|r| format!("({args} -> {r}"));

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
                        start: lsp::Position {
                            line: position.line,
                            character: position.character - symbol.len() as u32,
                        },
                        end: position,
                    },
                    new_text: n.try_to_string()?.into_std(),
                })),
                label_details: Some(CompletionItemLabelDetails {
                    detail: None,
                    description: Some(prefix.try_to_string()?.into_std()),
                }),
                data: Some(serde_json::to_value(meta.hash).unwrap()),
                ..Default::default()
            })?;
        }
    }

    Ok(())
}

pub(super) fn complete_native_loose_data(
    context: &Context,
    symbol: &str,
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
                .and_then(|hash| context.lookup_meta_by_hash(hash).next())
                .and_then(|r| r.item.as_deref());

            let docs = meta.docs.lines().join("\n");
            let args = meta.docs.args().unwrap_or_default().join(", ");

            let detail = return_type.map(|r| format!("({args}) -> {r}"));

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
                        start: lsp::Position {
                            line: position.line,
                            character: position.character - symbol.len() as u32,
                        },
                        end: position,
                    },
                    new_text: func_name.into_std(),
                })),
                data: Some(serde_json::to_value(meta.hash).unwrap()),
                ..Default::default()
            })?;
        }
    }

    Ok(())
}
