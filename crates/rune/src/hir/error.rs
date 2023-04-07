use thiserror::Error;

use crate::query::{QueryError, QueryErrorKind};

error! {
    /// An error while constructing HIR representation.
    #[derive(Debug)]
    pub struct HirError {
        kind: HirErrorKind,
    }

    impl From<QueryError>;
}

/// The kind of a hir error.
#[derive(Debug, Error)]
#[allow(missing_docs)]
#[non_exhaustive]
pub(crate) enum HirErrorKind {
    #[error("{error}")]
    QueryError {
        #[source]
        #[from]
        error: Box<QueryErrorKind>,
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
