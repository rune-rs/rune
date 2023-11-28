use crate::alloc::{self, Vec};
use crate::workspace::WorkspaceError;
use crate::SourceId;

/// A fatal diagnostic in a workspace.
#[derive(Debug)]
pub struct FatalDiagnostic {
    source_id: SourceId,
    error: WorkspaceError,
}

impl FatalDiagnostic {
    /// Get source id of the diagnostic.
    pub fn source_id(&self) -> SourceId {
        self.source_id
    }

    /// Access the underlying workspace error.
    pub fn error(&self) -> &WorkspaceError {
        &self.error
    }
}

/// A single workspace diagnostic.
#[derive(Debug)]
#[non_exhaustive]
pub enum Diagnostic {
    /// An error in a workspace.
    Fatal(FatalDiagnostic),
}

/// Diagnostics emitted about a workspace.
#[derive(Default)]
pub struct Diagnostics {
    pub(crate) diagnostics: Vec<Diagnostic>,
}

impl Diagnostics {
    /// Access underlying diagnostics.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Test if diagnostics has errors.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|e| matches!(e, Diagnostic::Fatal(..)))
    }

    /// Test if diagnostics is empty.
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Report a single workspace error.
    pub(crate) fn fatal(
        &mut self,
        source_id: SourceId,
        error: WorkspaceError,
    ) -> alloc::Result<()> {
        self.diagnostics
            .try_push(Diagnostic::Fatal(FatalDiagnostic { source_id, error }))
    }
}

impl Diagnostics {
    /// Construct an empty diagnostics container.
    pub fn new() -> Self {
        Self::default()
    }
}
