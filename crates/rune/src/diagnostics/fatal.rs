use core::fmt;

use rust_alloc::boxed::Box;

#[cfg(feature = "emit")]
use crate::ast::{Span, Spanned};
use crate::compile::{self, LinkerError};
use crate::SourceId;

use super::PolicyDiagnostic;

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
    pub(crate) fn kind(&self) -> &FatalDiagnosticKind {
        &self.kind
    }

    /// The kind of the load error.
    #[cfg(test)]
    pub(crate) fn into_kind(self) -> FatalDiagnosticKind {
        *self.kind
    }

    /// Get the span of the diagnostic if available.
    pub fn span(&self) -> Option<Span> {
        match &*self.kind {
            FatalDiagnosticKind::Compile(error) => Some(error.span()),
            FatalDiagnosticKind::Policy(policy) => Some(policy.span),
            FatalDiagnosticKind::Linker(..) => None,
            FatalDiagnosticKind::Internal(..) => None,
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
            FatalDiagnosticKind::Compile(error) => Some(error),
            FatalDiagnosticKind::Linker(error) => Some(error),
            _ => None,
        }
    }
}

/// The kind of a [FatalDiagnostic].
#[derive(Debug)]
#[allow(missing_docs)]
#[non_exhaustive]
pub(crate) enum FatalDiagnosticKind {
    Compile(compile::Error),
    Policy(PolicyDiagnostic),
    Linker(LinkerError),
    /// An internal error.
    Internal(&'static str),
}

impl fmt::Display for FatalDiagnosticKind {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FatalDiagnosticKind::Compile(error) => error.fmt(f),
            FatalDiagnosticKind::Policy(policy) => policy.fmt(f),
            FatalDiagnosticKind::Linker(error) => error.fmt(f),
            FatalDiagnosticKind::Internal(message) => message.fmt(f),
        }
    }
}

impl From<compile::Error> for FatalDiagnosticKind {
    #[inline]
    fn from(error: compile::Error) -> Self {
        FatalDiagnosticKind::Compile(error)
    }
}

impl From<LinkerError> for FatalDiagnosticKind {
    #[inline]
    fn from(error: LinkerError) -> Self {
        FatalDiagnosticKind::Linker(error)
    }
}
