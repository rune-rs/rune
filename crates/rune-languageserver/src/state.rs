use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Context as _, Result};
use hashbrown::HashMap;
use lsp::Url;
use ropey::Rope;
use rune::ast::{Span, Spanned};
use rune::compile::{
    meta, CompileError, CompileVisitor, ComponentRef, FileSourceLoader, Item, LinkerError,
    Location, MetaRef, SourceMeta,
};
use rune::diagnostics::{Diagnostic, FatalDiagnosticKind};
use rune::{Context, Options, SourceId};
use tokio::sync::Notify;

use crate::Output;

#[derive(Default)]
struct Build {
    id_to_url: HashMap<SourceId, Url>,
    sources: rune::Sources,
}

/// Shared server state.
pub struct State<'a> {
    /// The output abstraction.
    pub(crate) output: Output,
    /// Sender to indicate interest in rebuilding the project.
    /// Can be triggered on modification.
    rebuild_notify: &'a Notify,
    /// The rune context to build for.
    context: rune::Context,
    /// Build options.
    options: Options,
    /// Indicate if the server is initialized.
    initialized: bool,
    /// Indicate that the server is stopped.
    stopped: bool,
    /// Sources used in the project.
    workspace: Workspace,
}

impl<'a> State<'a> {
    /// Construct a new state.
    pub fn new(
        output: Output,
        rebuild_notify: &'a Notify,
        context: Context,
        options: Options,
    ) -> Self {
        Self {
            output,
            rebuild_notify,
            context,
            options,
            initialized: bool::default(),
            stopped: bool::default(),
            workspace: Workspace::default(),
        }
    }

    /// Mark server as initialized.
    pub fn initialize(&mut self) {
        self.initialized = true;
    }

    /// Test if server is initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Mark server as stopped.
    pub fn stop(&mut self) {
        self.stopped = true;
    }

    /// Test if server is stopped.
    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    /// Get mutable access to the workspace.
    pub fn workspace_mut(&mut self) -> &mut Workspace {
        &mut self.workspace
    }

    /// Indicate interest in having the project rebuild.
    ///
    /// Sources that have been modified will be marked as dirty.
    pub fn rebuild_interest(&self) {
        self.rebuild_notify.notify_one();
    }

    /// Find definition at the given uri and LSP position.
    pub async fn goto_definition(
        &self,
        uri: &Url,
        position: lsp::Position,
    ) -> Option<lsp::Location> {
        let source = self.workspace.get(uri)?;
        let offset = source.lsp_position_to_offset(position);
        let def = source.find_definition_at(Span::point(offset))?;

        let url = match def.source.path() {
            Some(path) => crate::url::from_file_path(path).ok()?,
            None => uri.clone(),
        };

        let source = source.build_sources.as_ref()?.get(def.source.source_id())?;

        let (l, c) = source.pos_to_utf16cu_linecol(def.source.span().start.into_usize());
        let start = lsp::Position {
            line: l as u32,
            character: c as u32,
        };

        let (l, c) = source.pos_to_utf16cu_linecol(def.source.span().end.into_usize());
        let end = lsp::Position {
            line: l as u32,
            character: c as u32,
        };

        let range = lsp::Range { start, end };

        let location = lsp::Location { uri: url, range };

        tracing::trace!("go to location: {:?}", location);
        Some(location)
    }

