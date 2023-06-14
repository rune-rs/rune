//! Runtime helpers for loading code and emitting diagnostics.

use core::fmt::{self, Write};

use crate::no_std::io;
use crate::no_std::prelude::*;

use codespan_reporting::diagnostic as d;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::WriteColor;
pub use codespan_reporting::term::termcolor;

use crate::compile::{ErrorKind, Location, LinkerError};
use crate::diagnostics::{
    Diagnostic, FatalDiagnostic, FatalDiagnosticKind, WarningDiagnostic, WarningDiagnosticKind,
};
use crate::runtime::{Unit, VmErrorKind, VmError};
use crate::{Source, Diagnostics, SourceId, Sources};
use crate::ast::{Span, Spanned};

struct StackFrame {
    source_id: SourceId,
    span: Span,
}

/// Errors that can be raised when formatting diagnostics.
#[derive(Debug)]
#[non_exhaustive]
pub enum EmitError {
    /// Source Error.
    Io(io::Error),
    /// Source Error.
    Fmt(fmt::Error),
    /// Codespan reporting error.
    CodespanReporting(codespan_reporting::files::Error),
}

impl fmt::Display for EmitError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EmitError::Io(error) => error.fmt(f),
            EmitError::Fmt(error) => error.fmt(f),
            EmitError::CodespanReporting(error) => error.fmt(f),
        }
    }
}

impl From<io::Error> for EmitError {
    fn from(source: io::Error) -> Self {
        EmitError::Io(source)
    }
}

impl From<fmt::Error> for EmitError {
    fn from(source: fmt::Error) -> Self {
        EmitError::Fmt(source)
    }
}

impl From<codespan_reporting::files::Error> for EmitError {
    fn from(source: codespan_reporting::files::Error) -> Self {
        EmitError::CodespanReporting(source)
    }
}

impl crate::no_std::error::Error for EmitError {
}

impl Diagnostics {
    /// Generate formatted diagnostics capable of referencing source lines and
    /// hints.
    ///
    /// See [prepare][crate::prepare] for how to use.
    pub fn emit<O>(
        &self,
        out: &mut O,
        sources: &Sources,
    ) -> Result<(), EmitError>
    where
        O: WriteColor,
    {
        if self.is_empty() {
            return Ok(());
        }

        let config = codespan_reporting::term::Config::default();

        for diagnostic in self.diagnostics() {
            match diagnostic {
                Diagnostic::Fatal(e) => {
                    fatal_diagnostics_emit(e, out, sources, &config)?;
                }
                Diagnostic::Warning(w) => {
                    warning_diagnostics_emit(w, out, sources, &config)?;
                }
            }
        }

        Ok(())
    }
}

impl VmError {
    /// Generate formatted diagnostics capable of referencing source lines and
    /// hints.
    ///
    /// See [prepare][crate::prepare] for how to use.
    pub fn emit<O>(
        &self,
        out: &mut O,
        sources: &Sources,
    ) -> Result<(), EmitError>
    where
        O: WriteColor,
    {
        let mut backtrace = vec![];
        let config = codespan_reporting::term::Config::default();

        for l in &self.inner.stacktrace {
            let debug_info = match l.unit.debug_info() {
                Some(debug_info) => debug_info,
                None => continue,
            };

            for ip in [l.ip].into_iter().chain(l.frames.iter().rev().map(|v| v.ip)) {
                let debug_inst = match debug_info.instruction_at(ip) {
                    Some(debug_inst) => debug_inst,
                    None => continue,
                };

                let source_id = debug_inst.source_id;
                let span = debug_inst.span;

                backtrace.push(StackFrame { source_id, span });
            }
        }

        let mut diagnostic = d::Diagnostic::error();

        for at in [&self.inner.error].into_iter().chain(&self.inner.chain) {
            let get = || {
                let l = self.inner.stacktrace.get(at.instruction())?;
                let debug_info = l.unit.debug_info()?;
                let debug_inst = debug_info.instruction_at(l.ip)?;
                Some(debug_inst)
            };

            let debug_inst = match get() {
                Some(debug_inst) => debug_inst,
                None => {
                    println!("error: {} (no debug information)", at);
                    continue;
                }
            };

            let source_id = debug_inst.source_id;
            let span = debug_inst.span;

            let mut labels = Vec::new();

            let (reason, notes) = match at.kind() {
                VmErrorKind::Panic { reason } => {
                    labels.push(d::Label::primary(source_id, span.range()).with_message("panicked"));
                    ("panic in runtime".to_owned(), vec![reason.to_string()])
                }
                VmErrorKind::UnsupportedBinaryOperation { lhs, rhs, op } => {
                    labels.push(
                        d::Label::primary(source_id, span.range())
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
                        d::Label::primary(source_id, span.range())
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
                        d::Label::primary(source_id, span.range())
                            .with_message("in this expression".to_string()),
                    );
                    ("internal vm error".to_owned(), vec![e.to_string()])
                }
            };

            diagnostic = diagnostic.with_message(reason)
                .with_labels(labels)
                .with_notes(notes);
        }

        term::emit(out, &config, sources, &diagnostic)?;

        if !backtrace.is_empty() {
            writeln!(out, "Backtrace:")?;

            for frame in &backtrace {
                let source = match sources.get(frame.source_id) {
                    Some(source) => source,
                    None => continue,
                };

                let (line, line_count, text) = match source.line(frame.span) {
                    Some((line, line_count, text)) => (
                        line.saturating_add(1),
                        line_count.saturating_add(1),
                        text.trim_end(),
                    ),
                    None => continue,
                };

                writeln!(out, "At {}:{}:{}:", source.name(), line, line_count)?;
                writeln!(out, "{text}")?;
            }
        }

        Ok(())
    }
}

