use crate::{SourceId};
use crate::workspace::WorkspaceError;

/// A reported diagnostic error.
pub(crate) struct FatalDiagnostic {
    pub(crate) source_id: SourceId,
    pub(crate) error: WorkspaceError,
}

/// A single workspace diagnostic.
pub(crate) enum Diagnostic {
    /// An error in a workspace.
    Fatal(FatalDiagnostic),
}

/// Diagnostics emitted about a workspace.
#[derive(Default)]
pub struct Diagnostics {
    pub(crate) diagnostics: Vec<Diagnostic>,
}

impl Diagnostics {
    /// Report a single workspace error.
    pub fn fatal(&mut self, source_id: SourceId, error: WorkspaceError) {
        self.diagnostics.push(Diagnostic::Fatal(FatalDiagnostic {
            source_id,
            error,
        }))
    }

    /// Test if diagnostics has errors.
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|e| matches!(e, Diagnostic::Fatal(..)))
    }

    /// Test if diagnostics is empty.
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

impl Diagnostics {
    /// Construct an empty diagnostics container.
    pub fn new() -> Self {
        Self::default()
    }
}
