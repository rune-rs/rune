use core::fmt;

use crate::no_std as std;
use crate::no_std::prelude::*;
use crate::no_std::thiserror;

use thiserror::Error;

#[cfg(feature = "emit")]
use crate::ast::{Span, Spanned};
use crate::compile::{CompileError, LinkerError};
use crate::SourceId;

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

    /// The kind of the load error.
    #[cfg(test)]
    pub(crate) fn into_kind(self) -> FatalDiagnosticKind {
        *self.kind
    }

    #[cfg(feature = "emit")]
    pub(crate) fn span(&self) -> Option<Span> {
        match &*self.kind {
            FatalDiagnosticKind::CompileError(error) => Some(error.span()),
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

impl crate::no_std::error::Error for FatalDiagnostic {
    #[inline]
    fn source(&self) -> Option<&(dyn crate::no_std::error::Error + 'static)> {
        self.kind.source()
    }
}

/// The kind of a [FatalDiagnostic].
#[derive(Debug, Error)]
#[allow(missing_docs)]
#[non_exhaustive]
pub enum FatalDiagnosticKind {
    #[error("compile error")]
    CompileError(
        #[from]
        #[source]
        CompileError,
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
