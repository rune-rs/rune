//! Runtime helpers for loading code and emitting diagnostics.

use crate::{
    CompileErrorKind, Diagnostics, Error, ErrorKind, IrErrorKind, LinkerError, QueryErrorKind,
    ResolveErrorKind, Sources, Spanned as _, Warning, WarningKind,
};
use runestick::{Location, Source, SourceId, Span, Unit, VmError, VmErrorKind};
use std::error::Error as _;
use std::fmt;
use std::fmt::Write as _;
use std::io;
use thiserror::Error;

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::{Files, SimpleFiles};
use codespan_reporting::term;
use codespan_reporting::term::termcolor::WriteColor;

pub use codespan_reporting::term::termcolor;

struct StackFrame {
    source_id: SourceId,
    span: Span,
}

/// Errors that can be raised when formatting diagnostics.
#[derive(Debug, Error)]
pub enum DiagnosticsError {
    /// Source Error.
    #[error("I/O error")]
    Io(#[from] io::Error),
    /// Source Error.
    #[error("formatting error")]
    Fmt(#[from] fmt::Error),
    /// Codespan reporting error.
    #[error("codespan reporting error")]
    CodespanReporting(#[from] codespan_reporting::files::Error),
}

/// Helper trait for emitting diagnostics.
///
/// See [load_sources](crate::load_sources) for how to use.
pub trait EmitDiagnostics {
    /// Emit diagnostics for the current type.
    fn emit_diagnostics<O>(&self, out: &mut O, sources: &Sources) -> Result<(), DiagnosticsError>
    where
        O: WriteColor;
}

/// Emit collected diagnostics.
///
/// See [load_sources](crate::load_sources) for how to use.
impl EmitDiagnostics for Diagnostics {
    fn emit_diagnostics<O>(&self, out: &mut O, sources: &Sources) -> Result<(), DiagnosticsError>
    where
        O: WriteColor,
    {
        if self.is_empty() {
            return Ok(());
        }

        let config = codespan_reporting::term::Config::default();
        let mut files = SimpleFiles::new();

        for source in sources.iter() {
            files.add(source.name(), source.as_str());
        }

        for diagnostic in self.diagnostics() {
            match diagnostic {
                crate::Diagnostic::Error(e) => {
                    error_emit_diagnostics_with(e, out, sources, &files, &config)?;
                }
                crate::Diagnostic::Warning(w) => {
                    warning_emit_diagnostics_with(w, out, sources, &files, &config)?;
                }
            }
        }

        Ok(())
    }
}

impl EmitDiagnostics for VmError {
    fn emit_diagnostics<O>(&self, out: &mut O, sources: &Sources) -> Result<(), DiagnosticsError>
    where
        O: WriteColor,
    {
        let mut files = SimpleFiles::new();

        for source in sources.iter() {
            files.add(source.name(), source.as_str());
        }

        let (error, unwound) = self.as_unwound();

        let (unit, ip, frames) = match unwound {
            Some((unit, ip, frames)) => (unit, ip, frames),
            None => {
                writeln!(
                    out,
                    "virtual machine error: {} (no diagnostics available)",
                    error
                )?;

                return Ok(());
            }
        };

        let debug_info = match unit.debug_info() {
            Some(debug_info) => debug_info,
            None => {
                writeln!(out, "virtual machine error: {} (no debug info)", error)?;
                return Ok(());
            }
        };
        let debug_inst = match debug_info.instruction_at(ip) {
            Some(debug_inst) => debug_inst,
            None => {
                writeln!(
                    out,
                    "virtual machine error: {} (no debug instruction)",
                    error
                )?;

                return Ok(());
            }
        };

        let config = codespan_reporting::term::Config::default();

        let mut labels = Vec::new();

        let source_id = debug_inst.source_id;
        let span = debug_inst.span;

        let mut backtrace = vec![StackFrame { source_id, span }];
        let (reason, notes) = match error {
            VmErrorKind::Panic { reason } => {
                labels.push(Label::primary(source_id, span.range()).with_message("panicked"));
                ("panic in runtime".to_owned(), vec![reason.to_string()])
            }
            VmErrorKind::UnsupportedBinaryOperation { lhs, rhs, op } => {
                labels.push(
                    Label::primary(source_id, span.range())
                        .with_message("in this expression".to_string()),
                );

                (
                    format!("type mismatch for operation `{}`", op),
                    vec![
                        format!("left hand side has type `{}`", lhs),
                        format!("right hand side has type `{}`", rhs),
                    ],
                )
            }
            VmErrorKind::BadArgumentCount { actual, expected } => {
                labels.push(
                    Label::primary(source_id, span.range())
                        .with_message("in this function call".to_string()),
                );

                (
                    "wrong number of arguments".to_string(),
                    vec![
                        format!("expected `{}`", expected),
                        format!("got `{}`", actual),
                    ],
                )
            }
            e => {
                labels.push(
                    Label::primary(source_id, span.range())
                        .with_message("in this expression".to_string()),
                );
                ("internal vm error".to_owned(), vec![e.to_string()])
            }
        };

        for ip in frames.iter().map(|v| v.ip()) {
            let debug_inst = match debug_info.instruction_at(ip) {
                Some(debug_inst) => debug_inst,
                None => {
                    writeln!(
                        out,
                        "virtual machine error: {} (no debug instruction)",
                        error
                    )?;

                    return Ok(());
                }
            };

            let source_id = debug_inst.source_id;
            let span = debug_inst.span;

            backtrace.push(StackFrame { source_id, span });
        }
        let diagnostic = Diagnostic::error()
            .with_message(reason)
            .with_labels(labels)
            .with_notes(notes);

        term::emit(out, &config, &files, &diagnostic)?;

        writeln!(out, "Callstack:")?;
        for frame in &backtrace {
            let line = files
                .line_index(frame.source_id, frame.span.start.into_usize())
                .unwrap();
            let line = files.line_number(frame.source_id, line).unwrap() - 1;
            let line_range = files.line_range(frame.source_id, line).expect("a range");
            let source = files.get(frame.source_id)?;
            let name = source.name();
            let slice = &source.source()[line_range];

            write!(out, "\t{}:{}\n\t\t{}", name, line, slice)?;
        }
        Ok(())
    }
}

/// Helper to emit diagnostics for a warning.
fn warning_emit_diagnostics_with<'a, O>(
    this: &Warning,
    out: &mut O,
    sources: &'a Sources,
    files: &'a impl Files<'a, FileId = SourceId>,
    config: &codespan_reporting::term::Config,
) -> Result<(), DiagnosticsError>
where
    O: WriteColor,
{
    let mut notes = Vec::new();
    let mut labels = Vec::new();

    let context = match this.kind() {
        WarningKind::NotUsed { span, context } => {
            labels.push(Label::primary(this.source_id(), span.range()).with_message("not used"));

            *context
        }
        WarningKind::LetPatternMightPanic { span, context } => {
            labels.push(
                Label::primary(this.source_id(), span.range())
                    .with_message("let binding might panic"),
            );

            let binding = sources
                .source_at(this.source_id())
                .and_then(|s| s.source(*span));

            if let Some(binding) = binding {
                let mut note = String::new();
                writeln!(note, "Hint: Rewrite to:")?;
                writeln!(note, "if {} {{", binding)?;
                writeln!(note, "    // ..")?;
                writeln!(note, "}}")?;
                notes.push(note);
            }

            *context
        }
        WarningKind::TemplateWithoutExpansions { span, context } => {
            labels.push(
                Label::primary(this.source_id(), span.range())
                    .with_message("template string without expansions like `${1 + 2}`"),
            );

            *context
        }
        WarningKind::RemoveTupleCallParams {
            span,
            variant,
            context,
        } => {
            labels.push(
                Label::secondary(this.source_id(), span.range())
                    .with_message("constructing this variant could be done without parentheses"),
            );

            let variant = sources
                .source_at(this.source_id())
                .and_then(|s| s.source(*variant));

            if let Some(variant) = variant {
                let mut note = String::new();
                writeln!(note, "Hint: Rewrite to `{}`", variant)?;
                notes.push(note);
            }

            *context
        }
        WarningKind::UnecessarySemiColon { span } => {
            labels.push(
                Label::primary(this.source_id(), span.range())
                    .with_message("unnecessary semicolon"),
            );

            None
        }
    };

    if let Some(context) = context {
        labels.push(
            Label::secondary(this.source_id(), context.range()).with_message("in this context"),
        );
    }

    let diagnostic = Diagnostic::warning()
        .with_message("warning")
        .with_labels(labels)
        .with_notes(notes);

    term::emit(out, config, files, &diagnostic)?;
    Ok(())
}

/// Custom shared helper for emitting diagnostics for a single error.
fn error_emit_diagnostics_with<O>(
    this: &Error,
    out: &mut O,
    sources: &Sources,
    files: &SimpleFiles<&str, &str>,
    config: &codespan_reporting::term::Config,
) -> Result<(), DiagnosticsError>
where
    O: WriteColor,
{
    let mut labels = Vec::new();
    let mut notes = Vec::new();

    let span = match this.kind() {
        ErrorKind::Internal(message) => {
            writeln!(out, "internal error: {}", message)?;
            return Ok(());
        }
        ErrorKind::BuildError(error) => {
            writeln!(out, "build error: {}", error)?;
            return Ok(());
        }
        ErrorKind::LinkError(error) => {
            match error {
                LinkerError::MissingFunction { hash, spans } => {
                    let mut labels = Vec::new();

                    for (span, source_id) in spans {
                        labels.push(
                            Label::primary(*source_id, span.range()).with_message("called here."),
                        );
                    }

                    let diagnostic = Diagnostic::error()
                        .with_message(format!(
                            "linker error: missing function with hash `{}`",
                            hash
                        ))
                        .with_labels(labels);

                    term::emit(out, config, files, &diagnostic)?;
                }
            }

            return Ok(());
        }
        ErrorKind::ParseError(error) => error.span(),
        ErrorKind::CompileError(error) => {
            format_compile_error(
                this,
                sources,
                error.span(),
                error.kind(),
                &mut labels,
                &mut notes,
            )?;

            error.span()
        }
        ErrorKind::QueryError(error) => {
            format_query_error(
                this,
                sources,
                error.span(),
                error.kind(),
                &mut labels,
                &mut notes,
            )?;

            error.span()
        }
    };

    if let Some(e) = this.kind().source() {
        labels.push(Label::primary(this.source_id(), span.range()).with_message(e.to_string()));
    }

    let diagnostic = Diagnostic::error()
        .with_message(this.kind().to_string())
        .with_labels(labels)
        .with_notes(notes);

    term::emit(out, config, files, &diagnostic)?;
    return Ok(());

    fn format_compile_error(
        this: &Error,
        sources: &Sources,
        error_span: Span,
        kind: &CompileErrorKind,
        labels: &mut Vec<Label<SourceId>>,
        notes: &mut Vec<String>,
    ) -> fmt::Result {
        match kind {
            CompileErrorKind::QueryError { error } => {
                format_query_error(this, sources, error_span, error, labels, notes)?;
            }
            CompileErrorKind::DuplicateObjectKey { existing, object } => {
                labels.push(
                    Label::secondary(this.source_id(), existing.range())
                        .with_message("previously defined here"),
                );

                labels.push(
                    Label::secondary(this.source_id(), object.range())
                        .with_message("object being defined here"),
                );
            }
            CompileErrorKind::ModAlreadyLoaded { existing, .. } => {
                let (existing_source_id, existing_span) = *existing;

                labels.push(
                    Label::secondary(existing_source_id, existing_span.range())
                        .with_message("previously loaded here"),
                );
            }
            CompileErrorKind::ExpectedBlockSemiColon { followed_span } => {
                labels.push(
                    Label::secondary(this.source_id(), followed_span.range())
                        .with_message("because this immediately follows"),
                );

                let binding = sources
                    .source_at(this.source_id())
                    .and_then(|s| s.source(error_span));

                if let Some(binding) = binding {
                    let mut note = String::new();
                    writeln!(note, "Hint: Rewrite to `{};`", binding)?;
                    notes.push(note);
                }
            }
            CompileErrorKind::VariableMoved { moved_at, .. } => {
                labels.push(
                    Label::secondary(this.source_id(), moved_at.range()).with_message("moved here"),
                );
            }
            CompileErrorKind::CallMacroError { item, .. } => {
                notes.push(format!("Error originated in the `{}` macro", item));
            }
            CompileErrorKind::NestedTest { nested_span } => {
                labels.push(
                    Label::secondary(this.source_id(), nested_span.range())
                        .with_message("nested in here"),
                );
            }
            _ => (),
        }

        Ok(())
    }

    fn format_query_error(
        this: &Error,
        sources: &Sources,
        error_span: Span,
        kind: &QueryErrorKind,
        labels: &mut Vec<Label<SourceId>>,
        notes: &mut Vec<String>,
    ) -> fmt::Result {
        match kind {
            QueryErrorKind::ResolveError { error } => {
                format_resolve_error(this, sources, error_span, error, labels, notes)?;
            }
            QueryErrorKind::IrError { error } => {
                format_ir_error(this, sources, error_span, error, labels, notes)?;
            }
            QueryErrorKind::ImportCycle { path } => {
                let mut it = path.iter();
                let last = it.next_back();

                for (step, entry) in (1..).zip(it) {
                    labels.push(
                        Label::secondary(entry.location.source_id, entry.location.span.range())
                            .with_message(format!("step #{} for `{}`", step, entry.item)),
                    );
                }

                if let Some(entry) = last {
                    labels.push(
                        Label::secondary(entry.location.source_id, entry.location.span.range())
                            .with_message(format!("final step cycling back to `{}`", entry.item)),
                    );
                }
            }
            QueryErrorKind::ItemConflict {
                other: Location { source_id, span },
                ..
            } => {
                labels.push(
                    Label::secondary(*source_id, span.range())
                        .with_message("previously defined here"),
                );
            }
            QueryErrorKind::NotVisible {
                chain,
                location: Location { source_id, span },
                ..
            } => {
                for Location { source_id, span } in chain {
                    labels.push(
                        Label::secondary(*source_id, span.range()).with_message("re-exported here"),
                    );
                }

                labels
                    .push(Label::secondary(*source_id, span.range()).with_message("defined here"));
            }
            QueryErrorKind::NotVisibleMod {
                chain,
                location: Location { source_id, span },
                ..
            } => {
                for Location { source_id, span } in chain {
                    labels.push(
                        Label::secondary(*source_id, span.range()).with_message("re-exported here"),
                    );
                }

                labels.push(
                    Label::secondary(*source_id, span.range()).with_message("module defined here"),
                );
            }
            QueryErrorKind::AmbiguousItem { locations, .. } => {
                for (Location { source_id, span }, item) in locations {
                    labels.push(
                        Label::secondary(*source_id, span.range())
                            .with_message(format!("here as `{}`", item)),
                    );
                }
            }
            _ => (),
        }

        Ok(())
    }

    fn format_ir_error(
        this: &Error,
        sources: &Sources,
        error_span: Span,
        kind: &IrErrorKind,
        labels: &mut Vec<Label<SourceId>>,
        notes: &mut Vec<String>,
    ) -> fmt::Result {
        if let IrErrorKind::QueryError { error } = kind {
            format_query_error(this, sources, error_span, error, labels, notes)?;
        }

        Ok(())
    }

    fn format_resolve_error(
        _: &Error,
        _: &Sources,
        _: Span,
        _: &ResolveErrorKind,
        _: &mut Vec<Label<SourceId>>,
        _: &mut Vec<String>,
    ) -> fmt::Result {
        Ok(())
    }
}

impl EmitDiagnostics for Error {
    fn emit_diagnostics<O>(&self, out: &mut O, sources: &Sources) -> Result<(), DiagnosticsError>
    where
        O: WriteColor,
    {
        let config = codespan_reporting::term::Config::default();

        let mut files = SimpleFiles::new();

        for source in sources.iter() {
            files.add(source.name(), source.as_str());
        }

        error_emit_diagnostics_with(self, out, sources, &files, &config)
    }
}

/// Get the line number and source line for the given source and span.
pub fn line_for(source: &Source, span: Span) -> Option<(usize, &str, Span)> {
    let line_starts = source.line_starts();

    let line = match line_starts.binary_search(&span.start.into_usize()) {
        Ok(n) => n,
        Err(n) => n.saturating_sub(1),
    };

    let start = *line_starts.get(line)?;
    let end = line.checked_add(1)?;

    let s = if let Some(end) = line_starts.get(end) {
        source.get(start..*end)?
    } else {
        source.get(start..)?
    };

    Some((
        line,
        s,
        Span::new(
            span.start.into_usize() - start,
            span.end.into_usize() - start,
        ),
    ))
}

/// Trait to dump the instructions of a unit to the given writer.
///
/// This is implemented for [Unit].
pub trait DumpInstructions {
    /// Dump the instructions of the current unit to the given writer.
    fn dump_instructions<O>(
        &self,
        out: &mut O,
        sources: &Sources,
        with_source: bool,
    ) -> io::Result<()>
    where
        O: WriteColor;
}

impl DumpInstructions for Unit {
    fn dump_instructions<O>(
        &self,
        out: &mut O,
        sources: &Sources,
        with_source: bool,
    ) -> io::Result<()>
    where
        O: WriteColor,
    {
        let mut first_function = true;

        for (n, inst) in self.iter_instructions().enumerate() {
            let debug = self.debug_info().and_then(|d| d.instruction_at(n));

            if let Some((hash, signature)) = self.debug_info().and_then(|d| d.function_at(n)) {
                if !std::mem::take(&mut first_function) {
                    writeln!(out)?;
                }

                writeln!(out, "fn {} ({}):", signature, hash)?;
            }

            if with_source {
                if let Some((source, span)) =
                    debug.and_then(|d| sources.get(d.source_id).map(|s| (s, d.span)))
                {
                    source.emit_source_line(out, span)?;
                }
            }

            if let Some(label) = debug.and_then(|d| d.label.as_ref()) {
                writeln!(out, "{}:", label)?;
            }

            write!(out, "  {:04} = {}", n, inst)?;

            if let Some(comment) = debug.and_then(|d| d.comment.as_ref()) {
                write!(out, " // {}", comment)?;
            }

            writeln!(out)?;
        }

        Ok(())
    }
}

/// Helper trait to emit source code locations.
///
/// These are implemented for [Source], so that you can print diagnostics about
/// a source conveniently.
pub trait EmitSource {
    /// Emit the source location as a single line of the given writer.
    fn emit_source_line<O>(&self, out: &mut O, span: Span) -> io::Result<()>
    where
        O: WriteColor;
}

impl EmitSource for Source {
    fn emit_source_line<O>(&self, out: &mut O, span: Span) -> io::Result<()>
    where
        O: WriteColor,
    {
        let mut highlight = termcolor::ColorSpec::new();
        highlight.set_fg(Some(termcolor::Color::Blue));

        let diagnostics = line_for(self, span);

        if let Some((count, line, span)) = diagnostics {
            let line = line.trim_end();
            let end = usize::min(span.end.into_usize(), line.len());

            let before = &line[0..span.start.into_usize()];
            let inner = &line[span.start.into_usize()..end];
            let after = &line[end..];

            write!(out, "  {}:{: <3} - {}", self.name(), count + 1, before,)?;

            out.set_color(&highlight)?;
            write!(out, "{}", inner)?;
            out.reset()?;
            write!(out, "{}", after)?;

            if span.end != end {
                write!(out, " .. trimmed")?;
            }

            writeln!(out)?;
        }

        Ok(())
    }
}
