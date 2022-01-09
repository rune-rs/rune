use crate::ast::{Span, Spanned};
use crate::compile::{CompileError, LinkerError};
use crate::parse::ParseError;
use crate::query::QueryError;
use crate::SourceId;
use std::error;
use std::fmt;
use thiserror::Error;

/// Fatal diagnostic emitted during compilation. Fatal diagnostics indicates an
/// unrecoverable issue.
#[derive(Debug)]
pub struct FatalDiagnostic {
    /// The source id of the error.
    pub(crate) source_id: SourceId,
    /// The kind of the load error.
    pub(crate) kind: Box<FatalDiagnosticKind>,
}

impl FatalDiagnostic {
    /// The source id where the error originates from.
    pub fn source_id(&self) -> SourceId {
        self.source_id
    }

    /// The kind of the load error.
    pub fn kind(&self) -> &FatalDiagnosticKind {
        &self.kind
    }

    /// Convert into the kind of the load error.
    pub fn into_kind(self) -> FatalDiagnosticKind {
        *self.kind
    }

    pub(crate) fn span(&self) -> Option<Span> {
        match &*self.kind {
            FatalDiagnosticKind::ParseError(error) => Some(error.span()),
            FatalDiagnosticKind::CompileError(error) => Some(error.span()),
            FatalDiagnosticKind::QueryError(error) => Some(error.span()),
            FatalDiagnosticKind::LinkError(..) => None,
            FatalDiagnosticKind::Internal(..) => None,
        }
    }
}

impl fmt::Display for FatalDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl error::Error for FatalDiagnostic {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.kind.source()
    }
}

/// The kind of a [FatalDiagnostic].
#[derive(Debug, Error)]
#[allow(missing_docs)]
#[non_exhaustive]
pub enum FatalDiagnosticKind {
    #[error("parse error")]
    ParseError(
        #[from]
        #[source]
        ParseError,
    ),
    #[error("compile error")]
    CompileError(
        #[from]
        #[source]
        CompileError,
    ),
    #[error("query error")]
    QueryError(
        #[from]
        #[source]
        QueryError,
    ),
    #[error("linker error")]
    LinkError(
        #[from]
        #[source]
        LinkerError,
    ),
    /// An internal error.
    #[error("internal error: {0}")]
    Internal(&'static str),
}