    /// Rebuild the project.
    pub async fn rebuild(&mut self) -> Result<()> {
        let mut by_url = HashMap::<Url, Vec<lsp::Diagnostic>>::new();

        for url in self.workspace.removed.drain(..) {
            by_url.insert(url.clone(), Vec::new());
        }

        let mut source_loader = SourceLoader::new(&self.workspace.sources);

        let mut workspace_build = Build::default();
        let mut scripts_build = Build::default();

        let mut workspace_diagnostics = rune::workspace::Diagnostics::default();

        if let Some((workspace_url, workspace_path)) = &self.workspace.manifest_path {
            if let Err(error) = load_workspace(
                workspace_url,
                workspace_path,
                &mut workspace_build,
                &mut workspace_diagnostics,
                &self.workspace,
                &mut scripts_build,
                &mut by_url,
            ) {
                tracing::error!("error loading workspace: {error}");

                for error in error.chain().skip(1) {
                    tracing::error!("caused by: {error}");
                }
            }
        } else {
            for (url, source) in &self.workspace.sources {
                tracing::trace!("build plain source: {}", url);
                let input =
                    rune::Source::with_path(url, source.to_string(), url.to_file_path().ok());
                let id = scripts_build.sources.insert(input);
                scripts_build.id_to_url.insert(id, url.clone());
                by_url.insert(url.clone(), Vec::default());
            }
        }

        let mut diagnostics = rune::Diagnostics::new();
        let mut visitor = Visitor::default();

        let _ = rune::prepare(&mut scripts_build.sources)
            .with_context(&self.context)
            .with_diagnostics(&mut diagnostics)
            .with_options(&self.options)
            .with_visitor(&mut visitor)
            .with_source_loader(&mut source_loader)
            .build();

        emit_workspace(workspace_diagnostics, &workspace_build, &mut by_url);
        emit_scripts(diagnostics, &scripts_build, &mut by_url);

        let sources = Arc::new(scripts_build.sources);

        for (source_id, value) in visitor.into_indexes() {
            let Some(url) = scripts_build.id_to_url.get(&source_id) else {
                continue;
            };

            let Some(source) = self.workspace.sources.get_mut(url) else {
                continue;
            };

            source.index = value;
            source.build_sources = Some(sources.clone());
        }

        for (url, diagnostics) in by_url {
            let diagnostics = lsp::PublishDiagnosticsParams {
                uri: url.clone(),
                diagnostics,
                version: None,
            };

            tracing::info!(url = ?url.to_string(), ?diagnostics, "diagnostic");

            self.output
                .notification::<lsp::notification::PublishDiagnostics>(diagnostics)
                .await?;
        }

        Ok(())
    }
}

/// Try to load workspace.
fn load_workspace(
    url: &Url,
    path: &Path,
    workspace_build: &mut Build,
    workspace_diagnostics: &mut rune::workspace::Diagnostics,
    workspace: &Workspace,
    scripts_build: &mut Build,
    diags: &mut HashMap<Url, Vec<lsp::Diagnostic>>,
) -> Result<(), anyhow::Error> {
    tracing::info!(url = ?url.to_string(), "building workspace");

    let source = match workspace.sources.get(url) {
        Some(source) => source.chunks().collect::<String>(),
        None => std::fs::read_to_string(path).with_context(|| url.to_string())?,
    };

    diags.insert(url.clone(), Vec::default());

    let entry_id = workspace_build
        .sources
        .insert(rune::Source::with_path(url, source, Some(path)));
    workspace_build.id_to_url.insert(entry_id, url.clone());

    let manifest = rune::workspace::prepare(&mut workspace_build.sources)
        .with_diagnostics(workspace_diagnostics)
        .build()?;

    let output = manifest.find_all(rune::workspace::WorkspaceFilter::All)?;

    for found in output {
        let Ok(url) = crate::url::from_file_path(&found.path) else {
            continue;
        };

        tracing::trace!("build manifest source: {}", url);

        let source = match workspace.sources.get(&url) {
            Some(source) => source.chunks().collect::<String>(),
            None => std::fs::read_to_string(&found.path)
                .with_context(|| found.path.display().to_string())?,
        };

        let input = rune::Source::with_path(&url, source, Some(found.path));
        let id = scripts_build.sources.insert(input);
        workspace_build.id_to_url.insert(id, url.clone());
        diags.insert(url.clone(), Vec::default());
    }

    Ok(())
}

/// Emit diagnostics workspace.
fn emit_workspace(
    diagnostics: rune::workspace::Diagnostics,
    build: &Build,
    by_url: &mut HashMap<Url, Vec<lsp::Diagnostic>>,
) {
    for diagnostic in diagnostics.diagnostics() {
        tracing::trace!("diagnostic: {:?}", diagnostic);

        if let rune::workspace::Diagnostic::Fatal(f) = diagnostic {
            report(build, by_url, f.source_id(), f.error(), to_error);
        }
    }
}

