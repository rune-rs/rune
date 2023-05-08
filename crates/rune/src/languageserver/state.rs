use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::no_std::prelude::*;

use anyhow::{anyhow, Context as _, Result};
use lsp::Url;
use ropey::Rope;
use tokio::sync::Notify;

use crate::ast::{Span, Spanned};
use crate::collections::HashMap;
use crate::compile::meta;
use crate::compile::{
    self, CompileVisitor, ComponentRef, Item, ItemBuf, LinkerError, Location, MetaRef, SourceMeta,
};
use crate::diagnostics::{Diagnostic, FatalDiagnosticKind};
use crate::doc::VisitorData;
use crate::languageserver::connection::Output;
use crate::languageserver::Language;
use crate::workspace::{self, WorkspaceError};
use crate::{BuildError, Context, Options, SourceId, Unit};

#[derive(Default)]
struct Reporter {
    by_url: BTreeMap<Url, Vec<lsp::Diagnostic>>,
}

impl Reporter {
    /// Ensure that the given URL is being reporter.
    fn ensure(&mut self, url: &Url) {
        if !self.by_url.contains_key(url) {
            self.by_url.insert(url.clone(), Vec::new());
        }
    }

    /// Get entry for the given URL.
    fn entry(&mut self, url: &Url) -> &mut Vec<lsp::Diagnostic> {
        self.by_url.entry(url.clone()).or_default()
    }
}

#[derive(Default)]
struct Build {
    id_to_url: HashMap<SourceId, Url>,
    sources: crate::Sources,
}

impl Build {
    pub(super) fn populate(&mut self, reporter: &mut Reporter) {
        for id in self.sources.source_ids() {
            let Some(source) = self.sources.get(id) else {
                continue;
            };

            let Some(path) = source.path() else {
                continue;
            };

            let Ok(url) = crate::languageserver::url::from_file_path(path) else {
                continue;
            };

            reporter.ensure(&url);
            self.id_to_url.insert(id, url);
        }
    }

    pub(super) fn visit(&mut self, visited: &mut HashSet<Url>) {
        for id in self.sources.source_ids() {
            let Some(source) = self.sources.get(id) else {
                continue;
            };

            let Some(path) = source.path() else {
                continue;
            };

            let Ok(url) = crate::languageserver::url::from_file_path(path) else {
                continue;
            };

            visited.insert(url.clone());
        }
    }
}

