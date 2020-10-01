//! Runtime helpers for loading code and emitting diagnostics.

use crate::{
    CompileErrorKind, Error, ErrorKind, Errors, LinkerError, Sources, Spanned as _, WarningKind,
    Warnings,
};
use runestick::{Source, Span, Unit, VmError};
use std::error::Error as _;
use std::fmt;
use std::fmt::Write as _;
use std::io;
use thiserror::Error;

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::WriteColor;

pub use codespan_reporting::term::termcolor;

/// Errors that can be raised when formatting diagnostics.
#[derive(Debug, Error)]
pub enum DiagnosticsError {
    /// Source Error.
    #[error("I/O error")]
    Io(#[from] io::Error),
    /// Source Error.
    #[error("formatting error")]
    Fmt(#[from] fmt::Error),
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

/// Emit error diagnostics.
///
/// See [load_sources](crate::load_sources) for how to use.
impl EmitDiagnostics for Errors {
    fn emit_diagnostics<O>(&self, out: &mut O, sources: &Sources) -> Result<(), DiagnosticsError>
    where
        O: WriteColor,
    {
        for error in self {
            error.emit_diagnostics(out, sources)?;
        }

        Ok(())
    }
}

/// Emit warning diagnostics.
///
/// See [load_sources](crate::load_sources) for how to use.
impl EmitDiagnostics for Warnings {
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

        let mut labels = Vec::new();
        let mut notes = Vec::new();

        for w in self {
            let context = match &w.kind {
                WarningKind::NotUsed { span, context } => {
                    labels.push(
                        Label::primary(w.source_id, span.start..span.end)
                            .with_message("value not used"),
                    );

                    *context
                }
                WarningKind::LetPatternMightPanic { span, context } => {
                    labels.push(
                        Label::primary(w.source_id, span.start..span.end)
                            .with_message("let binding might panic"),
                    );

                    let binding = sources.source_at(w.source_id).and_then(|s| s.source(*span));

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
                        Label::primary(w.source_id, span.start..span.end)
                            .with_message("template string without expansions like `{1 + 2}`"),
                    );

                    *context
                }
                WarningKind::RemoveTupleCallParams {
                    span,
                    variant,
                    context,
                } => {
                    labels.push(
                        Label::secondary(w.source_id, span.start..span.end).with_message(
                            "constructing this variant could be done without parentheses",
                        ),
                    );

                    let variant = sources
                        .source_at(w.source_id)
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
                        Label::primary(w.source_id, span.start..span.end)
                            .with_message("unnecessary semicolon"),
                    );

                    None
                }
            };

            if let Some(context) = context {
                labels.push(
                    Label::secondary(w.source_id, context.start..context.end)
                        .with_message("in this context"),
                );
            }
        }

        let diagnostic = Diagnostic::warning()
            .with_message("warning")
            .with_labels(labels)
            .with_notes(notes);

        term::emit(out, &config, &files, &diagnostic)?;
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

        let (unit, ip) = match unwound {
            Some((unit, ip)) => (unit, ip),
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

        labels
            .push(Label::primary(source_id, span.start..span.end).with_message(error.to_string()));

        let diagnostic = Diagnostic::error()
            .with_message("virtual machine error")
            .with_labels(labels);

        term::emit(out, &config, &files, &diagnostic)?;
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

        let mut labels = Vec::new();
        let mut notes = Vec::new();

        let span = match self.kind() {
            ErrorKind::Internal(message) => {
                writeln!(out, "internal error: {}", message)?;
                return Ok(());
            }
            ErrorKind::LinkError(error) => {
                match error {
                    LinkerError::MissingFunction { hash, spans } => {
                        let mut labels = Vec::new();

                        for (span, source_id) in spans {
                            labels.push(
                                Label::primary(*source_id, span.start..span.end)
                                    .with_message("called here."),
                            );
                        }

                        let diagnostic = Diagnostic::error()
                            .with_message(format!(
                                "linker error: missing function with hash `{}`",
                                hash
                            ))
                            .with_labels(labels);

                        term::emit(out, &config, &files, &diagnostic)?;
                    }
                }

                return Ok(());
            }
            ErrorKind::ParseError(error) => error.span(),
            ErrorKind::CompileError(error) => {
                match error.kind() {
                    CompileErrorKind::DuplicateObjectKey { existing, object } => {
                        labels.push(
                            Label::secondary(self.source_id(), existing.start..existing.end)
                                .with_message("previously defined here"),
                        );

                        labels.push(
                            Label::secondary(self.source_id(), object.start..object.end)
                                .with_message("object being defined here"),
                        );
                    }
                    CompileErrorKind::ModAlreadyLoaded { existing, .. } => {
                        let (existing_source_id, existing_span) = *existing;

                        labels.push(
                            Label::secondary(
                                existing_source_id,
                                existing_span.start..existing_span.end,
                            )
                            .with_message("previously loaded here"),
                        );
                    }
                    CompileErrorKind::ExpectedBlockSemiColon { followed_span } => {
                        labels.push(
                            Label::secondary(
                                self.source_id(),
                                followed_span.start..followed_span.end,
                            )
                            .with_message("because this immediately follows"),
                        );

                        let binding = sources
                            .source_at(self.source_id())
                            .and_then(|s| s.source(error.span()));

                        if let Some(binding) = binding {
                            let mut note = String::new();
                            writeln!(note, "Hint: Rewrite to `{};`", binding)?;
                            notes.push(note);
                        }
                    }
                    CompileErrorKind::ImportConflict {
                        existing: (source_id, span),
                        ..
                    } => {
                        labels.push(
                            Label::secondary(*source_id, span.start..span.end)
                                .with_message("previous import here"),
                        );
                    }
                    CompileErrorKind::ImportCycle { path } => {
                        let mut it = path.into_iter();
                        let last = it.next_back();

                        for (step, entry) in (1..).zip(it) {
                            labels.push(
                                Label::secondary(entry.source_id, entry.span.start..entry.span.end)
                                    .with_message(format!("step #{} for `{}`", step, entry.item)),
                            );
                        }

                        if let Some(entry) = last {
                            labels.push(
                                Label::secondary(entry.source_id, entry.span.start..entry.span.end)
                                    .with_message(format!(
                                        "final step cycling back to `{}`",
                                        entry.item
                                    )),
                            );
                        }
                    }
                    _ => (),
                }

                error.span()
            }
            ErrorKind::QueryError(error) => error.span(),
        };

        if let Some(e) = self.kind().source() {
            labels.push(
                Label::primary(self.source_id(), span.start..span.end).with_message(e.to_string()),
            );
        }

        let diagnostic = Diagnostic::error()
            .with_message(self.kind().to_string())
            .with_labels(labels)
            .with_notes(notes);

        term::emit(out, &config, &files, &diagnostic)?;
        Ok(())
    }
}

/// Get the line number and source line for the given source and span.
pub fn line_for(source: &Source, span: Span) -> Option<(usize, &str, Span)> {
    let line_starts = source.line_starts();

    let line = match line_starts.binary_search(&span.start) {
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

    Some((line, s, Span::new(span.start - start, span.end - start)))
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
            let end = usize::min(span.end, line.len());

            let before = &line[0..span.start];
            let inner = &line[span.start..end];
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
