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
pub enum HirErrorKind {
    #[error("{message}")]
    Custom { message: &'static str },
    #[error("writing arena slice out of bounds for index {index}")]
    ArenaWriteSliceOutOfBounds { index: usize },
    #[error("allocation error for {requested} bytes")]
    ArenaAllocError { requested: usize },
    #[error("`..` is not supported in this location")]
    UnsupportedPatternRest,
    #[error("{error}")]
    QueryError {
        #[source]
        #[from]
        error: Box<QueryErrorKind>,
    },
}
