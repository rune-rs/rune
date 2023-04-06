//! <img alt="rune logo" src="https://raw.githubusercontent.com/rune-rs/rune/main/assets/icon.png" />
//! <br>
//! <a href="https://github.com/rune-rs/rune"><img alt="github" src="https://img.shields.io/badge/github-rune--rs/rune-8da0cb?style=for-the-badge&logo=github" height="20"></a>
//! <a href="https://crates.io/crates/rune-languageserver"><img alt="crates.io" src="https://img.shields.io/crates/v/rune-languageserver.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20"></a>
//! <a href="https://docs.rs/rune-languageserver"><img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-rune--languageserver-66c2a5?style=for-the-badge&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20"></a>
//! <a href="https://discord.gg/v5AeNkT"><img alt="chat on discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square" height="20"></a>
//! <br>
//! Minimum support: Rust <b>1.65+</b>.
//! <br>
//! <br>
//! <a href="https://rune-rs.github.io"><b>Visit the site 🌐</b></a>
//! &mdash;
//! <a href="https://rune-rs.github.io/book/"><b>Read the book 📖</b></a>
//! <br>
//! <br>
//!
//! A language server for the Rune Language, an embeddable dynamic programming language for Rust.
//!
//! <br>
//!
//! ## Usage
//!
//! This is part of the [Rune language](https://rune-rs.github.io).

#![allow(clippy::too_many_arguments)]

mod connection;
pub mod envelope;
mod fs;
mod server;
mod state;

use anyhow::Result;
use rune::workspace::MANIFEST_FILE;
use rune::{Context, Options};
use tokio::sync::mpsc;

pub use crate::connection::stdio;
pub use crate::connection::{Input, Output};
pub use crate::server::Server;
pub use crate::state::State;

pub const VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/version.txt"));

pub fn run(context: Context, options: Options) -> Result<()> {
    let (mut input, output) = stdio()?;

    let (rebuild_tx, mut rebuild_rx) = mpsc::channel(1);

    let mut server = Server::new(output, rebuild_tx, context, options);

    server.request_handler::<lsp::request::Initialize, _, _>(initialize);
    server.request_handler::<lsp::request::Shutdown, _, _>(shutdown);

    server.request_handler::<lsp::request::GotoDefinition, _, _>(goto_definition);

    server.notification_handler::<lsp::notification::DidOpenTextDocument, _, _>(
        did_open_text_document,
    );
    server.notification_handler::<lsp::notification::DidChangeTextDocument, _, _>(
        did_change_text_document,
    );
    server.notification_handler::<lsp::notification::DidCloseTextDocument, _, _>(
        did_close_text_document,
    );
    server.notification_handler::<lsp::notification::DidSaveTextDocument, _, _>(
        did_save_text_document,
    );
    server.notification_handler::<lsp::notification::Initialized, _, _>(initialized);

    tracing::info!("Starting server");

    let runtime = tokio::runtime::Runtime::new()?;

    let result: Result<()> = runtime.block_on(async {
        while !server.state().is_stopped() {
            tokio::select! {
                _ = rebuild_rx.recv() => {
                    tracing::info!("rebuilding project");
                    server.rebuild().await?;
                },
                frame = input.next() => {
                    let frame = match frame? {
                        Some(frame) => frame,
                        None => break,
                    };

                    let request: envelope::IncomingMessage<'_> = serde_json::from_slice(frame.content)?;
                    tracing::trace!(?request);
                    server.process(request).await?;
                },
            }
        };

        Ok(())
    });

    match result {
        Ok(()) => tracing::info!("Server stopped"),
        Err(err) => tracing::error!("Server stopped with error: {}", err),
    }

    Ok(())
}

/// Initialize the language server.
async fn initialize(
    state: State,
    output: Output,
    params: lsp::InitializeParams,
) -> Result<lsp::InitializeResult> {
    state.initialize();

    output
        .log(lsp::MessageType::INFO, "Starting language server")
        .await?;

    let capabilities = lsp::ServerCapabilities {
        text_document_sync: Some(lsp::TextDocumentSyncCapability::Kind(
            lsp::TextDocumentSyncKind::INCREMENTAL,
        )),
        definition_provider: Some(lsp::OneOf::Left(true)),
        ..Default::default()
    };

    let server_info = lsp::ServerInfo {
        name: String::from("Rune Language Server"),
        version: None,
    };

    if let Some(root_uri) = &params.root_uri {
        let mut manifest_uri = root_uri.clone();

        if let Ok(mut path) = manifest_uri.path_segments_mut() {
            path.push(MANIFEST_FILE);
        }

        if let Ok(manifest_path) = manifest_uri.to_file_path() {
            if fs::is_file(&manifest_path).await? {
                tracing::trace!(?manifest_uri, ?manifest_path, "Activating workspace");
                state.workspace_mut().await.manifest_path = Some((manifest_uri, manifest_path));
            }
        }
    }

    Ok(lsp::InitializeResult {
        capabilities,
        server_info: Some(server_info),
    })
}

async fn shutdown(state: State, _: Output, _: ()) -> Result<()> {
    state.stop();
    Ok(())
}

/// Handle initialized notification.
async fn initialized(_: State, _: Output, _: lsp::InitializedParams) -> Result<()> {
    tracing::info!("Initialized");
    Ok(())
}

/// Handle initialized notification.
async fn goto_definition(
    state: State,
    _: Output,
    params: lsp::GotoDefinitionParams,
) -> Result<Option<lsp::GotoDefinitionResponse>> {
    let position = state
        .goto_definition(
            &params.text_document_position_params.text_document.uri,
            params.text_document_position_params.position,
        )
        .await;

    Ok(position.map(lsp::GotoDefinitionResponse::Scalar))
}

/// Handle open text document.
async fn did_open_text_document(
    state: State,
    _: Output,
    params: lsp::DidOpenTextDocumentParams,
) -> Result<()> {
    let mut sources = state.workspace_mut().await;

    if sources
        .insert_text(params.text_document.uri.clone(), params.text_document.text)
        .is_some()
    {
        tracing::warn!(
            "opened text document `{}`, but it was already open!",
            params.text_document.uri
        );
    }

    state.rebuild_interest().await?;
    Ok(())
}

/// Handle open text document.
async fn did_change_text_document(
    state: State,
    _: Output,
    params: lsp::DidChangeTextDocumentParams,
) -> Result<()> {
    let mut interest = false;

    {
        let mut sources = state.workspace_mut().await;

        if let Some(source) = sources.get_mut(&params.text_document.uri) {
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
    }

    if interest {
        state.rebuild_interest().await?;
    }

    Ok(())
}

/// Handle open text document.
async fn did_close_text_document(
    state: State,
    _: Output,
    params: lsp::DidCloseTextDocumentParams,
) -> Result<()> {
    let mut sources = state.workspace_mut().await;
    sources.remove(&params.text_document.uri);
    state.rebuild_interest().await?;
    Ok(())
}

/// Handle saving of text documents.
async fn did_save_text_document(
    _: State,
    _: Output,
    _: lsp::DidSaveTextDocumentParams,
) -> Result<()> {
    Ok(())
}
