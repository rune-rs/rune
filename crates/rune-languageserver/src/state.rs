use crate::Output;
use anyhow::{anyhow, Result};
use hashbrown::HashMap;
use lsp::Url;
use ropey::Rope;
use runestick::Span;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::sync::{Mutex, MutexGuard};

/// Shared server state.
#[derive(Clone)]
pub struct State {
    inner: Arc<Inner>,
}

impl State {
    /// Construct a new state.
    pub fn new(
        rebuild_tx: mpsc::Sender<()>,
        context: runestick::Context,
        options: rune::Options,
    ) -> Self {
        Self {
            inner: Arc::new(Inner::new(rebuild_tx, context, options)),
        }
    }

    /// Mark server as initialized.
    pub fn initialize(&self) {
        self.inner.initialized.store(true, Ordering::Release);
    }

    /// Test if server is initialized.
    pub fn is_initialized(&self) -> bool {
        self.inner.initialized.load(Ordering::Acquire)
    }

    /// Acess sources in the current state.
    pub async fn sources(&self) -> MutexGuard<'_, Sources> {
        self.inner.sources.lock().await
    }

    /// Indicate interest in having the project rebuild.
    ///
    /// Sources that have been modified will be marked as dirty.
    pub async fn rebuild_interest(&self) -> Result<()> {
        self.inner
            .rebuild_tx
            .clone()
            .send(())
            .await
            .map_err(|_| anyhow!("failed to send rebuild interest"))
    }

    /// Rebuild the current project.
    pub async fn rebuild(&self, output: &Output) -> Result<()> {
        let mut inner = self.inner.sources.lock().await;

        for (url, source) in &mut inner.sources {
            if !std::mem::take(&mut source.dirty) {
                continue;
            }

            let mut sources = rune::Sources::new();
            sources.insert_default(runestick::Source::new(url.to_string(), source.to_string()));

            let mut warnings = rune::Warnings::new();
            let mut diagnostics = Vec::new();

            let error = rune::load_sources(
                &self.inner.context,
                &self.inner.options,
                &mut sources,
                &mut warnings,
            );

            if let Err(error) = error {
                match error.kind() {
                    rune::LoadErrorKind::ReadFile { error, path } => {
                        diagnostics.push(source.display_to_error(
                            Span::empty(),
                            format!("failed to read file: {}: {}", path.display(), error),
                        ));
                    }
                    // TODO: match source id with the document that has the error.
                    rune::LoadErrorKind::ParseError { error, .. } => {
                        diagnostics.push(source.display_to_error(error.span(), error));
                    }
                    // TODO: match the source id with the document that has the error.
                    rune::LoadErrorKind::CompileError { error, .. } => {
                        diagnostics.push(source.display_to_error(error.span(), error));
                    }
                    rune::LoadErrorKind::LinkError { errors } => {
                        for error in errors {
                            match error {
                                rune::LinkerError::MissingFunction { hash, spans } => {
                                    for (span, _) in spans {
                                        diagnostics.push(source.display_to_error(
                                            *span,
                                            format!("missing function with hash `{}`", hash),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    rune::LoadErrorKind::Internal { message } => {
                        diagnostics.push(source.display_to_error(Span::empty(), message));
                    }
                }

                log::error!("build error: {:?}", error);
            }

            for warning in &warnings {
                diagnostics.push(source.display_to_warning(warning.span(), warning.kind()));
            }

            let diagnostics = lsp::PublishDiagnosticsParams {
                uri: url.clone(),
                diagnostics,
                version: None,
            };

            output
                .notification::<lsp::notification::PublishDiagnostics>(diagnostics)
                .await?;
        }

        Ok(())
    }
}

struct Inner {
    /// The rune context to build for.
    context: runestick::Context,
    /// Build options.
    options: rune::Options,
    /// Sender to indicate interest in rebuilding the project.
    /// Can be triggered on modification.
    rebuild_tx: mpsc::Sender<()>,
    /// Indicate if the server is initialized.
    initialized: AtomicBool,
    /// Sources used in the project.
    sources: Mutex<Sources>,
    /// Indexes used to answer queries.
    /// TODO: will be used.
    #[allow(unused)]
    indexes: RwLock<Indexes>,
}

impl Inner {
    /// Construct a new empty inner state.
    pub fn new(
        rebuild_tx: mpsc::Sender<()>,
        context: runestick::Context,
        options: rune::Options,
    ) -> Self {
        Self {
            context,
            options,
            rebuild_tx,
            initialized: Default::default(),
            sources: Default::default(),
            indexes: Default::default(),
        }
    }
}

/// A collection of open sources.
#[derive(Default)]
pub struct Sources {
    sources: HashMap<Url, Source>,
}

impl Sources {
    /// Insert the given source at the given url.
    pub fn insert_text(&mut self, url: Url, text: String) -> Option<Source> {
        let source = Source {
            dirty: true,
            content: Rope::from(text),
        };

        self.sources.insert(url, source)
    }

    /// Get the mutable source at the given url.
    pub fn get_mut(&mut self, url: &Url) -> Option<&mut Source> {
        self.sources.get_mut(url)
    }

    /// Remove the given url as a source.
    pub fn remove(&mut self, url: &Url) -> Option<Source> {
        self.sources.remove(url)
    }
}

/// A single open source.
pub struct Source {
    /// If the source is dirty and needs to be rebuilt.
    dirty: bool,
    /// The content of the current source.
    content: Rope,
}

impl Source {
    /// Modify the given lsp range in the file.
    pub fn modify_lsp_range(&mut self, range: lsp::Range, content: &str) -> Result<()> {
        let start = rope_utf16_position(&self.content, range.start)?;
        let end = rope_utf16_position(&self.content, range.end)?;
        self.content.remove(start..end);

        if !content.is_empty() {
            self.content.insert(start, content);
        }

        self.dirty = true;
        Ok(())
    }

    /// Convert the given span and error into an error diagnostic.
    fn display_to_error<E>(&self, span: Span, error: E) -> lsp::Diagnostic
    where
        E: fmt::Display,
    {
        self.display_to_diagnostic(span, error, lsp::DiagnosticSeverity::Error)
    }

    /// Convert the given span and error into a warning diagnostic.
    fn display_to_warning<E>(&self, span: Span, error: E) -> lsp::Diagnostic
    where
        E: fmt::Display,
    {
        self.display_to_diagnostic(span, error, lsp::DiagnosticSeverity::Warning)
    }

    /// Convert a span and something displayeable into diagnostics.
    fn display_to_diagnostic<E>(
        &self,
        span: Span,
        error: E,
        severity: lsp::DiagnosticSeverity,
    ) -> lsp::Diagnostic
    where
        E: fmt::Display,
    {
        let start = self.offset_to_lsp_position(span.start);
        let end = self.offset_to_lsp_position(span.end);

        lsp::Diagnostic {
            range: lsp::Range::new(start, end),
            severity: Some(severity),
            code: None,
            source: None,
            message: error.to_string(),
            related_information: None,
            tags: None,
        }
    }

    /// Offset in the rope to lsp position.
    fn offset_to_lsp_position(&self, offset: usize) -> lsp::Position {
        let line = self.content.byte_to_line(offset);

        let col_char = self.content.byte_to_char(offset);
        let col_char = self.content.char_to_utf16_cu(col_char);

        let line_char = self.content.line_to_char(line);
        let line_char = self.content.char_to_utf16_cu(line_char);

        let col_char = col_char - line_char;

        // TODO: handle utf-16 conversion.
        lsp::Position::new(line as u64, col_char as u64)
    }

    /// Iterate over the text chunks in the source.
    pub fn chunks(&self) -> impl Iterator<Item = &str> {
        self.content.chunks()
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.content)
    }
}

/// Translate the given lsp::Position, which is in UTF-16 because Microsoft.
///
/// Please go complain here:
/// https://github.com/microsoft/language-server-protocol/issues/376
fn rope_utf16_position(rope: &Rope, position: lsp::Position) -> Result<usize> {
    let line = rope.line(position.line as usize);

    // encoding target.
    let character = position.character as usize;

    let mut utf16_offset = 0usize;
    let mut char_offset = 0usize;

    for c in line.chars() {
        if utf16_offset == character {
            break;
        }

        if utf16_offset > character {
            return Err(anyhow!("character is not on an offset boundary"));
        }

        utf16_offset += c.len_utf16();
        char_offset += 1;
    }

    Ok(rope.line_to_char(position.line as usize) + char_offset)
}

/// All indexes used to answer questions.
/// TODO: will be used.
#[allow(unused)]
#[derive(Default)]
pub struct Indexes {
    /// Indexes by url.
    by_url: HashMap<Url, Index>,
}

/// TODO: will be used.
#[allow(unused)]
pub struct Index {
    /// Spans mapping to their corresponding definitions.
    goto_definitions: BTreeMap<Span, Definition>,
}

/// Definitions that can be jumped to.
/// TODO: will be used.
#[allow(unused)]
pub enum Definition {}
