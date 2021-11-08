use crate::compiling::LinkerError;
use crate::{BuildError, CompileError, ParseError, QueryError};
use runestick::SourceId;
use std::error;
use std::fmt;
use thiserror::Error;

/// An error raised when using one of the `load_*` functions.
#[derive(Debug)]
pub struct Error {
    /// The source id of the error.
    pub(super) source_id: SourceId,
    /// The kind of the load error.
    pub(super) kind: Box<ErrorKind>,
}

impl Error {
    /// The source id where the error originates from.
    pub fn source_id(&self) -> SourceId {
        self.source_id
    }

    /// The kind of the load error.
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    /// Convert into the kind of the load error.
    pub fn into_kind(self) -> ErrorKind {
        *self.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.kind.source()
    }
}

#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum ErrorKind {
    #[error("parse error")]
    ParseError(
        #[from]
        #[source]
        ParseError,
    ),
    #[error("compile error")]
    CompileError(
        #[from]
        #[source]
        CompileError,
    ),
    #[error("query error")]
    QueryError(
        #[from]
        #[source]
        QueryError,
    ),
    #[error("linker error")]
    LinkError(
        #[from]
        #[source]
        LinkerError,
    ),
    #[error("builder error: {0}")]
    BuildError(
        #[from]
        #[source]
        BuildError,
    ),
    /// An internal error.
    #[error("internal error: {0}")]
    Internal(&'static str),
}