/// Shared server state.
pub(super) struct State<'a> {
    /// The output abstraction.
    pub(super) output: Output,
    /// Sender to indicate interest in rebuilding the project.
    /// Can be triggered on modification.
    rebuild_notify: &'a Notify,
    /// The rune context to build for.
    context: crate::Context,
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
    pub(super) fn new(
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
    pub(super) fn initialize(&mut self) {
        self.initialized = true;
    }

    /// Test if server is initialized.
    pub(super) fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Mark server as stopped.
    pub(super) fn stop(&mut self) {
        self.stopped = true;
    }

    /// Test if server is stopped.
    pub(super) fn is_stopped(&self) -> bool {
        self.stopped
    }

    /// Get mutable access to the workspace.
    pub(super) fn workspace_mut(&mut self) -> &mut Workspace {
        &mut self.workspace
    }

    /// Indicate interest in having the project rebuild.
    ///
    /// Sources that have been modified will be marked as dirty.
    pub(super) fn rebuild_interest(&self) {
        self.rebuild_notify.notify_one();
    }

    /// Find definition at the given uri and LSP position.
    pub(super) async fn goto_definition(
        &self,
        uri: &Url,
        position: lsp::Position,
    ) -> Option<lsp::Location> {
        let source = self.workspace.get(uri)?;
        let offset = source.lsp_position_to_offset(position);
        let def = source.find_definition_at(Span::point(offset))?;

        let url = match def.source.path() {
            Some(path) => crate::languageserver::url::from_file_path(path).ok()?,
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

    /// Find definition at the given uri and LSP position.
    #[tracing::instrument(skip_all)]
    pub(super) fn complete(
        &self,
        uri: &Url,
        position: lsp::Position,
    ) -> Option<Vec<lsp::CompletionItem>> {
        let sources = &self.workspace.sources;
        tracing::trace!(uri = ?uri, uri_exists = sources.get(uri).is_some());
        let workspace_source = sources.get(uri)?;

        let offset = workspace_source.lsp_position_to_offset(position);

        let (mut symbol, start) = workspace_source.looking_back(offset)?;
        tracing::trace!(symbol = ?symbol, start = ?start);

        if symbol.is_empty() {
            return None;
        }

        let mut results = vec![];

        let can_use_instance_fn: &[_] = &['.'];
        let first_char = symbol.remove(0);
        let symbol = symbol.trim();

        if let Some(unit) = workspace_source.unit.as_ref() {
            super::completion::complete_for_unit(
                workspace_source,
                unit,
                symbol,
                position,
                &mut results,
            );
        }

        if first_char.is_ascii_alphabetic() || can_use_instance_fn.contains(&first_char) {
            super::completion::complete_native_instance_data(
                &self.context,
                symbol,
                position,
                &mut results,
            );
        } else {
            super::completion::complete_native_loose_data(
                &self.context,
                symbol,
                position,
                &mut results,
            );
        }

        Some(results)
    }

    /// Rebuild the project.
    pub(super) async fn rebuild(&mut self) -> Result<()> {
        // Keep track of URLs visited as part of workspace builds.
        let mut visited = HashSet::new();
        // Workspace results.
        let mut workspace_results = Vec::new();
        // Build results.
        let mut script_results = Vec::new();
        // Emitted diagnostics, grouped by URL.
        let mut reporter = Reporter::default();

        if let Some((workspace_url, workspace_path)) = &self.workspace.manifest_path {
            let mut diagnostics = workspace::Diagnostics::default();
            let mut build = Build::default();

            let result = self.load_workspace(
                workspace_url,
                workspace_path,
                &mut build,
                &mut diagnostics,
                &self.workspace,
            );

            match result {
                Err(error) => {
                    tracing::error!("error loading workspace: {error}");

                    for error in error.chain().skip(1) {
                        tracing::error!("caused by: {error}");
                    }
                }
                Ok(script_builds) => {
                    for script_build in script_builds {
                        script_results.push(self.build_scripts(script_build, Some(&mut visited)));
                    }
                }
            };

            workspace_results.push((diagnostics, build));
        }

        for (url, source) in &self.workspace.sources {
            if visited.contains(url) {
                tracing::trace!(url = url.to_string(), "already populated by workspace");
                continue;
            }

            if !matches!(source.language, Language::Rune) {
                continue;
            }

            tracing::trace!(url = url.to_string(), "build plain source");

            let mut build = Build::default();

            let input = match url.to_file_path() {
                Ok(path) => crate::Source::with_path(url, source.to_string(), path),
                Err(..) => crate::Source::new(url, source.to_string()),
            };

            build.sources.insert(input);
            script_results.push(self.build_scripts(build, None));
        }

        // We need to pupulate diagnostics for everything we know about, in
        // order to clear errors which might've previously been set.
        for url in self.workspace.removed.drain(..) {
            reporter.ensure(&url);
        }

        for (diagnostics, mut build) in workspace_results {
            build.populate(&mut reporter);
            emit_workspace(diagnostics, &build, &mut reporter);
        }

        for (diagnostics, mut build, source_visitor, doc_visitor, unit) in script_results {
            build.populate(&mut reporter);
            emit_scripts(diagnostics, &build, &mut reporter);

            let sources = Arc::new(build.sources);
            let doc_visitor = Arc::new(doc_visitor);

            for (source_id, value) in source_visitor.into_indexes() {
                let Some(url) = build.id_to_url.get(&source_id) else {
                    continue;
                };

                let Some(source) = self.workspace.sources.get_mut(url) else {
                    continue;
                };

                source.index = value;
                source.build_sources = Some(sources.clone());

                if let Ok(unit) = unit.as_ref().map(|v| v.clone()) {
                    source.unit = Some(unit.clone());
                }

                source.docs = Some(doc_visitor.clone());
            }
        }

        for (url, diagnostics) in reporter.by_url {
            tracing::info!(url = ?url.to_string(), diagnostics = diagnostics.len(), "publishing diagnostics");

            let diagnostics = lsp::PublishDiagnosticsParams {
                uri: url.clone(),
                diagnostics,
                version: None,
            };

            self.output
                .notification::<lsp::notification::PublishDiagnostics>(diagnostics)
                .await?;
        }

        Ok(())
    }

    /// Try to load workspace.
    fn load_workspace(
        &self,
        url: &Url,
        path: &Path,
        manifest_build: &mut Build,
        diagnostics: &mut workspace::Diagnostics,
        workspace: &Workspace,
    ) -> Result<Vec<Build>, anyhow::Error> {
        tracing::info!(url = ?url.to_string(), "building workspace");

        let source = match workspace.sources.get(url) {
            Some(source) => source.chunks().collect::<String>(),
            None => std::fs::read_to_string(path).with_context(|| url.to_string())?,
        };

        manifest_build
            .sources
            .insert(crate::Source::with_path(url, source, path));

        let mut source_loader = WorkspaceSourceLoader::new(&self.workspace.sources);

        let manifest = workspace::prepare(&mut manifest_build.sources)
            .with_diagnostics(diagnostics)
            .with_source_loader(&mut source_loader)
            .build()?;

        let mut script_builds = Vec::new();

        for p in manifest.find_all(workspace::WorkspaceFilter::All)? {
            let Ok(url) = crate::languageserver::url::from_file_path(&p.found.path) else {
                continue;
            };

            tracing::trace!("found manifest source: {}", url);

            let source = match workspace.sources.get(&url) {
                Some(source) => source.chunks().collect::<String>(),
                None => std::fs::read_to_string(&p.found.path)
                    .with_context(|| p.found.path.display().to_string())?,
            };

            let mut build = Build::default();
            build
                .sources
                .insert(crate::Source::with_path(&url, source, p.found.path));

            script_builds.push(build);
        }

        Ok(script_builds)
    }

    fn build_scripts(
        &self,
        mut build: Build,
        built: Option<&mut HashSet<Url>>,
    ) -> (
        crate::Diagnostics,
        Build,
        Visitor,
        crate::doc::Visitor,
        Result<Unit, BuildError>,
    ) {
        let mut diagnostics = crate::Diagnostics::new();
        let mut source_visitor = Visitor::default();
        let mut doc_visitor = crate::doc::Visitor::new(ItemBuf::new());

        let mut source_loader = ScriptSourceLoader::new(&self.workspace.sources);

        let unit = crate::prepare(&mut build.sources)
            .with_context(&self.context)
            .with_diagnostics(&mut diagnostics)
            .with_options(&self.options)
            .with_visitor(&mut doc_visitor)
            .with_visitor(&mut source_visitor)
            .with_source_loader(&mut source_loader)
            .build();

        if let Some(built) = built {
            build.visit(built);
        }

        (diagnostics, build, source_visitor, doc_visitor, unit)
    }
}

/// Emit diagnostics workspace.
fn emit_workspace(diagnostics: workspace::Diagnostics, build: &Build, reporter: &mut Reporter) {
    if tracing::enabled!(tracing::Level::TRACE) {
        let id_to_url = build
            .id_to_url
            .iter()
            .map(|(k, v)| (*k, v.to_string()))
            .collect::<HashMap<_, _>>();
        tracing::trace!(?id_to_url, "emitting manifest diagnostics");
    }

    for diagnostic in diagnostics.diagnostics() {
        tracing::trace!(?diagnostic, "workspace diagnostic");

        let workspace::Diagnostic::Fatal(f) = diagnostic;
        report(build, reporter, f.source_id(), f.error(), to_error);
    }
}

/// Emit regular compile diagnostics.
fn emit_scripts(diagnostics: crate::Diagnostics, build: &Build, reporter: &mut Reporter) {
    if tracing::enabled!(tracing::Level::TRACE) {
        let id_to_url = build
            .id_to_url
            .iter()
            .map(|(k, v)| (*k, v.to_string()))
            .collect::<HashMap<_, _>>();

        tracing::trace!(?id_to_url, "emitting script diagnostics");
    }

    for diagnostic in diagnostics.diagnostics() {
        tracing::trace!(?diagnostic, id_to_url = ?build.id_to_url, "script diagnostic");

        match diagnostic {
            Diagnostic::Fatal(f) => match f.kind() {
                FatalDiagnosticKind::CompileError(e) => {
                    report(build, reporter, f.source_id(), e, to_error);
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

                            let diagnostics = reporter.entry(url);

                            diagnostics.push(to_error(
                                range,
                                format!("missing function with hash `{}`", hash),
                            ));
                        }
                    }
                },
                FatalDiagnosticKind::Internal(e) => {
                    report_without_span(build, reporter, f.source_id(), e, to_error);
                }
            },
            Diagnostic::Warning(e) => {
                report(build, reporter, e.source_id(), e, to_warning);
            }
        }
    }
}

