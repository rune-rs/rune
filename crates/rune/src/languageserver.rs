//! Utility for building a language server.

#![allow(clippy::too_many_arguments)]

mod completion;
mod connection;
pub mod envelope;
mod fs;
mod state;
mod url;

use lsp::notification::Notification;
use lsp::request::Request;
use serde::Deserialize;
use tokio::sync::Notify;

use crate::alloc::String;
use crate::languageserver::connection::stdio;
use crate::languageserver::envelope::Code;
use crate::languageserver::state::State;
use crate::support::Result;
use crate::workspace::MANIFEST_FILE;
use crate::{Context, Options};

enum Language {
    Rune,
    Other,
}

/// Run a language server with the given options.
pub async fn run(context: Context, options: Options) -> Result<()> {
    let (mut input, output) = stdio()?;

    let rebuild_notify = Notify::new();

    let rebuild = rebuild_notify.notified();
    tokio::pin!(rebuild);

    let mut state = State::new(output, &rebuild_notify, context, options);
    tracing::info!("Starting server");
    state.rebuild().await?;

    while !state.is_stopped() {
        tokio::select! {
            _ = rebuild.as_mut() => {
                tracing::info!("rebuilding project");
                state.rebuild().await?;
                rebuild.set(rebuild_notify.notified());
            },
            frame = input.next() => {
                let frame = match frame? {
                    Some(frame) => frame,
                    None => break,
                };

                let incoming: envelope::IncomingMessage = serde_json::from_slice(frame.content)?;
                tracing::trace!(?incoming);

                // If server is not initialized, reject incoming requests.
                if !state.is_initialized() && incoming.method != lsp::request::Initialize::METHOD {
                    state.output
                        .error(
                            incoming.id,
                            Code::InvalidRequest,
                            "Server not initialized",
                            None::<()>,
                        )
                        .await?;

                    continue;
                }

                macro_rules! handle {
                    ($(req($req_ty:ty, $req_handle:ident)),* $(, notif($notif_ty:ty, $notif_handle:ident))* $(,)?) => {
                        match incoming.method.as_str() {
                            $(<$req_ty>::METHOD => {
                                let params = <$req_ty as Request>::Params::deserialize(incoming.params)?;
                                let result = $req_handle(&mut state, params).await?;
                                state.output.response(incoming.id, result).await?;
                            })*
                            $(<$notif_ty>::METHOD => {
                                let params = <$notif_ty as Notification>::Params::deserialize(incoming.params)?;
                                let () = $notif_handle(&mut state, params).await?;
                            })*
                            _ => {
                                state.output
                                .log(
                                    lsp::MessageType::INFO,
                                    format!("Unhandled method `{}`", incoming.method),
                                )
                                .await?;
                                state.output.method_not_found(incoming.id).await?;
                            }
                        }
                    }
                }

                handle! {
                    req(lsp::request::Initialize, initialize),
                    req(lsp::request::Shutdown, shutdown),
                    req(lsp::request::GotoDefinition, goto_definition),
                    req(lsp::request::Completion, completion),
                    req(lsp::request::Formatting, formatting),
                    notif(lsp::notification::DidOpenTextDocument, did_open_text_document),
                    notif(lsp::notification::DidChangeTextDocument, did_change_text_document),
                    notif(lsp::notification::DidCloseTextDocument, did_close_text_document),
                    notif(lsp::notification::DidSaveTextDocument, did_save_text_document),
                    notif(lsp::notification::Initialized, initialized),
                }
            },
        }
    }

    Ok(())
}