impl FatalDiagnostic {
    /// Generate formatted diagnostics capable of referencing source lines and
    /// hints.
    ///
    /// See [prepare][crate::prepare] for how to use.
    pub fn emit<O>(
        &self,
        out: &mut O,
        sources: &Sources,
    ) -> Result<(), EmitError>
    where
        O: WriteColor,
    {
        let config = codespan_reporting::term::Config::default();

        fatal_diagnostics_emit(self, out, sources, &config)
    }
}

impl Unit {
    /// Dump instructions in a human readable manner.
    pub fn emit_instructions<O>(
        &self,
        out: &mut O,
        sources: &Sources,
        with_source: bool,
    ) -> io::Result<()>
    where
        O: WriteColor,
    {
        let mut first_function = true;

        for (n, inst) in self.iter_instructions() {
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

            for label in debug.map(|d| d.labels.as_slice()).unwrap_or_default() {
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

impl Source {
    /// Print formatted diagnostics about a source conveniently.
    pub fn emit_source_line<O>(&self, out: &mut O, span: Span) -> io::Result<()>
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

    let start = start.try_into().unwrap();

    Some((
        line,
        s,
        Span::new(
            span.start.saturating_sub(start),
            span.end.saturating_sub(start),
        ),
    ))
}

/// Helper to emit diagnostics for a warning.
fn warning_diagnostics_emit<O>(
    this: &WarningDiagnostic,
    out: &mut O,
    sources: &Sources,
    config: &codespan_reporting::term::Config,
) -> Result<(), EmitError>
where
    O: WriteColor,
{
    let mut notes = Vec::new();
    let mut labels = Vec::new();

    labels.push(d::Label::primary(this.source_id(), this.span().range()).with_message(this.to_string()));

    match this.kind() {
        WarningDiagnosticKind::LetPatternMightPanic { span, .. } => {
            if let Some(binding) = sources.source(this.source_id(), *span) {
                let mut note = String::new();
                writeln!(note, "Hint: Rewrite to:")?;
                writeln!(note, "if {} {{", binding)?;
                writeln!(note, "    // ..")?;
                writeln!(note, "}}")?;
                notes.push(note);
            }
        }
        WarningDiagnosticKind::RemoveTupleCallParams {
            variant,
            ..
        } => {
            if let Some(variant) = sources.source(this.source_id(), *variant) {
                let mut note = String::new();
                writeln!(note, "Hint: Rewrite to `{}`", variant)?;
                notes.push(note);
            }
        }
        _ => {}
    };

    if let Some(context) = this.context() {
        labels.push(
            d::Label::secondary(this.source_id(), context.range()).with_message("In this context"),
        );
    }

    let diagnostic = d::Diagnostic::warning()
        .with_message("Warning")
        .with_labels(labels)
        .with_notes(notes);

    term::emit(out, config, sources, &diagnostic)?;
    Ok(())
}

/// Custom shared helper for emitting diagnostics for a single error.
fn fatal_diagnostics_emit<O>(
    this: &FatalDiagnostic,
    out: &mut O,
    sources: &Sources,
    config: &codespan_reporting::term::Config,
) -> Result<(), EmitError>
where
    O: WriteColor,
{
    let mut labels = Vec::new();
    let mut notes = Vec::new();

    if let Some(span) = this.span() {
        labels.push(d::Label::primary(this.source_id(), span.range()).with_message(this.kind().to_string()));
    }

    match this.kind() {
        FatalDiagnosticKind::Internal(message) => {
            writeln!(out, "internal error: {}", message)?;
            return Ok(());
        }
        FatalDiagnosticKind::LinkError(error) => {
            match error {
                LinkerError::MissingFunction { hash, spans } => {
                    let mut labels = Vec::new();

                    for (span, source_id) in spans {
                        labels.push(
                            d::Label::primary(*source_id, span.range())
                                .with_message("called here."),
                        );
                    }

                    let diagnostic = d::Diagnostic::error()
                        .with_message(format!(
                            "linker error: missing function with hash `{}`",
                            hash
                        ))
                        .with_labels(labels);

                    term::emit(out, config, sources, &diagnostic)?;
                }
            }

            return Ok(());
        }
        FatalDiagnosticKind::CompileError(error) => {
            format_compile_error(
                this,
                sources,
                error.span(),
                error.kind(),
                &mut labels,
                &mut notes,
            )?;
        }
    };

    let diagnostic = d::Diagnostic::error()
        .with_message(this.kind().to_string())
        .with_labels(labels)
        .with_notes(notes);

    term::emit(out, config, sources, &diagnostic)?;
    return Ok(());

    fn format_compile_error(
        this: &FatalDiagnostic,
        sources: &Sources,
        span: Span,
        kind: &ErrorKind,
        labels: &mut Vec<d::Label<SourceId>>,
        notes: &mut Vec<String>,
    ) -> fmt::Result {
        match kind {
            ErrorKind::ImportCycle { path } => {
                let mut it = path.iter();
                let last = it.next_back();

                for (step, entry) in (1..).zip(it) {
                    labels.push(
                        d::Label::secondary(entry.location.source_id, entry.location.span.range())
                            .with_message(format!("Step #{} for `{}`", step, entry.item)),
                    );
                }

                if let Some(entry) = last {
                    labels.push(
                        d::Label::secondary(entry.location.source_id, entry.location.span.range())
                            .with_message(format!("Final step cycling back to `{}`", entry.item)),
                    );
                }
            }
            ErrorKind::NotVisible {
                chain,
                location: Location { source_id, span },
                ..
            } => {
                for Location { source_id, span } in chain {
                    labels.push(
                        d::Label::secondary(*source_id, span.range())
                            .with_message("Re-exported here"),
                    );
                }

                labels.push(
                    d::Label::secondary(*source_id, span.range()).with_message("defined here"),
                );
            }
            ErrorKind::NotVisibleMod {
                chain,
                location: Location { source_id, span },
                ..
            } => {
                for Location { source_id, span } in chain {
                    labels.push(
                        d::Label::secondary(*source_id, span.range())
                            .with_message("Re-exported here"),
                    );
                }

                labels.push(
                    d::Label::secondary(*source_id, span.range())
                        .with_message("Module defined here"),
                );
            }
            ErrorKind::AmbiguousItem { locations, .. } => {
                for (Location { source_id, span }, item) in locations {
                    labels.push(
                        d::Label::secondary(*source_id, span.range())
                            .with_message(format!("Here as `{item}`")),
                    );
                }
            }
            ErrorKind::AmbiguousContextItem { infos, .. } => {
                for info in infos.as_ref() {
                    labels.push(
                        d::Label::secondary(this.source_id, span.range())
                            .with_message(format!("Could be `{info}`")),
                    );
                }
            }
            ErrorKind::DuplicateObjectKey { existing, object } => {
                labels.push(
                    d::Label::secondary(this.source_id(), existing.range())
                        .with_message("Previously defined here"),
                );

                labels.push(
                    d::Label::secondary(this.source_id(), object.range())
                        .with_message("Object being defined here"),
                );
            }
            ErrorKind::ModAlreadyLoaded { existing, .. } => {
                let (existing_source_id, existing_span) = *existing;

                labels.push(
                    d::Label::secondary(existing_source_id, existing_span.range())
                        .with_message("Previously loaded here"),
                );
            }
            ErrorKind::ExpectedBlockSemiColon { followed_span } => {
                labels.push(
                    d::Label::secondary(this.source_id(), followed_span.range())
                        .with_message("Because this immediately follows"),
                );

                let binding = sources.source(this.source_id(), span);

                if let Some(binding) = binding {
                    let mut note = String::new();
                    writeln!(note, "Hint: Rewrite to `{};`", binding)?;
                    notes.push(note);
                }
            }
            ErrorKind::VariableMoved { moved_at, .. } => {
                labels.push(
                    d::Label::secondary(this.source_id(), moved_at.range())
                        .with_message("Moved here"),
                );
            }
            ErrorKind::NestedTest { nested_span } => {
                labels.push(
                    d::Label::secondary(this.source_id(), nested_span.range())
                        .with_message("Nested in here"),
                );
            }
            ErrorKind::NestedBench { nested_span } => {
                labels.push(
                    d::Label::secondary(this.source_id(), nested_span.range())
                        .with_message("Nested in here"),
                );
            }
            ErrorKind::PatternMissingFields { fields, .. } => {
                let pl = if fields.len() == 1 {
                    "field"
                } else {
                    "fields"
                };

                let fields = fields.join(", ");

                labels.push(
                    d::Label::secondary(this.source_id(), span.range())
                        .with_message(format!("Missing {}: {}", pl, fields)),
                );

                notes.push("You can also make the pattern non-exhaustive by adding `..`".to_string());
            }
            _ => (),
        }

        Ok(())
    }
}