/// A collection of open sources.
#[derive(Default)]
pub(super) struct Workspace {
    /// Found workspace root.
    pub(super) manifest_path: Option<(Url, PathBuf)>,
    /// Sources that might be modified.
    sources: HashMap<Url, Source>,
    /// A source that has been removed.
    removed: Vec<Url>,
}

impl Workspace {
    /// Insert the given source at the given url.
    pub(super) fn insert_source(
        &mut self,
        url: Url,
        text: String,
        language: Language,
    ) -> Option<Source> {
        let source = Source {
            content: Rope::from(text),
            index: Default::default(),
            build_sources: None,
            language,
            unit: None,
            docs: None,
        };
        self.sources.insert(url, source)
    }

    /// Get the source at the given url.
    pub(super) fn get(&self, url: &Url) -> Option<&Source> {
        self.sources.get(url)
    }

    /// Get the mutable source at the given url.
    pub(super) fn get_mut(&mut self, url: &Url) -> Option<&mut Source> {
        self.sources.get_mut(url)
    }

    /// Remove the given url as a source.
    pub(super) fn remove(&mut self, url: &Url) {
        if self.sources.remove(url).is_some() {
            self.removed.push(url.clone());
        }
    }
}

/// A single open source.
pub(super) struct Source {
    /// The content of the current source.
    content: Rope,
    /// Indexes used to answer queries.
    index: Index,
    /// Loaded Rune sources for this source file. Will be present after the
    /// source file has been built.
    build_sources: Option<Arc<crate::Sources>>,
    /// The language of the source.
    language: Language,
    /// The compiled unit
    unit: Option<Unit>,
    /// Comments captured
    docs: Option<Arc<crate::doc::Visitor>>,
}

