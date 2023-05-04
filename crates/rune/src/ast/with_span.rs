use crate::ast::{Span, Spanned};

/// Trait to coerce a result of a non-spanned error into a spanned error.
pub(crate) trait WithSpanExt<T, E> {
    /// Convert the given result into a result which produces a spanned error.
    fn with_span<S>(self, spanned: S) -> Result<T, WithSpan<E>>
    where
        S: Spanned;
}

/// Blanket implementation that is helpful.
impl<T, E> WithSpanExt<T, E> for Result<T, E> {
    fn with_span<S>(self, spanned: S) -> Result<T, WithSpan<E>>
    where
        S: Spanned,
    {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(WithSpan::new(spanned.span(), error)),
        }
    }
}

/// An error with an associated span.
#[derive(Debug)]
pub(crate) struct WithSpan<E> {
    pub(crate) span: Span,
    pub(crate) error: E,
}

impl<E> WithSpan<E> {
    /// Construct a new error with the associated span.
    pub(crate) fn new(span: Span, error: E) -> Self {
        Self { span, error }
    }
}
