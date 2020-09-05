use crate::CompileError;
use runestick::{LinkerErrors, Source};
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
    /// Compiler error.
    #[error("compile error")]
    CompileError {
        /// The source error.
        #[source]
        error: CompileError,
        /// The source file we tried to compile.
        code_source: Source,
    },
    /// A linker error occured.
    #[error("linker error")]
    LinkError {
        /// Errors that happened during linking.
        errors: LinkerErrors,
        /// The file id of the link error.
        code_source: Source,
    },
}

impl LoadError {
    /// The kind of the load error.
    pub fn kind(&self) -> &LoadErrorKind {
        &self.kind
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
