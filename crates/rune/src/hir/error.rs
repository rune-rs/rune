use thiserror::Error;

error! {
    /// An error while constructing HIR representation.
    #[derive(Debug, Clone)]
    pub struct HirError {
        kind: HirErrorKind,
    }
}

/// The kind of a hir error.
#[derive(Debug, Clone, Copy, Error)]
#[allow(missing_docs)]
#[non_exhaustive]
pub enum HirErrorKind {
    #[error("{message}")]
    Custom { message: &'static str },
    #[error("writing arena slice out of bounds for index {index}")]
    ArenaWriteSliceOutOfBounds { index: usize },
    #[error("allocation error for {requested} bytes")]
    ArenaAllocError { requested: usize },
}
