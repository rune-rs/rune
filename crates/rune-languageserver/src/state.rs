use crate::Output;
use anyhow::{anyhow, Result};
use hashbrown::HashMap;
use lsp::Url;
use ropey::Rope;
use rune::{CompileVisitor, Var};
use runestick::{CompileMeta, CompileMetaKind, Span};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLockWriteGuard;
use tokio::sync::{mpsc, RwLock};

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
    pub async fn sources_mut(&self) -> RwLockWriteGuard<'_, Sources> {
        self.inner.sources.write().await
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

    /// Find definition at the given uri and LSP position.
    pub async fn goto_definition(
        &self,
        uri: &Url,
        position: lsp::Position,
    ) -> Option<lsp::Location> {
        let sources = self.inner.sources.read().await;
        let source = sources.get(uri)?;
        let offset = source.lsp_position_to_offset(position);
        let span = source.find_definition_at(Span::point(offset))?.span?;

        let start = source.offset_to_lsp_position(span.start);
        let end = source.offset_to_lsp_position(span.end);

        let range = lsp::Range { start, end };

        Some(lsp::Location {
            uri: uri.clone(),
            range,
        })
    }

    /// Rebuild the current project.
    pub async fn rebuild(&self, output: &Output) -> Result<()> {
        let mut inner = self.inner.sources.write().await;

        for (url, source) in &mut inner.sources {
            if !std::mem::take(&mut source.dirty) {
                continue;
            }

            let mut sources = rune::Sources::new();
            sources.insert_default(runestick::Source::new(url.to_string(), source.to_string()));

            let mut warnings = rune::Warnings::new();
            let mut diagnostics = Vec::new();

            let mut visitor = Visitor::default();

            let error = rune::load_sources_with_visitor(
                &self.inner.context,
                &self.inner.options,
                &mut sources,
                &mut warnings,
                &mut visitor,
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

            source.index.definitions = visitor.definitions;
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
    sources: RwLock<Sources>,
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
            index: Default::default(),
        };

        self.sources.insert(url, source)
    }

    /// Get the source at the given url.
    pub fn get(&self, url: &Url) -> Option<&Source> {
        self.sources.get(url)
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
    /// Indexes used to answer queries.
    index: Index,
}

impl Source {
    /// Find the definition at the given span.
    pub fn find_definition_at(&self, span: Span) -> Option<&Definition> {
        let (found_span, definition) = self.index.definitions.range(..=span).rev().next()?;

        if span.start >= found_span.start && span.end <= found_span.end {
            return Some(definition);
        }

        None
    }

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

    /// Offset in the rope to lsp position.
    fn lsp_position_to_offset(&self, position: lsp::Position) -> usize {
        let line = self.content.line_to_char(position.line as usize);
        self.content
            .utf16_cu_to_char(line + position.character as usize)
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

#[derive(Default)]
pub struct Index {
    /// Spans mapping to their corresponding definitions.
    definitions: BTreeMap<Span, Definition>,
}

#[derive(Debug, Clone)]
pub struct Definition {
    pub(crate) span: Option<Span>,
    pub(crate) kind: DefinitionKind,
}

#[derive(Debug, Clone, Copy)]
pub enum DefinitionKind {
    /// A tuple.
    Tuple,
    /// A tuple variant.
    TupleVariant,
    /// A struct.
    Struct,
    /// A struct variant.
    StructVariant,
    /// An enum.
    Enum,
    /// A function.
    Function,
    /// A defined closure.
    Closure,
    /// A local variable.
    Local,
}

#[derive(Default)]
struct Visitor {
    definitions: BTreeMap<Span, Definition>,
}

impl CompileVisitor for Visitor {
    fn visit_meta(&mut self, meta: &CompileMeta, span: Span) {
        match &meta.kind {
            CompileMetaKind::Tuple { .. } => {
                self.definitions.insert(
                    span,
                    Definition {
                        span: meta.span,
                        kind: DefinitionKind::Tuple,
                    },
                );
            }
            CompileMetaKind::TupleVariant { .. } => {
                self.definitions.insert(
                    span,
                    Definition {
                        span: meta.span,
                        kind: DefinitionKind::TupleVariant,
                    },
                );
            }
            CompileMetaKind::Struct { .. } => {
                self.definitions.insert(
                    span,
                    Definition {
                        span: meta.span,
                        kind: DefinitionKind::Struct,
                    },
                );
            }
            CompileMetaKind::StructVariant { .. } => {
                self.definitions.insert(
                    span,
                    Definition {
                        span: meta.span,
                        kind: DefinitionKind::StructVariant,
                    },
                );
            }
            CompileMetaKind::Enum { .. } => {
                self.definitions.insert(
                    span,
                    Definition {
                        span: meta.span,
                        kind: DefinitionKind::Enum,
                    },
                );
            }
            CompileMetaKind::Function { .. } => {
                self.definitions.insert(
                    span,
                    Definition {
                        span: meta.span,
                        kind: DefinitionKind::Function,
                    },
                );
            }
            CompileMetaKind::Closure { .. } => {
                self.definitions.insert(
                    span,
                    Definition {
                        span: meta.span,
                        kind: DefinitionKind::Closure,
                    },
                );
            }
            CompileMetaKind::AsyncBlock { .. } => {}
            CompileMetaKind::Macro { .. } => {}
        }
    }

    fn visit_variable_use(&mut self, var: &Var, span: Span) {
        self.definitions.insert(
            span,
            Definition {
                span: Some(var.span()),
                kind: DefinitionKind::Local,
            },
        );
    }
}
