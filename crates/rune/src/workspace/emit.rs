//! Runtime helpers for loading code and emitting diagnostics.

use crate::{Sources};
use crate::ast::{Spanned};
use std::fmt;
use std::io;
use thiserror::Error;
use codespan_reporting::diagnostic as d;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::WriteColor;
pub use codespan_reporting::term::termcolor;
use crate::workspace::{Diagnostics, Diagnostic, FatalDiagnostic};

/// Errors that can be raised when formatting diagnostics.
#[derive(Debug, Error)]
pub enum EmitError {
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

        for diagnostic in &self.diagnostics {
            match diagnostic {
                Diagnostic::Fatal(e) => {
                    error_diagnostics_emit(e, out, sources, &config)?;
                }
            }
        }

        Ok(())
    }
}

/// Custom shared helper for emitting diagnostics for a single error.
fn error_diagnostics_emit<O>(
    this: &FatalDiagnostic,
    out: &mut O,
    sources: &Sources,
    config: &codespan_reporting::term::Config,
) -> Result<(), EmitError>
where
    O: WriteColor,
{
    let mut labels = Vec::new();

    let span = this.error().span();
    labels.push(d::Label::primary(this.source_id(), span.range()).with_message(this.error().to_string()));

    let diagnostic = d::Diagnostic::error()
        .with_message(this.error().to_string())
        .with_labels(labels);

    term::emit(out, config, sources, &diagnostic)?;
    Ok(())
}
