use core::fmt;

use crate::alloc::{Box, String};
#[cfg(feature = "emit")]
use crate::ast::{Span, Spanned};
use crate::compile::{self, LinkerError};
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

    /// If this fatal diagnostic is a compile error, return it.
    pub fn as_compile_error(&self) -> Option<&compile::Error> {
        match &*self.kind {
            FatalDiagnosticKind::CompileError(error) => Some(error),
            _ => None,
        }
    }

    /// If this fatal diagnostic is a link error, return it.
    pub fn as_link_error(&self) -> Option<&LinkerError> {
        match &*self.kind {
            FatalDiagnosticKind::LinkError(error) => Some(error),
            _ => None,
        }
    }

    /// The kind of the load error.
    #[cfg(any(feature = "emit", feature = "languageserver"))]
    pub(crate) fn kind(&self) -> &FatalDiagnosticKind {
        &self.kind
    }

    /// The kind of the load error.
    #[cfg(test)]
    pub(crate) fn into_kind(self) -> FatalDiagnosticKind {
        Box::into_inner(self.kind)
    }

    #[cfg(feature = "emit")]
    pub(crate) fn span(&self) -> Option<Span> {
        match &*self.kind {
            FatalDiagnosticKind::CompileError(error) => Some(error.span()),
            FatalDiagnosticKind::LinkError(..) => None,
            FatalDiagnosticKind::Custom(..) => None,
        }
    }
}

impl fmt::Display for FatalDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl core::error::Error for FatalDiagnostic {
    #[inline]
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match &*self.kind {
            FatalDiagnosticKind::CompileError(error) => Some(error),
            FatalDiagnosticKind::LinkError(error) => Some(error),
            _ => None,
        }
    }
}

/// The kind of a [FatalDiagnostic].
#[derive(Debug)]
#[allow(missing_docs)]
#[non_exhaustive]
pub(crate) enum FatalDiagnosticKind {
    /// A compilation error.
    CompileError(compile::Error),
    /// A linking error.
    LinkError(LinkerError),
    /// An internal error.
    Custom(String),
}

impl fmt::Display for FatalDiagnosticKind {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FatalDiagnosticKind::CompileError(error) => error.fmt(f),
            FatalDiagnosticKind::LinkError(error) => error.fmt(f),
            FatalDiagnosticKind::Custom(message) => message.fmt(f),
        }
    }
}

impl From<compile::Error> for FatalDiagnosticKind {
    #[inline]
    fn from(error: compile::Error) -> Self {
        FatalDiagnosticKind::CompileError(error)
    }
}

impl From<LinkerError> for FatalDiagnosticKind {
    #[inline]
    fn from(error: LinkerError) -> Self {
        FatalDiagnosticKind::LinkError(error)
    }
}
