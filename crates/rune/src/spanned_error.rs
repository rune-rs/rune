use crate::Span;
use std::error;
use std::fmt;

/// Trait to coerce a result of a non-spanned error into a spanned error.
pub(crate) trait WithSpan<T> {
    /// Convert the given result into a result which produces a spanned error.
    fn with_span(self, span: Span) -> Result<T, SpannedError>;
}

/// Blanket implementation that is helpful.
impl<T, E> WithSpan<T> for Result<T, E>
where
    anyhow::Error: From<E>,
{
    fn with_span(self, span: Span) -> Result<T, SpannedError> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(SpannedError::new(span, error)),
        }
    }
}

/// An error with an associated span.
#[derive(Debug)]
pub struct SpannedError {
    span: Span,
    inner: anyhow::Error,
}

impl SpannedError {
    /// Construct a new error with the associated span.
    pub fn new<E>(span: Span, error: E) -> Self
    where
        anyhow::Error: From<E>,
    {
        Self {
            span,
            inner: anyhow::Error::from(error),
        }
    }

    /// Construct a new error out of the given message.
    pub fn msg<M>(span: Span, message: M) -> Self
    where
        M: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        Self {
            span,
            inner: anyhow::Error::msg(message),
        }
    }

    /// Convert into inner.
    pub fn into_inner(self) -> anyhow::Error {
        self.inner
    }

    /// Get the span of the error.
    pub fn span(&self) -> Span {
        self.span
    }
}

impl fmt::Display for SpannedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl error::Error for SpannedError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.inner.source()
    }
}
