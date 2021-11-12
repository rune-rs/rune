use crate::Output;
use anyhow::{anyhow, Result};
use hashbrown::HashMap;
use lsp::Url;
use ropey::Rope;
use rune::compile::{CompileError, CompileVisitor, FileSourceLoader, LinkerError};
use rune::diagnostics::{Diagnostic, FatalDiagnosticKind};
use rune::meta::{CompileMeta, CompileMetaKind, CompileSource};
use rune::{ComponentRef, Context, Item, Location, Options, SourceId, Span, Spanned};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;
use std::rc::Rc;
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
    pub fn new(rebuild_tx: mpsc::Sender<()>, context: Context, options: Options) -> Self {
        Self {
            inner: Arc::new(Inner {
                rebuild_tx,
                context,
                options,
                initialized: Default::default(),
                sources: Default::default(),
            }),
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

    /// Access sources in the current state.
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
        let def = source.find_definition_at(Span::point(offset))?;

        let url = match def.source.path() {
            Some(path) => Url::from_file_path(path).ok()?,
            None => uri.clone(),
        };

        let source = source.build_sources.as_ref()?.get(def.source.source_id())?;

        let (l, c) = source.position_to_utf16cu_line_char(def.source.span().start.into_usize())?;
        let start = lsp::Position {
            line: l as u32,
            character: c as u32,
        };

        let (l, c) = source.position_to_utf16cu_line_char(def.source.span().end.into_usize())?;
        let end = lsp::Position {
            line: l as u32,
            character: c as u32,
        };

        let range = lsp::Range { start, end };

        let location = lsp::Location { uri: url, range };

        log::trace!("go to location: {:?}", location);
        Some(location)
    }

    /// Rebuild the current project.
    pub async fn rebuild(&self, output: &Output) -> Result<()> {
        let mut inner = self.inner.sources.write().await;

        let mut by_url = HashMap::<Url, Vec<lsp::Diagnostic>>::new();

        for (url, _) in inner.removed.drain(..) {
            by_url.insert(url.clone(), Vec::new());
        }

        let mut builds = Vec::new();

        let sources = std::mem::take(&mut inner.sources);
        let source_loader = Rc::new(SourceLoader::new(sources));

        for (url, source) in &inner.sources {
            log::trace!("build: {}", url);

            by_url.insert(url.clone(), Default::default());

            let mut sources = rune::Sources::new();
            let input = rune::Source::with_path(
                url.to_string(),
                source.to_string(),
                url.to_file_path().ok(),
            );

            sources.insert(input);

            let mut diagnostics = rune::Diagnostics::new();
            let visitor = Rc::new(Visitor::new(Index::default()));

            let result = rune::load_sources_with_visitor(
                &self.inner.context,
                &self.inner.options,
                &mut sources,
                &mut diagnostics,
                visitor.clone(),
                source_loader.clone(),
            );

            if let Err(rune::LoadSourcesError) = result {
                for diagnostic in diagnostics.diagnostics() {
                    match diagnostic {
                        Diagnostic::Fatal(fatal) => {
                            let source_id = fatal.source_id();

                            match fatal.kind() {
                                FatalDiagnosticKind::ParseError(error) => {
                                    report(
                                        &sources,
                                        &mut by_url,
                                        error.span(),
                                        source_id,
                                        error,
                                        display_to_error,
                                    );
                                }
                                FatalDiagnosticKind::CompileError(error) => {
                                    report(
                                        &sources,
                                        &mut by_url,
                                        error.span(),
                                        source_id,
                                        error,
                                        display_to_error,
                                    );
                                }
                                FatalDiagnosticKind::QueryError(error) => {
                                    report(
                                        &sources,
                                        &mut by_url,
                                        error.span(),
                                        source_id,
                                        error,
                                        display_to_error,
                                    );
                                }
                                FatalDiagnosticKind::LinkError(error) => match error {
                                    LinkerError::MissingFunction { hash, spans } => {
                                        for (span, _) in spans {
                                            let diagnostics =
                                                by_url.entry(url.clone()).or_default();

                                            let range = source.span_to_lsp_range(*span);

                                            diagnostics.push(display_to_error(
                                                range,
                                                format!("missing function with hash `{}`", hash),
                                            ));
                                        }
                                    }
                                    error => {
                                        let diagnostics = by_url.entry(url.clone()).or_default();
                                        let range = lsp::Range::default();
                                        diagnostics.push(display_to_error(range, error));
                                    }
                                },
                                FatalDiagnosticKind::Internal(message) => {
                                    let diagnostics = by_url.entry(url.clone()).or_default();
                                    let range = lsp::Range::default();
                                    diagnostics.push(display_to_error(range, message));
                                }
                                error => {
                                    let diagnostics = by_url.entry(url.clone()).or_default();
                                    let range = lsp::Range::default();
                                    diagnostics.push(display_to_error(range, error));
                                }
                            }
                        }
                        Diagnostic::Warning(warning) => {
                            report(
                                &sources,
                                &mut by_url,
                                warning.span(),
                                warning.source_id(),
                                warning.kind(),
                                display_to_warning,
                            );
                        }
                    }
                }
            }

            let visitor = match Rc::try_unwrap(visitor) {
                Ok(visitor) => visitor,
                Err(..) => panic!("visitor should be uniquely held"),
            };

            builds.push((url.clone(), sources, visitor.into_index()));
        }

        let source_loader = match Rc::try_unwrap(source_loader) {
            Ok(source_loader) => source_loader,
            Err(..) => panic!("source loader should be uniquely held"),
        };

        inner.sources = source_loader.into_sources();

        for (url, build_sources, index) in builds {
            if let Some(source) = inner.sources.get_mut(&url) {
                source.index = index;
                source.build_sources = Some(build_sources);
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
    /// Sender to indicate interest in rebuilding the project.
    /// Can be triggered on modification.
    rebuild_tx: mpsc::Sender<()>,
    /// The rune context to build for.
    context: rune::Context,
    /// Build options.
    options: Options,
    /// Indicate if the server is initialized.
    initialized: AtomicBool,
    /// Sources used in the project.
    sources: RwLock<Sources>,
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
            build_sources: None,
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
    /// Loaded Rune sources for this source file. Will be present after the
    /// source file has been built.
    build_sources: Option<rune::Sources>,
}

impl Source {
    /// Find the definition at the given span.
    pub fn find_definition_at(&self, span: Span) -> Option<&Definition> {
        let (found_span, definition) = self.index.definitions.range(..=span).rev().next()?;

        if span.start >= found_span.start && span.end <= found_span.end {
            log::trace!("found {:?}", definition);
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
        let start = self.offset_to_lsp_position(span.start.into_usize());
        let end = self.offset_to_lsp_position(span.end.into_usize());

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

        lsp::Position::new(line as u32, col_char as u32)
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

/// Convert the given span into an lsp range.
fn span_to_lsp_range(source: &rune::Source, span: Span) -> Option<lsp::Range> {
    let (line, character) = source.position_to_utf16cu_line_char(span.start.into_usize())?;
    let start = lsp::Position::new(line as u32, character as u32);
    let (line, character) = source.position_to_utf16cu_line_char(span.end.into_usize())?;
    let end = lsp::Position::new(line as u32, character as u32);
    Some(lsp::Range::new(start, end))
}

/// Translate the given lsp::Position, which is in UTF-16 because Microsoft.
///
/// Please go complain here:
/// <https://github.com/microsoft/language-server-protocol/issues/376>
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
    source_id: SourceId,
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

    let url = match source.path() {
        Some(path) => match Url::from_file_path(path) {
            Ok(url) => url,
            Err(()) => return,
        },
        None => return,
    };

    let range = match span_to_lsp_range(&*source, span) {
        Some(range) => range,
        None => return,
    };

    let diagnostics = by_url.entry(url).or_default();
    diagnostics.push(report(range, error));
}

/// Convert the given span and error into an error diagnostic.
fn display_to_error<E>(range: lsp::Range, error: E) -> lsp::Diagnostic
where
    E: fmt::Display,
{
    display_to_diagnostic(range, error, lsp::DiagnosticSeverity::ERROR)
}

/// Convert the given span and error into a warning diagnostic.
fn display_to_warning<E>(range: lsp::Range, error: E) -> lsp::Diagnostic
where
    E: fmt::Display,
{
    display_to_diagnostic(range, error, lsp::DiagnosticSeverity::WARNING)
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
        code_description: None,
        source: None,
        message: error.to_string(),
        related_information: None,
        tags: None,
        data: None,
    }
}

#[derive(Default)]
pub struct Index {
    /// Spans mapping to their corresponding definitions.
    definitions: BTreeMap<Span, Definition>,
}

/// A definition source.
#[derive(Debug, Clone)]
pub enum DefinitionSource {
    /// Only a file source.
    Source(SourceId),
    /// A location definition (source and span).
    Location(Location),
    /// A complete compile source.
    CompileSource(CompileSource),
}

impl DefinitionSource {
    fn span(&self) -> Span {
        match self {
            Self::Source(..) => Span::empty(),
            Self::Location(location) => location.span,
            Self::CompileSource(compile_source) => compile_source.location.span,
        }
    }

    fn source_id(&self) -> SourceId {
        match self {
            Self::Source(source_id) => *source_id,
            Self::Location(location) => location.source_id,
            Self::CompileSource(compile_source) => compile_source.location.source_id,
        }
    }

    fn path(&self) -> Option<&Path> {
        match self {
            Self::CompileSource(compile_source) => compile_source.path.as_deref(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Definition {
    /// The kind of the definition.
    pub(crate) kind: DefinitionKind,
    /// The id of the source id the definition corresponds to.
    pub(crate) source: DefinitionSource,
}

#[derive(Debug, Clone, Copy)]
pub enum DefinitionKind {
    /// A unit struct.
    UnitStruct,
    /// A tuple struct.
    TupleStruct,
    /// A struct.
    Struct,
    /// A unit variant.
    UnitVariant,
    /// A tuple variant.
    TupleVariant,
    /// A struct variant.
    StructVariant,
    /// An enum.
    Enum,
    /// A function.
    Function,
    /// A local variable.
    Local,
    /// A module that can be jumped to.
    Module,
}

struct Visitor {
    index: RefCell<Index>,
}

impl Visitor {
    /// Construct a new visitor.
    pub fn new(index: Index) -> Self {
        Self {
            index: RefCell::new(index),
        }
    }

    /// Convert visitor back into an index.
    pub fn into_index(self) -> Index {
        self.index.into_inner()
    }
}

impl CompileVisitor for Visitor {
    fn visit_meta(&self, source_id: SourceId, meta: &CompileMeta, span: Span) {
        if source_id.into_index() != 0 {
            return;
        }

        let source = match meta.source.as_ref() {
            Some(source) => source,
            None => return,
        };

        let kind = match &meta.kind {
            CompileMetaKind::UnitStruct { .. } => DefinitionKind::UnitStruct,
            CompileMetaKind::TupleStruct { .. } => DefinitionKind::TupleStruct,
            CompileMetaKind::Struct { .. } => DefinitionKind::Struct,
            CompileMetaKind::UnitVariant { .. } => DefinitionKind::UnitVariant,
            CompileMetaKind::TupleVariant { .. } => DefinitionKind::TupleVariant,
            CompileMetaKind::StructVariant { .. } => DefinitionKind::StructVariant,
            CompileMetaKind::Enum { .. } => DefinitionKind::Enum,
            CompileMetaKind::Function { .. } => DefinitionKind::Function,
            _ => return,
        };

        let definition = Definition {
            kind,
            source: DefinitionSource::CompileSource(source.clone()),
        };

        if let Some(d) = self.index.borrow_mut().definitions.insert(span, definition) {
            log::warn!("replaced definition: {:?}", d.kind)
        }
    }

    fn visit_variable_use(&self, source_id: SourceId, var_span: Span, span: Span) {
        if source_id.into_index() != 0 {
            return;
        }

        let definition = Definition {
            kind: DefinitionKind::Local,
            source: DefinitionSource::Location(Location::new(source_id, var_span)),
        };

        if let Some(d) = self.index.borrow_mut().definitions.insert(span, definition) {
            log::warn!("replaced definition: {:?}", d.kind)
        }
    }

    fn visit_mod(&self, source_id: SourceId, span: Span) {
        if source_id.into_index() != 0 {
            return;
        }

        let definition = Definition {
            kind: DefinitionKind::Module,
            source: DefinitionSource::Source(source_id),
        };

        if let Some(d) = self.index.borrow_mut().definitions.insert(span, definition) {
            log::warn!("replaced definition: {:?}", d.kind)
        }
    }
}

struct SourceLoader {
    sources: RefCell<HashMap<Url, Source>>,
    base: FileSourceLoader,
}

impl SourceLoader {
    /// Construct a new source loader.
    pub fn new(sources: HashMap<Url, Source>) -> Self {
        Self {
            sources: RefCell::new(sources),
            base: FileSourceLoader::new(),
        }
    }

    /// Convert into sources.
    fn into_sources(self) -> HashMap<Url, Source> {
        self.sources.into_inner()
    }

    /// Generate a collection of URl candidates.
    fn candidates(root: &Path, item: &Item) -> Option<[Url; 2]> {
        let mut base = root.to_owned();

        let mut it = item.iter().peekable();
        let mut last = None;

        while let Some(c) = it.next() {
            if it.peek().is_none() {
                last = match c {
                    ComponentRef::Str(string) => Some(string),
                    _ => return None,
                };

                break;
            }

            if let ComponentRef::Str(string) = c {
                base.push(string);
            } else {
                return None;
            }
        }

        let last = last?;

        let mut a = base.clone();
        a.push(&format!("{}.rn", last));

        let mut b = base;
        b.push(last);
        b.push("mod.rn");

        let a = Url::from_file_path(&a).ok()?;
        let b = Url::from_file_path(&b).ok()?;

        Some([a, b])
    }
}

impl rune::compile::SourceLoader for SourceLoader {
    fn load(&self, root: &Path, item: &Item, span: Span) -> Result<rune::Source, CompileError> {
        log::trace!("load {} (root: {})", item, root.display());

        if let Some(candidates) = Self::candidates(root, item) {
            for url in candidates.iter() {
                if let Some(s) = self.sources.borrow().get(url) {
                    return Ok(rune::Source::new(url.to_string(), s.to_string()));
                }
            }
        }

        self.base.load(root, item, span)
    }
}
