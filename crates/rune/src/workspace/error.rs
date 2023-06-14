use core::fmt;

use crate::no_std::prelude::*;

use crate::no_std::path::Path;
use crate::no_std::io;

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

impl crate::no_std::error::Error for WorkspaceError {
    #[inline]
    fn source(&self) -> Option<&(dyn crate::no_std::error::Error + 'static)> {
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
#[derive(Debug)]
#[allow(missing_docs)]
#[non_exhaustive]
pub(crate) enum WorkspaceErrorKind {
    Custom { message: Box<str> },
    FileError {
        path: Box<Path>,
        error: io::Error,
    },
    Toml { error: toml::de::Error },
    Key { error: serde_hashkey::Error },
    MissingSourceId { source_id: SourceId },
    MissingField { field: &'static str },
    ExpectedArray,
    MissingManifestPath,
    ExpectedTable,
    UnsupportedKey { key: String },
}

impl crate::no_std::error::Error for WorkspaceErrorKind {
    fn source(&self) -> Option<&(dyn crate::no_std::error::Error + 'static)> {
        match self {
            WorkspaceErrorKind::FileError { error, .. } => {
                Some(error)
            }
            WorkspaceErrorKind::Toml { error, .. } => {
                Some(error)
            }
            WorkspaceErrorKind::Key { error, .. } => {
                Some(error)
            }
            _ => None,
        }
    }
}

impl fmt::Display for WorkspaceErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WorkspaceErrorKind::Custom { message } => {
                write!(f, "{message}")
            }
            WorkspaceErrorKind::FileError { path, error } => write!(
                f,
                "Failed to load `{path}`: {error}", path = path.display()
            ),
            WorkspaceErrorKind::Toml { error } => write!(
                f,
                "Failed to deserialize manifest: {error}",
            ),
            WorkspaceErrorKind::Key { error } => write!(
                f,
                "Failed to deserialize: {error}",
                error = error
            ),
            WorkspaceErrorKind::MissingSourceId { source_id } => write!(
                f,
                "Missing source id `{source_id}`",
            ),
            WorkspaceErrorKind::MissingField { field } => write!(
                f,
                "Missing required field `{field}`",
            ),
            WorkspaceErrorKind::ExpectedArray {} => write!(f, "Expected array"),
            WorkspaceErrorKind::MissingManifestPath {} => write!(
                f,
                "Element `[workspace]` can only be used in manifests with a valid path"
            ),
            WorkspaceErrorKind::ExpectedTable {} => write!(f, "Expected table"),
            WorkspaceErrorKind::UnsupportedKey { key } => write!(
                f,
                "Key `{key}` not supported",
            ),
        }
    }
}

impl From<toml::de::Error> for WorkspaceErrorKind {
    #[allow(deprecated)]
    fn from(source: toml::de::Error) -> Self {
        WorkspaceErrorKind::Toml { error: source }
    }
}

impl From<serde_hashkey::Error> for WorkspaceErrorKind {
    #[allow(deprecated)]
    fn from(source: serde_hashkey::Error) -> Self {
        WorkspaceErrorKind::Key { error: source }
    }
}