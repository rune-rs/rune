use std::path::Path;
use std::io;

use thiserror::Error;

use crate::{SourceId};
use crate::ast::Span;

error! {
    /// An error raised when interacting with workspaces.
    #[derive(Debug)]
    pub struct WorkspaceError {
        kind: WorkspaceErrorKind,
    }
}

impl WorkspaceError {
    pub(crate) fn missing_field(span: Span, field: &'static str) -> Self {
        Self::new(span, WorkspaceErrorKind::MissingField { field })
    }

    pub(crate) fn expected_array(span: Span) -> Self {
        Self::new(span, WorkspaceErrorKind::ExpectedArray)
    }
}

/// A workspace error.
#[derive(Debug, Error)]
#[allow(missing_docs)]
#[non_exhaustive]
pub enum WorkspaceErrorKind {
    #[error("{message}")]
    Custom { message: Box<str> },
    #[error("Failed to load `{path}`: {error}")]
    FileError {
        path: Box<Path>,
        #[source]
        error: io::Error,
    },
    #[error("Failed to deserialize manifest: {error}")]
    Toml { #[from] error: toml::de::Error },
    #[error("Failed to deserialize: {error}")]
    Key { #[from] error: serde_hashkey::Error },
    #[error("Missing source id `{source_id}`")]
    MissingSourceId { source_id: SourceId },
    #[error("Missing required field `{field}`")]
    MissingField { field: &'static str },
    #[error("Expected array")]
    ExpectedArray,
    #[error("Element `[workspace]` can only be used in manifests with a valid path")]
    MissingManifestPath,
    #[error("Expected table")]
    ExpectedTable,
    #[error("Key `{key}` not supported")]
    UnsupportedKey { key: String },
}
