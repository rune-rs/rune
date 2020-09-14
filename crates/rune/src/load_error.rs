use crate::unit_builder::LinkerError;
use crate::{CompileError, ParseError};
use thiserror::Error;

/// A collection of errors.
#[derive(Debug)]
pub struct Errors {
    errors: Vec<LoadError>,
}

impl Errors {
    /// Construct a new collection of errors.
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Push an error to the collection.
    pub fn push(&mut self, error: LoadError) {
        self.errors.push(error);
    }

    /// Test if the collection of errors is empty.
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }
}

impl IntoIterator for Errors {
    type Item = LoadError;
    type IntoIter = std::vec::IntoIter<LoadError>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.into_iter()
    }
}

/// An error raised when using one of the `load_*` functions.
#[derive(Debug)]
pub struct LoadError {
    /// The source id of the error.
    source_id: usize,
    /// The kind of the load error.
    kind: Box<LoadErrorKind>,
}

impl LoadError {
    /// Construct a new load error.
    pub fn new<E>(source_id: usize, err: E) -> Self
    where
        LoadErrorKind: From<E>,
    {
        Self {
            source_id,
            kind: Box::new(LoadErrorKind::from(err)),
        }
    }

    /// Construct an internal error.
    ///
    /// This should be used for programming invariants of the compiler which are
    /// broken for some reason.
    pub fn internal(source_id: usize, message: &'static str) -> Self {
        Self {
            source_id,
            kind: Box::new(LoadErrorKind::Internal(message)),
        }
    }

    /// The source id where the error originates from.
    pub fn source_id(&self) -> usize {
        self.source_id
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

/// The kind of the load error.
#[derive(Debug, Error)]
pub enum LoadErrorKind {
    /// Parse error.
    #[error("parse error")]
    ParseError(
        #[from]
        #[source]
        ParseError,
    ),
    /// Compiler error.
    #[error("compile error")]
    CompileError(
        #[from]
        #[source]
        CompileError,
    ),
    /// A linker error occured.
    #[error("linker error")]
    LinkError(
        #[from]
        #[source]
        LinkerError,
    ),
    /// An internal error.
    #[error("internal error: {0}")]
    Internal(&'static str),
}