/// Emit regular compile diagnostics.
fn emit_scripts(
    diagnostics: rune::Diagnostics,
    build: &Build,
    by_url: &mut HashMap<Url, Vec<lsp::Diagnostic>>,
) {
    for diagnostic in diagnostics.diagnostics() {
        tracing::trace!("diagnostic: {:?}", diagnostic);

        match diagnostic {
            Diagnostic::Fatal(f) => match f.kind() {
                FatalDiagnosticKind::ParseError(e) => {
                    report(build, by_url, f.source_id(), e, to_error);
                }
                FatalDiagnosticKind::CompileError(e) => {
                    report(build, by_url, f.source_id(), e, to_error);
                }
                FatalDiagnosticKind::QueryError(e) => {
                    report(build, by_url, f.source_id(), e, to_error);
                }
                FatalDiagnosticKind::LinkError(e) => match e {
                    LinkerError::MissingFunction { hash, spans } => {
                        for (span, source_id) in spans {
                            let (Some(url), Some(source)) = (build.id_to_url.get(source_id), build.sources.get(*source_id)) else {
                                continue;
                            };

                            let Some(range) = span_to_lsp_range(source, *span) else {
                                continue;
                            };

                            let diagnostics = by_url.entry(url.clone()).or_default();

                            diagnostics.push(to_error(
                                range,
                                format!("missing function with hash `{}`", hash),
                            ));
                        }
                    }
                    e => {
                        report_without_span(build, by_url, f.source_id(), e, to_error);
                    }
                },
                FatalDiagnosticKind::Internal(e) => {
                    report_without_span(build, by_url, f.source_id(), e, to_error);
                }
                e => {
                    report_without_span(build, by_url, f.source_id(), e, to_error);
                }
            },
            Diagnostic::Warning(e) => {
                report(build, by_url, e.source_id(), e, to_warning);
            }
            _ => {}
        }
    }
}

/// A collection of open sources.
#[derive(Default)]
pub struct Workspace {
    /// Found workspace root.
    pub(crate) manifest_path: Option<(Url, PathBuf)>,
    /// Sources that might be modified.
    sources: HashMap<Url, Source>,
    /// A source that has been removed.
    removed: Vec<Url>,
}

impl Workspace {
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
        if self.sources.remove(url).is_some() {
            self.removed.push(url.clone());
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
    build_sources: Option<Arc<rune::Sources>>,
}

impl Source {
    /// Find the definition at the given span.
    pub fn find_definition_at(&self, span: Span) -> Option<&Definition> {
        let (found_span, definition) = self.index.definitions.range(..=span).rev().next()?;

        if span.start >= found_span.start && span.end <= found_span.end {
            tracing::trace!("found {:?}", definition);
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
    let (line, character) = source.pos_to_utf16cu_linecol(span.start.into_usize());
    let start = lsp::Position::new(line as u32, character as u32);
    let (line, character) = source.pos_to_utf16cu_linecol(span.end.into_usize());
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
    build: &Build,
    by_url: &mut HashMap<Url, Vec<lsp::Diagnostic>>,
    source_id: SourceId,
    error: E,
    report: R,
) where
    E: fmt::Display,
    E: Spanned,
    R: Fn(lsp::Range, E) -> lsp::Diagnostic,
{
    let span = error.span();

    let (Some(source), Some(url)) = (build.sources.get(source_id), build.id_to_url.get(&source_id)) else {
        return;
    };

    let Some(range) = span_to_lsp_range(source, span) else {
        return;
    };

    let diagnostics = by_url.entry(url.clone()).or_default();
    diagnostics.push(report(range, error));
}

/// Convert the given span and error into an error diagnostic.
fn report_without_span<E, R>(
    build: &Build,
    by_url: &mut HashMap<Url, Vec<lsp::Diagnostic>>,
    source_id: SourceId,
    error: E,
    report: R,
) where
    E: fmt::Display,
    R: Fn(lsp::Range, E) -> lsp::Diagnostic,
{
    let Some(url) = build.id_to_url.get(&source_id) else {
        return;
    };

    let range = lsp::Range::default();
    let diagnostics = by_url.entry(url.clone()).or_default();
    diagnostics.push(report(range, error));
}

/// Convert the given span and error into an error diagnostic.
fn to_error<E>(range: lsp::Range, error: E) -> lsp::Diagnostic
where
    E: fmt::Display,
{
    display_to_diagnostic(range, error, lsp::DiagnosticSeverity::ERROR)
}

/// Convert the given span and error into a warning diagnostic.
fn to_warning<E>(range: lsp::Range, error: E) -> lsp::Diagnostic
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
    SourceMeta(SourceMeta),
}

impl DefinitionSource {
    fn span(&self) -> Span {
        match self {
            Self::Source(..) => Span::empty(),
            Self::Location(location) => location.span,
            Self::SourceMeta(compile_source) => compile_source.location.span,
        }
    }

