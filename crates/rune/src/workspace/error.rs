use core::fmt;

use std::path::Path;

use crate::alloc::{self, Box, String};
use crate::ast::{Span, Spanned};
use crate::compile::HasSpan;
use crate::source;
use crate::workspace::glob;
use crate::SourceId;

/// An error raised when interacting with workspaces.
#[derive(Debug)]
pub struct WorkspaceError {
    span: Span,
    kind: rust_alloc::boxed::Box<WorkspaceErrorKind>,
}

impl WorkspaceError {
    /// Construct a new workspace error with the given span and kind.
    #[inline]
    pub(crate) fn new<S, K>(spanned: S, kind: K) -> Self
    where
        S: Spanned,
        WorkspaceErrorKind: From<K>,
    {
        Self {
            span: spanned.span(),
            kind: rust_alloc::boxed::Box::new(WorkspaceErrorKind::from(kind)),
        }
    }

    /// Construct a custom message as an error.
    #[inline]
    pub fn msg<S, M>(spanned: S, message: M) -> Self
    where
        S: Spanned,
        M: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        Self::new(
            spanned,
            WorkspaceErrorKind::Custom {
                error: anyhow::Error::msg(message),
            },
        )
    }
}

impl Spanned for WorkspaceError {
    #[inline]
    fn span(&self) -> Span {
        self.span
    }
}

impl core::error::Error for WorkspaceError {
    #[inline]
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.kind.source()
    }
}

impl fmt::Display for WorkspaceError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.kind, f)
    }
}

impl WorkspaceError {
    #[inline]
    pub(crate) fn missing_field(span: Span, field: &'static str) -> Self {
        Self::new(span, WorkspaceErrorKind::MissingField { field })
    }

    #[inline]
    pub(crate) fn expected_array(span: Span) -> Self {
        Self::new(span, WorkspaceErrorKind::ExpectedArray)
    }
}

impl<S, E> From<HasSpan<S, E>> for WorkspaceError
where
    S: Spanned,
    WorkspaceErrorKind: From<E>,
{
    #[inline]
    fn from(spanned: HasSpan<S, E>) -> Self {
        Self::new(spanned.span(), spanned.into_inner())
    }
}

/// A workspace error.
#[derive(Debug)]
#[allow(missing_docs)]
#[non_exhaustive]
pub(crate) enum WorkspaceErrorKind {
    Custom {
        error: anyhow::Error,
    },
    GlobError {
        path: Box<Path>,
        error: glob::GlobError,
    },
    Source {
        path: Box<Path>,
        error: source::FromPathError,
    },
    Toml {
        error: toml::de::Error,
    },
    Key {
        error: serde_hashkey::Error,
    },
    MissingSourceId {
        source_id: SourceId,
    },
    MissingField {
        field: &'static str,
    },
    ExpectedArray,
    MissingManifestPath,
    ExpectedTable,
    UnsupportedKey {
        key: String,
    },
    AllocError {
        error: alloc::Error,
    },
}

impl core::error::Error for WorkspaceErrorKind {
    #[inline]
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            WorkspaceErrorKind::GlobError { error, .. } => Some(error),
            WorkspaceErrorKind::Source { error, .. } => Some(error),
            WorkspaceErrorKind::Toml { error, .. } => Some(error),
            WorkspaceErrorKind::Key { error, .. } => Some(error),
            _ => None,
        }
    }
}

impl fmt::Display for WorkspaceErrorKind {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WorkspaceErrorKind::Custom { error } => error.fmt(f),
            WorkspaceErrorKind::GlobError { path, error } => write!(
                f,
                "Failed to glob at `{path}`: {error}",
                path = path.display()
            ),
            WorkspaceErrorKind::Source { path, error } => write!(
                f,
                "Failed to load source at `{path}`: {error}",
                path = path.display()
            ),
            WorkspaceErrorKind::Toml { error } => {
                write!(f, "Failed to deserialize manifest: {error}",)
            }
            WorkspaceErrorKind::Key { error } => {
                write!(f, "Failed to deserialize: {error}")
            }
            WorkspaceErrorKind::MissingSourceId { source_id } => {
                write!(f, "Missing source id `{source_id}`",)
            }
            WorkspaceErrorKind::MissingField { field } => {
                write!(f, "Missing required field `{field}`",)
            }
            WorkspaceErrorKind::ExpectedArray => write!(f, "Expected array"),
            WorkspaceErrorKind::MissingManifestPath => write!(
                f,
                "Element `[workspace]` can only be used in manifests with a valid path"
            ),
            WorkspaceErrorKind::ExpectedTable => write!(f, "Expected table"),
            WorkspaceErrorKind::UnsupportedKey { key } => write!(f, "Key `{key}` not supported",),
            WorkspaceErrorKind::AllocError { error } => error.fmt(f),
        }
    }
}

impl From<anyhow::Error> for WorkspaceErrorKind {
    #[inline]
    fn from(error: anyhow::Error) -> Self {
        WorkspaceErrorKind::Custom { error }
    }
}

impl From<toml::de::Error> for WorkspaceErrorKind {
    #[inline]
    fn from(error: toml::de::Error) -> Self {
        WorkspaceErrorKind::Toml { error }
    }
}

impl From<serde_hashkey::Error> for WorkspaceErrorKind {
    #[inline]
    fn from(error: serde_hashkey::Error) -> Self {
        WorkspaceErrorKind::Key { error }
    }
}

impl From<alloc::Error> for WorkspaceError {
    #[inline]
    fn from(error: alloc::Error) -> Self {
        WorkspaceError::new(Span::empty(), error)
    }
}

impl From<alloc::Error> for WorkspaceErrorKind {
    #[inline]
    fn from(error: alloc::Error) -> Self {
        WorkspaceErrorKind::AllocError { error }
    }
}
