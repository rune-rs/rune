use crate::no_std as std;
use crate::no_std::prelude::*;
use crate::no_std::thiserror;

use thiserror::Error;

use crate::compile::{CompileError, CompileErrorKind};

error! {
    /// An error while constructing HIR representation.
    #[derive(Debug)]
    pub struct HirError {
        kind: HirErrorKind,
    }

    impl From<CompileError>;
}

/// The kind of a hir error.
#[derive(Debug, Error)]
#[allow(missing_docs)]
#[non_exhaustive]
pub(crate) enum HirErrorKind {
    #[error("{error}")]
    CompileError {
        #[source]
        #[from]
        error: Box<CompileErrorKind>,
    },
    #[error("{message}")]
    Custom { message: Box<str> },
    #[error("Writing arena slice out of bounds for index {index}")]
    ArenaWriteSliceOutOfBounds { index: usize },
    #[error("Allocation error for {requested} bytes")]
    ArenaAllocError { requested: usize },
    #[error("Pattern `..` is not supported in this location")]
    UnsupportedPatternRest,
}
