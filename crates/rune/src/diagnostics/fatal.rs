use core::fmt;

use rust_alloc::boxed::Box;

use crate::ast::Spanned;
use crate::compile;
use crate::SourceId;

use super::PolicyDiagnostic;

/// Fatal diagnostic emitted during compilation. Fatal diagnostics indicates an
/// unrecoverable issue.
#[derive(Debug, Spanned)]
#[rune(crate)]
pub struct FatalDiagnostic {
    /// The source id of the error.
    pub(crate) source_id: SourceId,
    /// The kind of the load error.
    #[rune(span)]
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
            FatalDiagnosticKind::Policy(error) => Some(error),
        }
    }
}

/// The kind of a [FatalDiagnostic].
#[derive(Debug, Spanned)]
#[rune(crate)]
pub(crate) enum FatalDiagnosticKind {
    Compile(compile::Error),
    Policy(PolicyDiagnostic),
}

impl fmt::Display for FatalDiagnosticKind {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FatalDiagnosticKind::Compile(error) => error.fmt(f),
            FatalDiagnosticKind::Policy(policy) => policy.fmt(f),
        }
    }
}

impl From<compile::Error> for FatalDiagnosticKind {
    #[inline]
    fn from(error: compile::Error) -> Self {
        FatalDiagnosticKind::Compile(error)
    }
}
