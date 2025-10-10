//! Utility for building a language server.

#![allow(clippy::too_many_arguments)]

#[cfg(test)]
mod tests;

mod completion;
mod connection;
pub mod envelope;
mod fs;
mod state;
mod url;

use anyhow::Context as _;
use lsp::notification::Notification;
use lsp::request::Request;
use serde::Deserialize;
#[cfg(feature = "std")]
use tokio::io::{self, Stdin, Stdout};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt as _};
use tokio::sync::Notify;

use crate::alloc::String;
use crate::languageserver::envelope::Code;
use crate::languageserver::state::State;
use crate::support::Result;
use crate::workspace::MANIFEST_FILE;
use crate::{Context, Options};

use self::connection::Input;
use self::state::StateEncoding;

/// Construct a new empty builder without any configured I/O.
///
/// In order to actually call build, the input and output streams must be
/// configured using [`with_input`], and [`with_output`], or a method such as
/// [`with_stdio`].
///
/// [`with_input`]: Builder::with_input
/// [`with_output`]: Builder::with_output
/// [`with_stdio`]: Builder::with_stdio
///
/// # Examples
///
/// ```no_run
/// use rune::Context;
/// use rune::languageserver;
///
/// let context = Context::with_default_modules()?;
///
/// let languageserver = languageserver::builder()
///     .with_context(context)
///     .with_stdio()
///     .build()?;
///
/// # Ok::<_, rune::support::Error>(())
/// ```
pub fn builder() -> Builder<Unset, Unset> {
    Builder {
        input: Unset,
        output: Unset,
        context: None,
        options: None,
    }
}

/// A builder for a language server.
///
/// See [`builder()`] for more details.
pub struct Builder<I, O> {
    input: I,
    output: O,
    context: Option<Context>,
    options: Option<Options>,
}

/// Unset placeholder I/O types for language server.
///
/// These must be replaced in order to actually construct a language server.
///
/// See [`builder()`] for more details.
pub struct Unset;

impl<I, O> Builder<I, O> {
    /// Associate the specified input with the builder.
    pub fn with_input<T>(self, input: T) -> Builder<T, O>
    where
        T: Unpin + AsyncRead,
    {
        Builder {
            input,
            output: self.output,
            context: self.context,
            options: self.options,
        }
    }

    /// Associate the specified output with the builder.
    pub fn with_output<T>(self, output: T) -> Builder<I, T>
    where
        T: Unpin + AsyncWrite,
    {
        Builder {
            input: self.input,
            output,
            context: self.context,
            options: self.options,
        }
    }

    /// Associate [`Stdin`] and [`Stdout`] as the input and output of the
    /// builder.
    #[cfg(feature = "std")]
    #[cfg_attr(docsrs, doc(cfg(feature = "std")))]
    pub fn with_stdio(self) -> Builder<Stdin, Stdout> {
        self.with_input(io::stdin()).with_output(io::stdout())
    }

    /// Associate the specified context with the builder.
    ///
    /// If none is specified, a default context will be constructed.
    pub fn with_context(self, context: Context) -> Self {
        Self {
            input: self.input,
            output: self.output,
            context: Some(context),
            options: self.options,
        }
    }

    /// Associate the specified options with the builder.
    pub fn with_options(self, options: Options) -> Self {
        Self {
            input: self.input,
            output: self.output,
            context: self.context,
            options: Some(options),
        }
    }

    /// Build a new language server using the provided options.
    pub fn build(self) -> Result<LanguageServer<I, O>>
    where
        I: Unpin + AsyncRead,
        O: Unpin + AsyncWrite,
    {
        let context = match self.context {
            Some(context) => context,
            None => Context::with_default_modules()?,
        };

        let options = match self.options {
            Some(options) => options,
            None => Options::from_default_env()?,
        };

        Ok(LanguageServer {
            input: self.input,
            output: self.output,
            context,
            options,
        })
    }
}

enum Language {
    Rune,
    Other,
}

/// The instance of a language server, as constructed through [`builder()`].
pub struct LanguageServer<I, O> {
    input: I,
    output: O,
    context: Context,
    options: Options,
}

