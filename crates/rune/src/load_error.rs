use crate::{CompileError, ParseError};
use runestick::LinkerErrors;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// An error raised when using one of the `load_*` functions.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct LoadError {
    /// The kind of the load error.
    kind: Box<LoadErrorKind>,
}

impl LoadError {
    /// Construct an internal error.
    ///
    /// This should be used for programming invariants of the compiler which are
    /// broken for some reason.
    pub fn internal(message: &'static str) -> Self {
        Self {
            kind: Box::new(LoadErrorKind::Internal { message }),
        }
    }

    /// The kind of the load error.
    pub fn kind(&self) -> &LoadErrorKind {
        &self.kind
    }

    /// Convert into the kind of the load error.
    pub fn into_kind(self) -> LoadErrorKind {
        *self.kind
    }
}

impl<E> From<E> for LoadError
where
    LoadErrorKind: From<E>,
{
    fn from(err: E) -> Self {
        Self {
            kind: Box::new(LoadErrorKind::from(err)),
        }
    }
}

/// The kind of the load error.
#[derive(Debug, Error)]
pub enum LoadErrorKind {
    /// Failed to read the given file.
    #[error("failed to read file: {path}: {error}")]
    ReadFile {
        /// The source error.
        #[source]
        error: io::Error,
        /// The path that we couldn't read.
        path: PathBuf,
    },
    /// Parse error.
    #[error("parse error")]
    ParseError {
        /// The source error.
        #[source]
        error: ParseError,
        /// The source id of the error.
        source_id: usize,
    },
    /// Compiler error.
    #[error("compile error")]
    CompileError {
        /// The source error.
        #[source]
        error: CompileError,
        /// The source id of the error.
        source_id: usize,
    },
    /// A linker error occured.
    #[error("linker error")]
    LinkError {
        /// Errors that happened during linking.
        errors: LinkerErrors,
    },
    /// An internal error.
    #[error("internal error: {message}")]
    Internal {
        /// The message of the internal error.
        message: &'static str,
    },
}