    fn source_id(&self) -> SourceId {
        match self {
            Self::Source(source_id) => *source_id,
            Self::Location(location) => location.source_id,
            Self::SourceMeta(compile_source) => compile_source.location.source_id,
        }
    }

    fn path(&self) -> Option<&Path> {
        match self {
            Self::SourceMeta(compile_source) => compile_source.path.as_deref(),
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

#[derive(Default)]
struct Visitor {
    indexes: HashMap<SourceId, Index>,
}

impl Visitor {
    /// Convert visitor back into an index.
    pub fn into_indexes(self) -> HashMap<SourceId, Index> {
        self.indexes
    }
}

impl CompileVisitor for Visitor {
    fn visit_meta(&mut self, location: Location, meta: MetaRef<'_>) {
        let source = match meta.source {
            Some(source) => source,
            None => return,
        };

        let kind = match &meta.kind {
            meta::Kind::Struct {
                variant: meta::Variant::Unit,
                ..
            } => DefinitionKind::UnitStruct,
            meta::Kind::Struct {
                variant: meta::Variant::Tuple(..),
                ..
            } => DefinitionKind::TupleStruct,
            meta::Kind::Struct {
                variant: meta::Variant::Struct(..),
                ..
            } => DefinitionKind::Struct,
            meta::Kind::Variant {
                variant: meta::Variant::Unit,
                ..
            } => DefinitionKind::UnitVariant,
            meta::Kind::Variant {
                variant: meta::Variant::Tuple(..),
                ..
            } => DefinitionKind::TupleVariant,
            meta::Kind::Variant {
                variant: meta::Variant::Struct(..),
                ..
            } => DefinitionKind::StructVariant,
            meta::Kind::Enum { .. } => DefinitionKind::Enum,
            meta::Kind::Function { .. } => DefinitionKind::Function,
            _ => return,
        };

        let definition = Definition {
            kind,
            source: DefinitionSource::SourceMeta(source.clone()),
        };

        let index = self.indexes.entry(location.source_id).or_default();

        if let Some(d) = index.definitions.insert(location.span, definition) {
            tracing::warn!("replaced definition: {:?}", d.kind)
        }
    }

    fn visit_variable_use(&mut self, source_id: SourceId, var_span: Span, span: Span) {
        let definition = Definition {
            kind: DefinitionKind::Local,
            source: DefinitionSource::Location(Location::new(source_id, var_span)),
        };

        let index = self.indexes.entry(source_id).or_default();

        if let Some(d) = index.definitions.insert(span, definition) {
            tracing::warn!("replaced definition: {:?}", d.kind)
        }
    }

    fn visit_mod(&mut self, source_id: SourceId, span: Span) {
        let definition = Definition {
            kind: DefinitionKind::Module,
            source: DefinitionSource::Source(source_id),
        };

        let index = self.indexes.entry(source_id).or_default();

        if let Some(d) = index.definitions.insert(span, definition) {
            tracing::warn!("replaced definition: {:?}", d.kind)
        }
    }
}

struct SourceLoader<'a> {
    sources: &'a HashMap<Url, Source>,
    base: FileSourceLoader,
}

impl<'a> SourceLoader<'a> {
    /// Construct a new source loader.
    pub fn new(sources: &'a HashMap<Url, Source>) -> Self {
        Self {
            sources,
            base: FileSourceLoader::new(),
        }
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

        let a = crate::url::from_file_path(&a).ok()?;
        let b = crate::url::from_file_path(&b).ok()?;

        Some([a, b])
    }
}

impl<'a> rune::compile::SourceLoader for SourceLoader<'a> {
    fn load(&mut self, root: &Path, item: &Item, span: Span) -> Result<rune::Source, CompileError> {
        tracing::trace!("load {} (root: {})", item, root.display());

        if let Some(candidates) = Self::candidates(root, item) {
            for url in candidates.iter() {
                if let Some(s) = self.sources.get(url) {
                    return Ok(rune::Source::new(url, s.to_string()));
                }
            }
        }

        self.base.load(root, item, span)
    }
}
