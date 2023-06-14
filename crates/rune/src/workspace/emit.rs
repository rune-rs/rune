//! Runtime helpers for loading code and emitting diagnostics.

use std::fmt;
use std::io;

use crate::no_std::prelude::*;

use codespan_reporting::diagnostic as d;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::WriteColor;
pub use codespan_reporting::term::termcolor;

use crate::{Sources};
use crate::ast::{Spanned};
use crate::workspace::{Diagnostics, Diagnostic, FatalDiagnostic};

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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EmitError::Io(..) => write!(f, "I/O error"),
            EmitError::Fmt(..) => write!(f, "formatting error"),
            EmitError::CodespanReporting(..) => write!(f, "codespan reporting error"),
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
    fn source(&self) -> Option<&(dyn crate::no_std::error::Error + 'static)> {
        match self {
            EmitError::Io(error) => Some(error),
            EmitError::Fmt(error) => Some(error),
            EmitError::CodespanReporting(error) => Some(error),
        }
    }
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
