//! Runtime helpers for loading code and emitting diagnostics.

use std::fmt;
use std::io;

use codespan_reporting::diagnostic as d;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::WriteColor;

use crate::alloc;
use crate::alloc::prelude::*;
use crate::ast::Spanned;
use crate::workspace::{Diagnostic, Diagnostics, FatalDiagnostic};
use crate::Sources;

/// Errors that can be raised when formatting diagnostics.
#[derive(Debug)]
#[non_exhaustive]
pub enum EmitError {
    /// Source Error.
    Io(io::Error),
    /// Allocation Error.
    Alloc(alloc::Error),
    /// Codespan reporting error.
    CodespanReporting(codespan_reporting::files::Error),
}

impl fmt::Display for EmitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EmitError::Io(..) => write!(f, "I/O error"),
            EmitError::Alloc(error) => error.fmt(f),
            EmitError::CodespanReporting(..) => write!(f, "codespan reporting error"),
        }
    }
}

impl From<io::Error> for EmitError {
    fn from(source: io::Error) -> Self {
        EmitError::Io(source)
    }
}

impl From<alloc::Error> for EmitError {
    fn from(error: alloc::Error) -> Self {
        EmitError::Alloc(error)
    }
}

impl From<codespan_reporting::files::Error> for EmitError {
    fn from(source: codespan_reporting::files::Error) -> Self {
        EmitError::CodespanReporting(source)
    }
}

cfg_std! {
    impl std::error::Error for EmitError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                EmitError::Io(error) => Some(error),
                EmitError::Alloc(error) => Some(error),
                EmitError::CodespanReporting(error) => Some(error),
            }
        }
    }
}

impl Diagnostics {
    /// Generate formatted diagnostics capable of referencing source lines and
    /// hints.
    ///
    /// See [prepare][crate::prepare] for how to use.
    pub fn emit<O>(&self, out: &mut O, sources: &Sources) -> Result<(), EmitError>
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
    let mut labels = rust_alloc::vec::Vec::new();

    let span = this.error().span();
    labels.push(
        d::Label::primary(this.source_id(), span.range())
            .with_message(this.error().try_to_string()?.into_std()),
    );

    let diagnostic = d::Diagnostic::error()
        .with_message(this.error().try_to_string()?.into_std())
        .with_labels(labels);

    term::emit(out, config, sources, &diagnostic)?;
    Ok(())
}