impl<I, O> LanguageServer<I, O>
where
    I: Unpin + AsyncRead,
    O: Unpin + AsyncWrite,
{
    /// Run a language server.
    pub async fn run(mut self) -> Result<()> {
        let mut input = Input::new(self.input);

        let rebuild_notify = Notify::new();

        let rebuild = rebuild_notify.notified();
        tokio::pin!(rebuild);

        let mut state = State::new(&rebuild_notify, self.context, self.options);
        tracing::info!("Starting server");
        state.rebuild()?;

        while !state.is_stopped() {
            tokio::select! {
                _ = rebuild.as_mut() => {
                    tracing::info!("rebuilding project");
                    state.rebuild()?;
                    rebuild.set(rebuild_notify.notified());
                },
                len = self.output.write(state.out.readable()), if !state.out.is_empty() => {
                    let len = len.context("writing output")?;
                    state.out.advance(len);

                    if state.out.is_empty() {
                        self.output.flush().await.context("flushing output")?;
                    }
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
                        state.out
                            .error(
                                incoming.id,
                                Code::InvalidRequest,
                                "Server not initialized",
                                None::<()>,
                            )?;

                        continue;
                    }

                    macro_rules! handle {
                        ($(req($req_ty:ty, $req_handle:ident)),* $(, notif($notif_ty:ty, $notif_handle:ident))* $(,)?) => {
                            match incoming.method.as_str() {
                                $(<$req_ty>::METHOD => {
                                    let params = <$req_ty as Request>::Params::deserialize(incoming.params)?;
                                    let result = $req_handle(&mut state, params)?;
                                    state.out.response(incoming.id, result)?;
                                })*
                                $(<$notif_ty>::METHOD => {
                                    let params = <$notif_ty as Notification>::Params::deserialize(incoming.params)?;
                                    let () = $notif_handle(&mut state, params)?;
                                })*
                                _ => {
                                    state.out.log(
                                        lsp::MessageType::INFO,
                                        format!("Unhandled method `{}`", incoming.method),
                                    )?;
                                    state.out.method_not_found(incoming.id)?;
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
                        req(lsp::request::RangeFormatting, range_formatting),
                        notif(lsp::notification::DidOpenTextDocument, did_open_text_document),
                        notif(lsp::notification::DidChangeTextDocument, did_change_text_document),
                        notif(lsp::notification::DidCloseTextDocument, did_close_text_document),
                        notif(lsp::notification::DidSaveTextDocument, did_save_text_document),
                        notif(lsp::notification::Initialized, initialized),
                    }
                },
            }
        }

        while !state.out.is_empty() {
            let len = self.output.write(state.out.readable()).await?;
            state.out.advance(len);
        }

        Ok(())
    }
}

fn is_utf8(params: &lsp::InitializeParams) -> bool {
    let Some(general) = &params.capabilities.general else {
        return false;
    };

    let Some(encodings) = &general.position_encodings else {
        return false;
    };

    for encoding in encodings {
        if *encoding == lsp::PositionEncodingKind::UTF8 {
            return true;
        }
    }

    false
}

/// Initialize the language state.
fn initialize(s: &mut State<'_>, params: lsp::InitializeParams) -> Result<lsp::InitializeResult> {
    s.initialize();

    s.out
        .log(lsp::MessageType::INFO, "Starting language server")?;

    let position_encoding;

    if is_utf8(&params) {
        s.encoding = StateEncoding::Utf8;
        position_encoding = Some(lsp::PositionEncodingKind::UTF8);
    } else {
        position_encoding = None;
    }

    s.out.log(
        lsp::MessageType::INFO,
        format_args!("Using {} position encoding", s.encoding),
    )?;

    let capabilities = lsp::ServerCapabilities {
        position_encoding,
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
        document_range_formatting_provider: Some(lsp::OneOf::Left(true)),
        ..Default::default()
    };

    let server_info = lsp::ServerInfo {
        name: String::try_from("Rune Language Server")?.into_std(),
        version: None,
    };

    let mut rebuild = false;

    #[allow(deprecated)]
    if let Some(root_uri) = &params.root_uri {
        let mut manifest_uri = root_uri.clone();

        if let Ok(mut path) = manifest_uri.path_segments_mut() {
            path.push(MANIFEST_FILE);
        }

        if let Ok(manifest_path) = manifest_uri.to_file_path() {
            if fs::is_file(&manifest_path)? {
                tracing::trace!(?manifest_uri, ?manifest_path, "Activating workspace");
                s.workspace.manifest_path = Some((manifest_uri, manifest_path));
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

fn shutdown(s: &mut State<'_>, _: ()) -> Result<()> {
    s.stop();
    Ok(())
}

/// Handle initialized notification.
fn initialized(_: &mut State<'_>, _: lsp::InitializedParams) -> Result<()> {
    tracing::info!("Initialized");
    Ok(())
}

/// Handle initialized notification.
fn goto_definition(
    s: &mut State<'_>,
    params: lsp::GotoDefinitionParams,
) -> Result<Option<lsp::GotoDefinitionResponse>> {
    let position = s.goto_definition(
        &params.text_document_position_params.text_document.uri,
        params.text_document_position_params.position,
    )?;

    Ok(position.map(lsp::GotoDefinitionResponse::Scalar))
}

/// Handle initialized notification.
fn completion(
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
fn formatting(
    state: &mut State<'_>,
    params: lsp::DocumentFormattingParams,
) -> Result<Option<rust_alloc::vec::Vec<lsp::TextEdit>>> {
    state
        .format(&params.text_document.uri)
        .map(|option| option.map(|formatted| vec![formatted]))
}

/// Handle formatting request.
fn range_formatting(
    state: &mut State<'_>,
    params: lsp::DocumentRangeFormattingParams,
) -> Result<Option<rust_alloc::vec::Vec<lsp::TextEdit>>> {
    state
        .range_format(&params.text_document.uri, &params.range)
        .map(|option| option.map(|formatted| vec![formatted]))
}

/// Handle open text document.
fn did_open_text_document(s: &mut State<'_>, params: lsp::DidOpenTextDocumentParams) -> Result<()> {
    let lagnuage = match params.text_document.language_id.as_str() {
        "rune" => Language::Rune,
        _ => Language::Other,
    };

    if s.workspace
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
fn did_change_text_document(
    s: &mut State<'_>,
    params: lsp::DidChangeTextDocumentParams,
) -> Result<()> {
    let mut interest = false;

    if let Some(source) = s.workspace.get_mut(&params.text_document.uri) {
        for change in params.content_changes {
            if let Some(range) = change.range {
                source.modify_lsp_range(&s.encoding, range, &change.text)?;
            } else {
                source.modify_lsp_full_range(&change.text)?;
            }
            interest = true;
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
fn did_close_text_document(
    s: &mut State<'_>,
    params: lsp::DidCloseTextDocumentParams,
) -> Result<()> {
    s.workspace.remove(&params.text_document.uri)?;
    s.rebuild_interest();
    Ok(())
}

/// Handle saving of text documents.
fn did_save_text_document(s: &mut State<'_>, _: lsp::DidSaveTextDocumentParams) -> Result<()> {
    s.rebuild_interest();
    Ok(())
}