/// Initialize the language state.
async fn initialize(
    s: &mut State<'_>,
    params: lsp::InitializeParams,
) -> Result<lsp::InitializeResult> {
    s.initialize();

    s.output
        .log(lsp::MessageType::INFO, "Starting language server")
        .await?;

    let capabilities = lsp::ServerCapabilities {
        text_document_sync: Some(lsp::TextDocumentSyncCapability::Kind(
            lsp::TextDocumentSyncKind::INCREMENTAL,
        )),
        definition_provider: Some(lsp::OneOf::Left(true)),
        completion_provider: Some(lsp::CompletionOptions {
            all_commit_characters: None,
            resolve_provider: Some(false),
            trigger_characters: Some(vec![".".into(), "::".into()]),
            work_done_progress_options: lsp::WorkDoneProgressOptions {
                work_done_progress: None,
            },
            completion_item: Some(lsp::CompletionOptionsCompletionItem {
                label_details_support: Some(true),
            }),
        }),
        document_formatting_provider: Some(lsp::OneOf::Left(true)),
        ..Default::default()
    };

    let server_info = lsp::ServerInfo {
        name: String::try_from("Rune Language Server")?.into_std(),
        version: None,
    };

    let mut rebuild = false;

    if let Some(root_uri) = &params.root_uri {
        let mut manifest_uri = root_uri.clone();

        if let Ok(mut path) = manifest_uri.path_segments_mut() {
            path.push(MANIFEST_FILE);
        }

        if let Ok(manifest_path) = manifest_uri.to_file_path() {
            if fs::is_file(&manifest_path).await? {
                tracing::trace!(?manifest_uri, ?manifest_path, "Activating workspace");
                s.workspace_mut().manifest_path = Some((manifest_uri, manifest_path));
                rebuild = true;
            }
        }
    }

    if rebuild {
        s.rebuild_interest();
    }

    Ok(lsp::InitializeResult {
        capabilities,
        server_info: Some(server_info),
    })
}

async fn shutdown(s: &mut State<'_>, _: ()) -> Result<()> {
    s.stop();
    Ok(())
}

/// Handle initialized notification.
async fn initialized(_: &mut State<'_>, _: lsp::InitializedParams) -> Result<()> {
    tracing::info!("Initialized");
    Ok(())
}

/// Handle initialized notification.
async fn goto_definition(
    s: &mut State<'_>,
    params: lsp::GotoDefinitionParams,
) -> Result<Option<lsp::GotoDefinitionResponse>> {
    let position = s
        .goto_definition(
            &params.text_document_position_params.text_document.uri,
            params.text_document_position_params.position,
        )
        .await;

    Ok(position.map(lsp::GotoDefinitionResponse::Scalar))
}

/// Handle initialized notification.
async fn completion(
    state: &mut State<'_>,
    params: lsp::CompletionParams,
) -> Result<Option<lsp::CompletionResponse>> {
    let Some(results) = state.complete(
        &params.text_document_position.text_document.uri,
        params.text_document_position.position,
    )?
    else {
        return Ok(None);
    };

    Ok(Some(lsp::CompletionResponse::Array(results.into_std())))
}

/// Handle formatting request.
async fn formatting(
    state: &mut State<'_>,
    params: lsp::DocumentFormattingParams,
) -> Result<Option<::rust_alloc::vec::Vec<lsp::TextEdit>>> {
    state
        .format(&params.text_document.uri)
        .map(|option| option.map(|formatted| vec![formatted]))
}

/// Handle open text document.
async fn did_open_text_document(
    s: &mut State<'_>,
    params: lsp::DidOpenTextDocumentParams,
) -> Result<()> {
    let lagnuage = match params.text_document.language_id.as_str() {
        "rune" => Language::Rune,
        _ => Language::Other,
    };

    if s.workspace_mut()
        .insert_source(
            params.text_document.uri.clone(),
            params.text_document.text.try_into()?,
            lagnuage,
        )?
        .is_some()
    {
        tracing::warn!(
            "opened text document `{}`, but it was already open!",
            params.text_document.uri
        );
    }

    s.rebuild_interest();
    Ok(())
}

/// Handle open text document.
async fn did_change_text_document(
    s: &mut State<'_>,
    params: lsp::DidChangeTextDocumentParams,
) -> Result<()> {
    let mut interest = false;

    if let Some(source) = s.workspace_mut().get_mut(&params.text_document.uri) {
        for change in params.content_changes {
            if let Some(range) = change.range {
                source.modify_lsp_range(range, &change.text)?;
                interest = true;
            }
        }
    } else {
        tracing::warn!(
            "tried to modify `{}`, but it was not open!",
            params.text_document.uri
        );
    }

    if interest {
        s.rebuild_interest();
    }

    Ok(())
}

/// Handle open text document.
async fn did_close_text_document(
    s: &mut State<'_>,
    params: lsp::DidCloseTextDocumentParams,
) -> Result<()> {
    s.workspace_mut().remove(&params.text_document.uri)?;
    s.rebuild_interest();
    Ok(())
}

/// Handle saving of text documents.
async fn did_save_text_document(
    s: &mut State<'_>,
    _: lsp::DidSaveTextDocumentParams,
) -> Result<()> {
    s.rebuild_interest();
    Ok(())
}
