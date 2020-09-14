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
        let definition = source.find_definition_at(Span::point(offset))?;

        let url = definition.url.as_ref()?;

        let range = match definition.span {
            Some(span) => {
                let start = source.offset_to_lsp_position(span.start);
                let end = source.offset_to_lsp_position(span.end);
                lsp::Range { start, end }
            }
            None => lsp::Range::default(),
        };

        Some(lsp::Location {
            uri: url.clone(),
            range,
        })
    }

    /// Rebuild the current project.
    pub async fn rebuild(&self, output: &Output) -> Result<()> {
        let mut inner = self.inner.sources.write().await;

        let mut by_url = HashMap::<Url, Vec<lsp::Diagnostic>>::new();

        for (url, _) in inner.removed.drain(..) {
            by_url.insert(url.clone(), Vec::new());
        }

        let mut definitions = HashMap::new();

        for (url, source) in &inner.sources {
            log::trace!("build: {}", url);

            by_url.insert(url.clone(), Default::default());
            definitions.insert(url.clone(), Default::default());

            let mut sources = rune::Sources::new();

            let mut input = runestick::Source::new(url.to_string(), source.to_string());
            *input.url_mut() = Some(url.clone());

            sources.insert(input);

            let mut warnings = rune::Warnings::new();
            let mut visitor = Visitor::new(&mut definitions);
            let mut source_loader = SourceLoader::new(&inner.sources);

            let error = rune::load_sources_with_visitor(
                &self.inner.context,
                &self.inner.options,
                &mut sources,
                &mut warnings,
                &mut visitor,
                &mut source_loader,
            );

            if let Err(error) = error {
                match error.kind() {
                    rune::LoadErrorKind::ReadFile { error, path } => {
                        let diagnostics = by_url.entry(url.clone()).or_default();

                        let range = lsp::Range::default();

                        diagnostics.push(display_to_error(
                            range,
                            format!("failed to read file: {}: {}", path.display(), error),
                        ));
                    }
                    // TODO: match source id with the document that has the error.
                    rune::LoadErrorKind::ParseError {
                        error, source_id, ..
                    } => {
                        report(
                            &sources,
                            &mut by_url,
                            error.span(),
                            *source_id,
                            error,
                            display_to_error,
                        );
                    }
                    // TODO: match the source id with the document that has the error.
                    rune::LoadErrorKind::CompileError {
                        error, source_id, ..
                    } => {
                        report(
                            &sources,
                            &mut by_url,
                            error.span(),
                            *source_id,
                            error,
                            display_to_error,
                        );
                    }
                    rune::LoadErrorKind::LinkError { errors } => {
                        for error in errors {
                            match error {
                                rune::LinkerError::MissingFunction { hash, spans } => {
                                    for (span, _) in spans {
                                        let diagnostics = by_url.entry(url.clone()).or_default();

                                        let range = source.span_to_lsp_range(*span);

                                        diagnostics.push(display_to_error(
                                            range,
                                            format!("missing function with hash `{}`", hash),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    rune::LoadErrorKind::Internal { message } => {
                        let diagnostics = by_url.entry(url.clone()).or_default();

                        let range = lsp::Range::default();
                        diagnostics.push(display_to_error(range, message));
                    }
                }
            }

            for warning in &warnings {
                report(
                    &sources,
                    &mut by_url,
                    warning.span(),
                    warning.source_id,
                    warning.kind(),
                    display_to_warning,
                );
            }
        }

        for (url, index) in definitions {
            if let Some(source) = inner.sources.get_mut(&url) {
                source.index = index;
            }
        }

        for (url, diagnostics) in by_url {
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
    /// Sources that might be modified.
    sources: HashMap<Url, Source>,
    /// A source that has been removed.
    removed: Vec<(Url, Source)>,
}

impl Sources {
    /// Insert the given source at the given url.
    pub fn insert_text(&mut self, url: Url, text: String) -> Option<Source> {
        let source = Source {
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
    pub fn remove(&mut self, url: &Url) {
        if let Some(source) = self.sources.remove(url) {
            self.removed.push((url.clone(), source));
        }
    }
}

/// A single open source.
pub struct Source {
    /// The content of the current source.
    content: Rope,
    /// Indexes used to answer queries.
    index: Index,
}

impl Source {
    /// Find the definition at the given span.
    pub fn find_definition_at(&self, span: Span) -> Option<&Definition> {
        let (found_span, definition) = self.index.definitions.range(..=span).rev().next()?;
        log::info!("found {:?} (at {:?})", definition.kind, definition.url);

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

        Ok(())
    }

    /// Convert a span to an lsp range.
    fn span_to_lsp_range(&self, span: Span) -> lsp::Range {
        let start = self.offset_to_lsp_position(span.start);
        let end = self.offset_to_lsp_position(span.end);

        lsp::Range { start, end }
    }

    /// Offset in the rope to lsp position.
    fn offset_to_lsp_position(&self, offset: usize) -> lsp::Position {
        let line = self.content.byte_to_line(offset);

        let col_char = self.content.byte_to_char(offset);
        let col_char = self.content.char_to_utf16_cu(col_char);

        let line_char = self.content.line_to_char(line);
        let line_char = self.content.char_to_utf16_cu(line_char);

        let col_char = col_char - line_char;

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

/// Conver the given span into an lsp range.
fn span_to_lsp_range(source: &runestick::Source, span: Span) -> Option<lsp::Range> {
    let (line, character) = source.position_to_utf16cu_line_char(span.start)?;
    let start = lsp::Position::new(line as u64, character as u64);
    let (line, character) = source.position_to_utf16cu_line_char(span.end)?;
    let end = lsp::Position::new(line as u64, character as u64);
    Some(lsp::Range::new(start, end))
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

/// Convert the given span and error into an error diagnostic.
fn report<E, R>(
    sources: &rune::Sources,
    by_url: &mut HashMap<Url, Vec<lsp::Diagnostic>>,
    span: Span,
    source_id: usize,
    error: E,
    report: R,
) where
    E: fmt::Display,
    R: Fn(lsp::Range, E) -> lsp::Diagnostic,
{
    let source = match sources.get(source_id) {
        Some(source) => &*source,
        None => return,
    };

    let url = match source.url() {
        Some(url) => url,
        None => return,
    };

    let range = match span_to_lsp_range(&*source, span) {
        Some(range) => range,
        None => return,
    };

    let diagnostics = by_url.entry(url.clone()).or_default();
    diagnostics.push(report(range, error));
}

/// Convert the given span and error into an error diagnostic.
fn display_to_error<E>(range: lsp::Range, error: E) -> lsp::Diagnostic
where
    E: fmt::Display,
{
    display_to_diagnostic(range, error, lsp::DiagnosticSeverity::Error)
}

/// Convert the given span and error into a warning diagnostic.
fn display_to_warning<E>(range: lsp::Range, error: E) -> lsp::Diagnostic
where
    E: fmt::Display,
{
    display_to_diagnostic(range, error, lsp::DiagnosticSeverity::Warning)
}

/// Convert a span and something displayeable into diagnostics.
fn display_to_diagnostic<E>(
    range: lsp::Range,
    error: E,
    severity: lsp::DiagnosticSeverity,
) -> lsp::Diagnostic
where
    E: fmt::Display,
{
    lsp::Diagnostic {
        range,
        severity: Some(severity),
        code: None,
        source: None,
        message: error.to_string(),
        related_information: None,
        tags: None,
    }
}

#[derive(Default)]
pub struct Index {
    /// Spans mapping to their corresponding definitions.
    definitions: BTreeMap<Span, Definition>,
}

#[derive(Debug, Clone)]
pub struct Definition {
    pub(crate) span: Option<Span>,
    pub(crate) url: Option<Url>,
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
    /// A module that can be jumped to.
    Module,
}

struct Visitor<'a> {
    indexes: &'a mut HashMap<Url, Index>,
}

impl<'a> Visitor<'a> {
    /// Construct a new visitor.
    pub fn new(indexes: &'a mut HashMap<Url, Index>) -> Self {
        Self { indexes }
    }
}

impl CompileVisitor for Visitor<'_> {
    fn visit_meta(&mut self, url: &Url, meta: &CompileMeta, span: Span) {
        let kind = match &meta.kind {
            CompileMetaKind::Tuple { .. } => DefinitionKind::Tuple,
            CompileMetaKind::TupleVariant { .. } => DefinitionKind::TupleVariant,
            CompileMetaKind::Struct { .. } => DefinitionKind::Struct,
            CompileMetaKind::StructVariant { .. } => DefinitionKind::StructVariant,
            CompileMetaKind::Enum { .. } => DefinitionKind::Enum,
            CompileMetaKind::Function { .. } => DefinitionKind::Function,
            CompileMetaKind::Closure { .. } => DefinitionKind::Closure,
            CompileMetaKind::AsyncBlock { .. } => return,
            CompileMetaKind::Macro { .. } => return,
        };

        let definition = Definition {
            span: meta.span,
            url: meta.url.clone(),
            kind,
        };

        if let Some(index) = self.indexes.get_mut(url) {
            if let Some(d) = index.definitions.insert(span, definition) {
                log::warn!("replaced definition: {:?}", d.kind)
            }
        }
    }

    fn visit_variable_use(&mut self, url: &Url, var: &Var, span: Span) {
        if let Some(index) = self.indexes.get_mut(url) {
            let definition = Definition {
                span: Some(var.span()),
                url: Some(url.clone()),
                kind: DefinitionKind::Local,
            };

            if let Some(d) = index.definitions.insert(span, definition) {
                log::warn!("replaced definition: {:?}", d.kind)
            }
        }
    }

    fn visit_mod(&mut self, url: &Url, span: Span) {
        if let Some(index) = self.indexes.get_mut(url) {
            let definition = Definition {
                span: None,
                url: Some(url.clone()),
                kind: DefinitionKind::Module,
            };

            if let Some(d) = index.definitions.insert(span, definition) {
                log::warn!("replaced definition: {:?}", d.kind)
            }
        }
    }
}

struct SourceLoader<'a> {
    sources: &'a HashMap<Url, Source>,
    base: rune::FileSourceLoader,
}

impl<'a> SourceLoader<'a> {
    /// Construct a new source loader.
    pub fn new(sources: &'a HashMap<Url, Source>) -> Self {
        Self {
            sources,
            base: rune::FileSourceLoader::new(),
        }
    }

    /// Generate a collection of URl candidates.
    fn candidates(url: &Url, name: &str) -> Option<[Url; 2]> {
        let mut a = url.clone();

        {
            let mut path = a.path_segments_mut().ok()?;
            path.pop();
            path.push(&format!("{}.rn", name));
        }

        let mut b = url.clone();

        {
            let mut path = b.path_segments_mut().ok()?;
            path.pop();
            path.push(name);
            path.push("mod.rn");
        };

        Some([a, b])
    }
}

impl rune::SourceLoader for SourceLoader<'_> {
    fn load(
        &mut self,
        url: &Url,
        name: &str,
        span: Span,
    ) -> Result<runestick::Source, rune::CompileError> {
        log::trace!("load: {}", url);

        if let Some(candidates) = Self::candidates(url, name) {
            for url in candidates.iter() {
                if let Some(s) = self.sources.get(url) {
                    // TODO: can this clone be avoided? The compiler requires a complete buffer.
                    let mut source = runestick::Source::new(url.to_string(), s.to_string());
                    *source.url_mut() = Some(url.clone());
                    return Ok(source);
                }
            }
        }

        self.base.load(url, name, span)
    }
}
