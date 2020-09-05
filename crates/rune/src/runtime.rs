//! Runtime helpers for loading code and emitting diagnostics.

use crate::compiler;
use crate::{CompileError, Options, WarningKind, Warnings};
use runestick::unit::{LinkerError, LinkerErrors};
use runestick::{Context, Source, Span, Unit};
use std::cell::RefCell;
use std::error::Error as _;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use thiserror::Error;

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::WriteColor;

pub use codespan_reporting::term::termcolor;

/// A runtime error.
#[derive(Debug, Error)]
pub enum LoadError {
    /// Failed to read the given file.
    #[error("failed to read file: {path}: {error}")]
    ReadFile {
        /// The source error.
        #[source]
        error: io::Error,
        /// The path that we couldn't read.
        path: PathBuf,
    },
    /// Compiler error.
    #[error("compile error")]
    CompileError {
        /// The source error.
        #[source]
        error: CompileError,
        /// The source file we tried to compile.
        code_source: Source,
    },
    /// A linker error occured.
    #[error("linker error")]
    LinkError {
        /// Errors that happened during linking.
        errors: LinkerErrors,
        /// The file id of the link error.
        code_source: Source,
    },
}

impl LoadError {
    /// Emit diagnostics about the last error we encountered.
    pub fn emit_diagnostics<O>(&self, out: &mut O) -> Result<(), DiagnosticsError>
    where
        O: WriteColor,
    {
        let config = codespan_reporting::term::Config::default();

        let mut labels = Vec::new();

        let (span, code_source) = match self {
            Self::ReadFile { error, path } => {
                writeln!(out, "failed to read file: {}: {}", path.display(), error)?;
                return Ok(());
            }
            Self::LinkError {
                errors,
                code_source,
            } => {
                let mut files = SimpleFiles::new();
                let source_id = files.add(&code_source.name, &code_source.source);

                for error in errors {
                    match error {
                        LinkerError::MissingFunction { hash, spans } => {
                            let mut labels = Vec::new();

                            for span in spans {
                                labels.push(
                                    Label::primary(source_id, span.start..span.end)
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
            Self::CompileError { error, code_source } => {
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

                (span, code_source)
            }
        };

        let mut files = SimpleFiles::new();
        let source_id = files.add(&code_source.name, &code_source.source);

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

/// Error emitted when we saw an error while we were emitting diagnostics.
#[derive(Debug, Error)]
pub enum DiagnosticsError {
    /// Source Error.
    #[error("I/O error")]
    Io(#[from] io::Error),
    /// Source Error.
    #[error("formatting error")]
    Fmt(#[from] fmt::Error),
}

/// Load the given path into the runtime.
///
/// The name of the loaded source will be the path as a string.
pub fn load_path(
    context: &Context,
    options: &Options,
    warnings: &mut Warnings,
    path: &Path,
) -> Result<Unit, LoadError> {
    let source = fs::read_to_string(path).map_err(|error| LoadError::ReadFile {
        error,
        path: path.to_owned(),
    })?;

    let name = path.display().to_string();
    let unit = load_source(context, options, warnings, Source::new(name, source))?;
    Ok(unit)
}

/// Load the given source and return a number corresponding to its file id.
///
/// Use the provided `name` when generating diagnostics to reference the
/// file.
pub fn load_source(
    context: &Context,
    options: &Options,
    warnings: &mut Warnings,
    code_source: Source,
) -> Result<Unit, LoadError> {
    let unit = Rc::new(RefCell::new(Unit::with_default_prelude()));

    if let Err(error) =
        compiler::compile_with_options(&*context, &code_source, &options, &unit, warnings)
    {
        return Err(LoadError::CompileError { error, code_source });
    }

    let unit = match Rc::try_unwrap(unit) {
        Ok(unit) => unit.into_inner(),
        Err(..) => {
            return Err(LoadError::CompileError {
                error: CompileError::internal("unit is not exlusively held", Span::empty()),
                code_source,
            })
        }
    };

    if options.link_checks {
        let mut errors = LinkerErrors::new();

        if !unit.link(&*context, &mut errors) {
            return Err(LoadError::LinkError {
                errors,
                code_source,
            });
        }
    }

    Ok(unit)
}

/// Emit warning diagnostics.
pub fn emit_warning_diagnostics<O>(
    out: &mut O,
    warnings: &Warnings,
    unit: &Unit,
) -> Result<(), DiagnosticsError>
where
    O: WriteColor,
{
    use std::fmt::Write as _;

    let config = codespan_reporting::term::Config::default();
    let mut files = SimpleFiles::new();

    if let Some(debug_info) = unit.debug_info() {
        for (source_id, source) in debug_info.sources() {
            let file_id = files.add(&source.name, source.as_str());
            debug_assert!(file_id == source_id);
        }
    }

    let mut labels = Vec::new();
    let mut notes = Vec::new();

    for w in warnings {
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

                let binding = unit
                    .debug_info()
                    .and_then(|dbg| dbg.source_at(w.source_id))
                    .and_then(|s| s.source(*span));

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

                let variant = unit
                    .debug_info()
                    .and_then(|dbg| dbg.source_at(w.source_id))
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

/// Emit diagnostics for the given vm error.
pub fn emit_vm_error_diagnostics<O>(
    out: &mut O,
    error: runestick::VmError,
) -> Result<(), DiagnosticsError>
where
    O: WriteColor,
{
    let (error, unwound) = error.into_unwound();

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

    let source = match debug_info.source_at(debug_inst.source_id) {
        Some(source) => source,
        None => {
            writeln!(
                out,
                "virtual machine error: {} (no source available)",
                error
            )?;

            return Ok(());
        }
    };

    let config = codespan_reporting::term::Config::default();

    let mut files = SimpleFiles::new();
    let id = files.add(&source.name, &source.source);

    let mut labels = Vec::new();
    let span = debug_inst.span;

    labels.push(Label::primary(id, span.start..span.end).with_message(error.to_string()));

    let diagnostic = Diagnostic::error()
        .with_message("virtual machine error")
        .with_labels(labels);

    term::emit(out, &config, &files, &diagnostic)?;
    Ok(())
}
