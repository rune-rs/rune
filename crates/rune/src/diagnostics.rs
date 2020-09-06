//! Runtime helpers for loading code and emitting diagnostics.

use crate::{CompileError, LoadError, LoadErrorKind, Sources, WarningKind, Warnings};
use runestick::{LinkerError, VmError};
use std::error::Error as _;
use std::fmt;
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
/// See [load_path](crate::load_path) for how to use.
pub trait EmitDiagnostics {
    /// Emit diagnostics for the current type.
    fn emit_diagnostics<O>(self, out: &mut O, sources: &Sources) -> Result<(), DiagnosticsError>
    where
        O: WriteColor;
}

/// Emit warning diagnostics.
///
/// See [load_path](crate::load_path) for how to use.
impl EmitDiagnostics for Warnings {
    fn emit_diagnostics<O>(self, out: &mut O, sources: &Sources) -> Result<(), DiagnosticsError>
    where
        O: WriteColor,
    {
        use std::fmt::Write as _;

        let config = codespan_reporting::term::Config::default();
        let mut files = SimpleFiles::new();

        for source in sources.iter() {
            files.add(source.name(), source.as_str());
        }

        let mut labels = Vec::new();
        let mut notes = Vec::new();

        for w in &self {
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
                        writeln!(note, "Consider rewriting to:")?;
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
                        writeln!(note, "Consider rewriting to `{}`", variant)?;
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
    fn emit_diagnostics<O>(self, out: &mut O, sources: &Sources) -> Result<(), DiagnosticsError>
    where
        O: WriteColor,
    {
        let mut files = SimpleFiles::new();

        for source in sources.iter() {
            files.add(source.name(), source.as_str());
        }

        let (error, unwound) = self.into_unwound();

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

impl EmitDiagnostics for LoadError {
    fn emit_diagnostics<O>(self, out: &mut O, sources: &Sources) -> Result<(), DiagnosticsError>
    where
        O: WriteColor,
    {
        let config = codespan_reporting::term::Config::default();

        let mut files = SimpleFiles::new();

        for source in sources.iter() {
            files.add(source.name(), source.as_str());
        }

        let mut labels = Vec::new();

        let (span, source_id) = match self.kind() {
            LoadErrorKind::Internal { message } => {
                writeln!(out, "internal error: {}", message)?;
                return Ok(());
            }
            LoadErrorKind::ReadFile { error, path } => {
                writeln!(out, "failed to read file: {}: {}", path.display(), error)?;
                return Ok(());
            }
            LoadErrorKind::LinkError { errors } => {
                for error in errors {
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
                                .with_message(format!("missing function with hash `{}`", hash))
                                .with_labels(labels);

                            term::emit(out, &config, &files, &diagnostic)?;
                        }
                    }
                }

                return Ok(());
            }
            LoadErrorKind::ParseError { source_id, error } => (error.span(), *source_id),
            LoadErrorKind::CompileError { source_id, error } => {
                let span = match error {
                    CompileError::ReturnLocalReferences {
                        block,
                        references_at,
                        span,
                        ..
                    } => {
                        for ref_span in references_at {
                            if span.overlaps(*ref_span) {
                                continue;
                            }

                            labels.push(
                                Label::secondary(0, ref_span.start..ref_span.end)
                                    .with_message("reference created here"),
                            );
                        }

                        labels.push(
                            Label::secondary(0, block.start..block.end)
                                .with_message("block returned from"),
                        );

                        *span
                    }
                    CompileError::DuplicateObjectKey {
                        span,
                        existing,
                        object,
                    } => {
                        labels.push(
                            Label::secondary(0, existing.start..existing.end)
                                .with_message("previously defined here"),
                        );

                        labels.push(
                            Label::secondary(0, object.start..object.end)
                                .with_message("object being defined here"),
                        );

                        *span
                    }
                    error => error.span(),
                };

                (span, *source_id)
            }
        };

        if let Some(e) = self.source() {
            labels
                .push(Label::primary(source_id, span.start..span.end).with_message(e.to_string()));
        }

        let diagnostic = Diagnostic::error()
            .with_message(self.to_string())
            .with_labels(labels);

        term::emit(out, &config, &files, &diagnostic)?;
        Ok(())
    }
}