impl Source {
    /// Find the definition at the given span.
    pub(super) fn find_definition_at(&self, span: Span) -> Option<&Definition> {
        let (found_span, definition) = self.index.definitions.range(..=span).rev().next()?;

        if span.start >= found_span.start && span.end <= found_span.end {
            tracing::trace!("found {:?}", definition);
            return Some(definition);
        }

        None
    }

    /// Modify the given lsp range in the file.
    pub(super) fn modify_lsp_range(&mut self, range: lsp::Range, content: &str) -> Result<()> {
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
    pub(super) fn chunks(&self) -> impl Iterator<Item = &str> {
        self.content.chunks()
    }

    /// Returns the best match wordwise when looking back. Note that this will also include the *previous* terminal token.
    pub fn looking_back(&self, offset: usize) -> Option<(String, usize)> {
        let (chunk, start_byte, _, _) = self.content.chunk_at_byte(offset);

        // The set of tokens that delimit symbols.
        let x: &[_] = &[
            ',', ';', '(', '.', '=', '+', '-', '*', '/', '}', '{', ']', '[', ')', ':',
        ];

        let end_search = (offset - start_byte + 1).min(chunk.len());
        chunk[..end_search].rfind(x).map(|looking_back| {
            (
                chunk[looking_back..end_search].trim().to_owned(),
                start_byte + looking_back,
            )
        })
    }

    pub(super) fn get_docs_by_hash(&self, hash: crate::Hash) -> Option<&VisitorData> {
        self.docs.as_ref().and_then(|docs| docs.get_by_hash(hash))
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.content)
    }
}

