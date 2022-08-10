use std::path::Path;
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
    #[error("manifest deserialization: {error}")]
    Toml { #[from] #[source] error: toml::de::Error },
    #[error("manifest serializationo: {error}")]
    Key { #[from] #[source] error: serde_hashkey::Error },
    #[error("failed to read `{path}`: {error}")]
    SourceError { path: Box<Path>, error: std::io::Error },
    #[error("custom: {message}")]
    Custom { message: Box<str> },
    #[error("missing source id `{source_id}`")]
    MissingSourceId { source_id: SourceId },
    #[error("missing required field `{field}`")]
    MissingField { field: &'static str },
    #[error("expected array")]
    ExpectedArray,
    #[error("[workspace] elements can only be used in manifests with a valid path")]
    MissingManifestPath,
    #[error("expected table")]
    ExpectedTable,
    #[error("key not supported")]
    UnsupportedKey,
}
