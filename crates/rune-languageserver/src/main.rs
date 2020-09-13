//! <div align="center">
//!     <img alt="Rune Logo" src="https://raw.githubusercontent.com/rune-rs/rune/master/assets/icon.png" />
//! </div>
//!
//! <br>
//!
//! <div align="center">
//! <a href="https://rune-rs.github.io/rune/">
//!     <b>Read the Book 📖</b>
//! </a>
//! </div>
//!
//! <br>
//!
//! <div align="center">
//! <a href="https://github.com/rune-rs/rune/actions">
//!     <img alt="Build Status" src="https://github.com/rune-rs/rune/workflows/Build/badge.svg">
//! </a>
//!
//! <a href="https://github.com/rune-rs/rune/actions">
//!     <img alt="Book Status" src="https://github.com/rune-rs/rune/workflows/Book/badge.svg">
//! </a>
//!
//! <a href="https://crates.io/crates/rune">
//!     <img alt="crates.io" src="https://img.shields.io/crates/v/rune.svg">
//! </a>
//!
//! <a href="https://docs.rs/rune">
//!     <img alt="docs.rs" src="https://docs.rs/rune/badge.svg">
//! </a>
//!
//! <a href="https://discord.gg/v5AeNkT">
//!     <img alt="Chat on Discord" src="https://img.shields.io/discord/558644981137670144.svg?logo=discord&style=flat-square">
//! </a>
//! </div>
//!
//! <br>
//!
//! The language server for the [Rune language].
//!
//! [Rune Language]: https://github.com/rune-rs/rune

use anyhow::Result;
use rune_languageserver::envelope::IncomingMessage;
use rune_languageserver::{Output, Server, State};
use tokio::sync::mpsc;

fn setup_logging() -> Result<()> {
    // Set environment variable to get the language server to trace log to the
    // given file.
    if let Some(log_path) = std::env::var_os("RUNE_TRACE_LOG_FILE") {
        use log::LevelFilter;
        use log4rs::append::file::FileAppender;
        use log4rs::config::{Appender, Config, Root};
        use log4rs::encode::pattern::PatternEncoder;

        let logfile = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
            .build(log_path)?;

        let config = Config::builder()
            .appender(Appender::builder().build("logfile", Box::new(logfile)))
            .build(Root::builder().appender("logfile").build(LevelFilter::Info))?;

        log4rs::init_config(config)?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_logging()?;

    let mut context = rune::default_context()?;
    context.install(&rune_macros::module()?)?;

    let mut options = rune::Options::default();
    options.macros(true);

    let (mut input, output) = rune_languageserver::stdio()?;

    let (rebuild_tx, mut rebuild_rx) = mpsc::channel(1);

    let mut server = Server::new(output.clone(), rebuild_tx, context, options);

    server.request_handler::<lsp::request::Initialize, _, _>(initialize);

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

    log::info!("Starting server");

    loop {
        tokio::select! {
            _ = rebuild_rx.recv() => {
                server.rebuild().await?;
            },
            frame = input.next() => {
                let frame = match frame? {
                    Some(frame) => frame,
                    None => break,
                };

                let request: IncomingMessage = serde_json::from_slice(frame.content)?;
                server.process(request).await?;
            },
        }
    }

    Ok(())
}

/// Initialize the language server.
async fn initialize(
    state: State,
    output: Output,
    _: lsp::InitializeParams,
) -> Result<lsp::InitializeResult> {
    state.initialize();

    output
        .log(lsp::MessageType::Info, "Starting language server")
        .await?;

    let mut capabilities = lsp::ServerCapabilities::default();

    capabilities.text_document_sync = Some(lsp::TextDocumentSyncCapability::Kind(
        lsp::TextDocumentSyncKind::Incremental,
    ));

    capabilities.definition_provider = Some(true);

    let server_info = lsp::ServerInfo {
        name: String::from("Rune Language Server"),
        version: None,
    };

    Ok(lsp::InitializeResult {
        capabilities,
        server_info: Some(server_info),
    })
}

/// Handle initialized notification.
async fn initialized(_: State, _: Output, _: lsp::InitializedParams) -> Result<()> {
    log::info!("Initialized");
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
    let mut sources = state.sources_mut().await;

    if sources
        .insert_text(params.text_document.uri.clone(), params.text_document.text)
        .is_some()
    {
        log::warn!(
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
        let mut sources = state.sources_mut().await;

        if let Some(source) = sources.get_mut(&params.text_document.uri) {
            for change in params.content_changes {
                if let Some(range) = change.range {
                    source.modify_lsp_range(range, &change.text)?;
                    interest = true;
                }
            }
        } else {
            log::warn!(
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
    let mut sources = state.sources_mut().await;

    if sources.remove(&params.text_document.uri).is_none() {
        log::warn!(
            "tried to close text document `{}`, but it was not open!",
            params.text_document.uri
        );
    }

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
