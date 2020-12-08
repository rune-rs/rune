use runestick::Span;

/// A result extension for associating a span with an error.
pub(crate) trait ResultExt<T, E> {
    /// Associate the given span with the current error.
    fn with_span(self, span: Span) -> Result<T, WithSpan<E>>;
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    fn with_span(self, span: Span) -> Result<T, WithSpan<E>> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(WithSpan { error, span }),
        }
    }
}

/// An error that has a span associated with it.
pub(crate) struct WithSpan<E> {
    /// The span associated with the error.
    pub(crate) span: Span,
    /// The wrapped error.
    pub(crate) error: E,
}
