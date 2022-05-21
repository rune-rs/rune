//! Runtime helpers for loading code and emitting diagnostics.

use crate::compile::{CompileErrorKind, LinkerError};
use crate::diagnostics::{
    Diagnostic, FatalDiagnostic, FatalDiagnosticKind, WarningDiagnostic, WarningDiagnosticKind,
};
use crate::compile::ir::IrErrorKind;
use crate::parse::ResolveErrorKind;
use crate::query::QueryErrorKind;
use crate::runtime::{VmError, VmErrorKind};
use crate::{Diagnostics, Location, SourceId, Sources, Span, Spanned};
use std::error::Error;
use std::fmt;
use std::fmt::Write;
use std::io;
use thiserror::Error;

use codespan_reporting::diagnostic as d;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::WriteColor;

pub use codespan_reporting::term::termcolor;

struct StackFrame {
    source_id: SourceId,
    span: Span,
}

/// Errors that can be raised when formatting diagnostics.
#[derive(Debug, Error)]
pub enum EmitDiagnosticsError {
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
/// See [build](crate::build) for how to use.
pub trait EmitDiagnostics {
    /// Emit diagnostics for the current type.
    fn emit_diagnostics<O>(
        &self,
        out: &mut O,
        sources: &Sources,
    ) -> Result<(), EmitDiagnosticsError>
    where
        O: WriteColor;
}

/// Emit collected diagnostics.
///
/// See [build](crate::build) for how to use.
impl EmitDiagnostics for Diagnostics {
    fn emit_diagnostics<O>(
        &self,
        out: &mut O,
        sources: &Sources,
    ) -> Result<(), EmitDiagnosticsError>
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

impl EmitDiagnostics for VmError {
    fn emit_diagnostics<O>(
        &self,
        out: &mut O,
        sources: &Sources,
    ) -> Result<(), EmitDiagnosticsError>
    where
        O: WriteColor,
    {
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

        let (reason, notes) = match error {
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

        let mut backtrace = vec![StackFrame { source_id, span }];

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

        let diagnostic = d::Diagnostic::error()
            .with_message(reason)
            .with_labels(labels)
            .with_notes(notes);

        term::emit(out, &config, sources, &diagnostic)?;

        if !backtrace.is_empty() {
            writeln!(out, "backtrace:")?;

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

                writeln!(out, "{}:{}:{}: {}", source.name(), line, line_count, text)?;
            }
        }

        Ok(())
    }
}

/// Helper to emit diagnostics for a warning.
fn warning_diagnostics_emit<'a, O>(
    this: &WarningDiagnostic,
    out: &mut O,
    sources: &'a Sources,
    config: &codespan_reporting::term::Config,
) -> Result<(), EmitDiagnosticsError>
where
    O: WriteColor,
{
    let mut notes = Vec::new();
    let mut labels = Vec::new();

    let context = match this.kind() {
        WarningDiagnosticKind::NotUsed { span, context } => {
            labels.push(d::Label::primary(this.source_id(), span.range()).with_message("not used"));

            *context
        }
        WarningDiagnosticKind::LetPatternMightPanic { span, context } => {
            labels.push(
                d::Label::primary(this.source_id(), span.range())
                    .with_message("let binding might panic"),
            );

            let binding = sources.source(this.source_id(), *span);

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
        WarningDiagnosticKind::TemplateWithoutExpansions { span, context } => {
            labels.push(
                d::Label::primary(this.source_id(), span.range())
                    .with_message("template string without expansions like `${1 + 2}`"),
            );

            *context
        }
        WarningDiagnosticKind::RemoveTupleCallParams {
            span,
            variant,
            context,
        } => {
            labels.push(
                d::Label::secondary(this.source_id(), span.range())
                    .with_message("constructing this variant could be done without parentheses"),
            );

            let variant = sources.source(this.source_id(), *variant);

            if let Some(variant) = variant {
                let mut note = String::new();
                writeln!(note, "Hint: Rewrite to `{}`", variant)?;
                notes.push(note);
            }

            *context
        }
        WarningDiagnosticKind::UnecessarySemiColon { span } => {
            labels.push(
                d::Label::primary(this.source_id(), span.range())
                    .with_message("unnecessary semicolon"),
            );

            None
        }
    };

    if let Some(context) = context {
        labels.push(
            d::Label::secondary(this.source_id(), context.range()).with_message("in this context"),
        );
    }

    let diagnostic = d::Diagnostic::warning()
        .with_message("warning")
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
) -> Result<(), EmitDiagnosticsError>
where
    O: WriteColor,
{
    let mut labels = Vec::new();
    let mut notes = Vec::new();

    let span = match this.kind() {
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
        FatalDiagnosticKind::ParseError(error) => error.span(),
        FatalDiagnosticKind::CompileError(error) => {
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
        FatalDiagnosticKind::QueryError(error) => {
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
        labels.push(d::Label::primary(this.source_id(), span.range()).with_message(e.to_string()));
    }

    let diagnostic = d::Diagnostic::error()
        .with_message(this.kind().to_string())
        .with_labels(labels)
        .with_notes(notes);

    term::emit(out, config, sources, &diagnostic)?;
    return Ok(());

    fn format_compile_error(
        this: &FatalDiagnostic,
        sources: &Sources,
        error_span: Span,
        kind: &CompileErrorKind,
        labels: &mut Vec<d::Label<SourceId>>,
        notes: &mut Vec<String>,
    ) -> fmt::Result {
        match kind {
            CompileErrorKind::QueryError { error } => {
                format_query_error(this, sources, error_span, error, labels, notes)?;
            }
            CompileErrorKind::DuplicateObjectKey { existing, object } => {
                labels.push(
                    d::Label::secondary(this.source_id(), existing.range())
                        .with_message("previously defined here"),
                );

                labels.push(
                    d::Label::secondary(this.source_id(), object.range())
                        .with_message("object being defined here"),
                );
            }
            CompileErrorKind::ModAlreadyLoaded { existing, .. } => {
                let (existing_source_id, existing_span) = *existing;

                labels.push(
                    d::Label::secondary(existing_source_id, existing_span.range())
                        .with_message("previously loaded here"),
                );
            }
            CompileErrorKind::ExpectedBlockSemiColon { followed_span } => {
                labels.push(
                    d::Label::secondary(this.source_id(), followed_span.range())
                        .with_message("because this immediately follows"),
                );

                let binding = sources.source(this.source_id(), error_span);

                if let Some(binding) = binding {
                    let mut note = String::new();
                    writeln!(note, "Hint: Rewrite to `{};`", binding)?;
                    notes.push(note);
                }
            }
            CompileErrorKind::VariableMoved { moved_at, .. } => {
                labels.push(
                    d::Label::secondary(this.source_id(), moved_at.range())
                        .with_message("moved here"),
                );
            }
            CompileErrorKind::CallMacroError { item, .. } => {
                notes.push(format!("Error originated in the `{}` macro", item));
            }
            CompileErrorKind::NestedTest { nested_span } => {
                labels.push(
                    d::Label::secondary(this.source_id(), nested_span.range())
                        .with_message("nested in here"),
                );
            }
            CompileErrorKind::NestedBench { nested_span } => {
                labels.push(
                    d::Label::secondary(this.source_id(), nested_span.range())
                        .with_message("nested in here"),
                );
            }
            _ => (),
        }

        Ok(())
    }

    fn format_query_error(
        this: &FatalDiagnostic,
        sources: &Sources,
        error_span: Span,
        kind: &QueryErrorKind,
        labels: &mut Vec<d::Label<SourceId>>,
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
                diagnose_import_path(&mut labels, path);
            }
            QueyrErrorKind::ImportRecursionLimit { path, .. } => {
                diagnose_import_path(&mut labels, path);
            }
            QueryErrorKind::ItemConflict {
                other: Location { source_id, span },
                ..
            } => {
                labels.push(
                    d::Label::secondary(*source_id, span.range())
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
                        d::Label::secondary(*source_id, span.range())
                            .with_message("re-exported here"),
                    );
                }

                labels.push(
                    d::Label::secondary(*source_id, span.range()).with_message("defined here"),
                );
            }
            QueryErrorKind::NotVisibleMod {
                chain,
                location: Location { source_id, span },
                ..
            } => {
                for Location { source_id, span } in chain {
                    labels.push(
                        d::Label::secondary(*source_id, span.range())
                            .with_message("re-exported here"),
                    );
                }

                labels.push(
                    d::Label::secondary(*source_id, span.range())
                        .with_message("module defined here"),
                );
            }
            QueryErrorKind::AmbiguousItem { locations, .. } => {
                for (Location { source_id, span }, item) in locations {
                    labels.push(
                        d::Label::secondary(*source_id, span.range())
                            .with_message(format!("here as `{}`", item)),
                    );
                }
            }
            _ => (),
        }

        Ok(())
    }

    fn format_ir_error(
        this: &FatalDiagnostic,
        sources: &Sources,
        error_span: Span,
        kind: &IrErrorKind,
        labels: &mut Vec<d::Label<SourceId>>,
        notes: &mut Vec<String>,
    ) -> fmt::Result {
        if let IrErrorKind::QueryError { error } = kind {
            format_query_error(this, sources, error_span, error, labels, notes)?;
        }

        Ok(())
    }

    fn format_resolve_error(
        _: &FatalDiagnostic,
        _: &Sources,
        _: Span,
        _: &ResolveErrorKind,
        _: &mut Vec<d::Label<SourceId>>,
        _: &mut Vec<String>,
    ) -> fmt::Result {
        Ok(())
    }

    fn diagnose_import_path(
        labels: &mut Vec<d::Label<SourceId>>,
        path: &[ImportStep],
    ) {
        let mut it = path.iter();
        let last = it.next_back();

        for (step, entry) in (1..).zip(it) {
            labels.push(
                d::Label::secondary(entry.location.source_id, entry.location.span.range())
                    .with_message(format!("step #{} for `{}`", step, entry.item)),
            );
        }

        if let Some(entry) = last {
            labels.push(
                d::Label::secondary(entry.location.source_id, entry.location.span.range())
                    .with_message(format!("final step cycling back to `{}`", entry.item)),
            );
        }
    }
}

impl EmitDiagnostics for FatalDiagnostic {
    fn emit_diagnostics<O>(
        &self,
        out: &mut O,
        sources: &Sources,
    ) -> Result<(), EmitDiagnosticsError>
    where
        O: WriteColor,
    {
        let config = codespan_reporting::term::Config::default();

        fatal_diagnostics_emit(self, out, sources, &config)
    }
}
