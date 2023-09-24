use crate::ast::{Span, Spanned};

/// Helper trait to coerce errors which do not carry a span into spanned ones.
///
/// This is primarily used to convert errors into
/// [compile::Error][crate::compile::Error].
///
/// This has a blanked implementation over [`Result<T, E>`].
pub trait WithSpan<T, E> {
    /// Convert the given result into a result which produces a spanned error.
    fn with_span<S>(self, spanned: S) -> Result<T, HasSpan<S, E>>
    where
        S: Spanned;
}

impl<T, E> WithSpan<T, E> for Result<T, E> {
    /// Attach the span extracted from `spanned` to the error if it is present.
    fn with_span<S>(self, span: S) -> Result<T, HasSpan<S, E>>
    where
        S: Spanned,
    {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(HasSpan::new(span, error)),
        }
    }
}

/// An error with an associated span.
#[derive(Debug)]
pub struct HasSpan<S, E> {
    span: S,
    error: E,
}

impl<S, E> HasSpan<S, E> {
    pub(crate) fn new(span: S, error: E) -> Self {
        Self { span, error }
    }

    pub(crate) fn span(&self) -> Span
    where
        S: Spanned,
    {
        self.span.span()
    }

    pub(crate) fn into_inner(self) -> E {
        self.error
    }
}
