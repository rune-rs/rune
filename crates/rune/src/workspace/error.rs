use core::fmt;

use crate::no_std::prelude::*;
use crate::no_std::thiserror;

use crate::no_std::error;
use crate::no_std::path::Path;
use crate::no_std::io;

use thiserror::Error;

use crate::{SourceId};
use crate::ast::{Span, Spanned};

/// An error raised when interacting with workspaces.
#[derive(Debug)]
pub struct WorkspaceError {
    span: Span,
    kind: Box<WorkspaceErrorKind>,
}

impl WorkspaceError {
    /// Construct a new workspace error with the given span and kind.
    #[allow(unused)]
    pub(crate) fn new<S, K>(spanned: S, kind: K) -> Self
    where
        S: Spanned,
        WorkspaceErrorKind: From<K>,
    {
        Self {
            span: spanned.span(),
            kind: Box::new(WorkspaceErrorKind::from(kind)),
        }
    }

    /// Construct a custom message as an error.
    pub fn msg<S, M>(spanned: S, message: M) -> Self
    where
        S: Spanned,
        M: fmt::Display,
    {
        Self::new(spanned, WorkspaceErrorKind::Custom { message: message.to_string().into() })
    }
}

impl Spanned for WorkspaceError {
    #[inline]
    fn span(&self) -> Span {
        self.span
    }
}

impl error::Error for WorkspaceError {
    #[inline]
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.kind.source()
    }
}

impl fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.kind, f)
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
pub(crate) enum WorkspaceErrorKind {
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
