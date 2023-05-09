use crate::no_std::prelude::*;

use lsp::CompletionItem;
use lsp::CompletionItemKind;
use lsp::CompletionItemLabelDetails;
use lsp::CompletionTextEdit;
use lsp::Documentation;
use lsp::MarkupContent;
use lsp::MarkupKind;
use lsp::TextEdit;

use crate::compile::meta::SignatureKind;
use crate::module::AssociatedKind;
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
) {
    let Some(debug_info) = unit.debug_info() else {
		return;
	};

    for (hash, function) in debug_info.functions.iter() {
        let func_name = function.to_string();
        if !func_name.starts_with(symbol) {
            continue;
        }

        let args = match &function.args {
            DebugArgs::EmptyArgs => None,
            DebugArgs::TupleArgs(n) => Some(
                (0..*n)
                    .map(|n| format!("_{}", n))
                    .fold("".to_owned(), |a, b| format!("{}, {}", a, b)),
            ),
            DebugArgs::Named(names) => Some(names.join(", ")),
        };

        let docs = workspace_source
            .get_docs_by_hash(*hash)
            .map(|docs| docs.docs.join("\n"));

        let detail = args.map(|a| format!("({a:}) -> ?"));
        results.push(CompletionItem {
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
        })
    }
}

pub(super) fn complete_native_instance_data(
    context: &Context,
    symbol: &str,
    position: lsp::Position,
    results: &mut Vec<CompletionItem>,
) {
    for info in context.iter_functions() {
        let (prefix, kind, function_kind) = match &info.1.kind {
            SignatureKind::Instance { name, .. } => {
                (info.1.item.clone(), CompletionItemKind::FUNCTION, name)
            }
            SignatureKind::Function { .. } => continue,
        };

        let n = match function_kind {
            AssociatedKind::Protocol(_) => continue,
            AssociatedKind::FieldFn(_, _) => continue,
            AssociatedKind::IndexFn(_, _) => continue,
            AssociatedKind::Instance(n) => n,
        };

        if n.starts_with(symbol) {
            let meta = context.lookup_meta_by_hash(info.0).next();

            let return_type = info
                .1
                .return_type
                .and_then(|hash| context.lookup_meta_by_hash(hash).next())
                .and_then(|r| r.item.as_deref());

            let docs = meta.map(|meta| meta.docs.lines().join("\n"));
            let args = meta
                .map(|meta| &meta.docs)
                .and_then(|d| d.args())
                .map(|args| args.join(", "));
            let detail = return_type
                .zip(args.clone())
                .map(|(r, a)| format!("({a:} -> {r}"));

            results.push(CompletionItem {
                label: n.to_string(),
                kind: Some(kind),
                detail,
                documentation: docs.map(|d| {
                    lsp::Documentation::MarkupContent(MarkupContent {
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
                    new_text: n.to_string(),
                })),
                label_details: Some(CompletionItemLabelDetails {
                    detail: None,
                    description: Some(prefix.to_string()),
                }),
                data: Some(serde_json::to_value(info.0).unwrap()),
                ..Default::default()
            })
        }
    }
}

pub(super) fn complete_native_loose_data(
    context: &Context,
    symbol: &str,
    position: lsp::Position,
    results: &mut Vec<CompletionItem>,
) {
    for info in context.iter_functions() {
        let (item, kind) = match info.1.kind {
            SignatureKind::Function { .. } => (info.1.item.clone(), CompletionItemKind::FUNCTION),
            SignatureKind::Instance { .. } => continue,
        };

        let func_name = item.to_string().trim_start_matches("::").to_owned();
        if func_name.starts_with(symbol) {
            let meta = context.lookup_meta_by_hash(info.0).next();

            let return_type = info
                .1
                .return_type
                .and_then(|hash| context.lookup_meta_by_hash(hash).next())
                .and_then(|r| r.item.as_deref());

            let docs = meta.map(|meta| meta.docs.lines().join("\n"));
            let args = meta
                .map(|meta| &meta.docs)
                .and_then(|d| d.args())
                .map(|args| args.join(", "));
            let detail = return_type
                .zip(args.clone())
                .map(|(r, a)| format!("({a:}) -> {r}"));

            results.push(CompletionItem {
                label: func_name.clone(),
                kind: Some(kind),
                detail,
                documentation: docs.map(|d| {
                    lsp::Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: d,
                    })
                }),
                text_edit: Some(lsp::CompletionTextEdit::Edit(TextEdit {
                    range: lsp::Range {
                        start: lsp::Position {
                            line: position.line,
                            character: position.character - symbol.len() as u32,
                        },
                        end: position,
                    },
                    new_text: func_name,
                })),
                data: Some(serde_json::to_value(info.0).unwrap()),
                ..Default::default()
            })
        }
    }
}