/// Convert the given span into an lsp range.
fn span_to_lsp_range(source: &crate::Source, span: Span) -> Option<lsp::Range> {
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
fn report<E, R>(build: &Build, reporter: &mut Reporter, source_id: SourceId, error: E, report: R)
where
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

    reporter.entry(url).push(report(range, error));
}

/// Convert the given span and error into an error diagnostic.
fn report_without_span<E, R>(
    build: &Build,
    reporter: &mut Reporter,
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
    let diagnostics = reporter.entry(url);
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
pub(super) struct Index {
    /// Spans mapping to their corresponding definitions.
    definitions: BTreeMap<Span, Definition>,
}

/// A definition source.
#[derive(Debug, Clone)]
pub(super) enum DefinitionSource {
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
pub(super) struct Definition {
    /// The kind of the definition.
    pub(super) kind: DefinitionKind,
    /// The id of the source id the definition corresponds to.
    pub(super) source: DefinitionSource,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum DefinitionKind {
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
    pub(super) fn into_indexes(self) -> HashMap<SourceId, Index> {
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
                fields: meta::Fields::Unit,
                ..
            } => DefinitionKind::UnitStruct,
            meta::Kind::Struct {
                fields: meta::Fields::Tuple(..),
                ..
            } => DefinitionKind::TupleStruct,
            meta::Kind::Struct {
                fields: meta::Fields::Struct(..),
                ..
            } => DefinitionKind::Struct,
            meta::Kind::Variant {
                fields: meta::Fields::Unit,
                ..
            } => DefinitionKind::UnitVariant,
            meta::Kind::Variant {
                fields: meta::Fields::Tuple(..),
                ..
            } => DefinitionKind::TupleVariant,
            meta::Kind::Variant {
                fields: meta::Fields::Struct(..),
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

struct ScriptSourceLoader<'a> {
    sources: &'a HashMap<Url, Source>,
    base: compile::FileSourceLoader,
}

impl<'a> ScriptSourceLoader<'a> {
    /// Construct a new source loader.
    pub(super) fn new(sources: &'a HashMap<Url, Source>) -> Self {
        Self {
            sources,
            base: compile::FileSourceLoader::new(),
        }
    }

    /// Generate a collection of URl candidates.
    fn candidates(root: &Path, item: &Item) -> Option<[(Url, PathBuf); 2]> {
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

        let a_url = crate::languageserver::url::from_file_path(&a).ok()?;
        let b_url = crate::languageserver::url::from_file_path(&b).ok()?;

        Some([(a_url, a), (b_url, b)])
    }
}

impl<'a> crate::compile::SourceLoader for ScriptSourceLoader<'a> {
    fn load(&mut self, root: &Path, item: &Item, span: Span) -> compile::Result<crate::Source> {
        tracing::trace!("load {} (root: {})", item, root.display());

        if let Some(candidates) = Self::candidates(root, item) {
            for (url, path) in candidates {
                if let Some(s) = self.sources.get(&url) {
                    return Ok(crate::Source::with_path(url, s.to_string(), path));
                }
            }
        }

        self.base.load(root, item, span)
    }
}

struct WorkspaceSourceLoader<'a> {
    sources: &'a HashMap<Url, Source>,
    base: workspace::FileSourceLoader,
}

impl<'a> WorkspaceSourceLoader<'a> {
    /// Construct a new source loader.
    pub(super) fn new(sources: &'a HashMap<Url, Source>) -> Self {
        Self {
            sources,
            base: workspace::FileSourceLoader::new(),
        }
    }
}

impl<'a> workspace::SourceLoader for WorkspaceSourceLoader<'a> {
    fn load(&mut self, span: Span, path: &Path) -> Result<crate::Source, WorkspaceError> {
        if let Ok(url) = crate::languageserver::url::from_file_path(path) {
            if let Some(s) = self.sources.get(&url) {
                return Ok(crate::Source::with_path(url, s.to_string(), path));
            }
        }

        self.base.load(span, path)
    }
}
