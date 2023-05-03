use core::fmt;

use crate::no_std;

use crate::ast::{Span, Spanned};

/// Trait to coerce a result of a non-spanned error into a spanned error.
pub(crate) trait WithSpan<T> {
    /// Convert the given result into a result which produces a spanned error.
    fn with_span(self, span: Span) -> Result<T, SpannedError>;
}

/// Blanket implementation that is helpful.
impl<T, E> WithSpan<T> for Result<T, E>
where
    no_std::Error: From<E>,
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
    inner: no_std::Error,
}

impl SpannedError {
    /// Construct a new error with the associated span.
    pub fn new<E>(span: Span, error: E) -> Self
    where
        no_std::Error: From<E>,
    {
        Self {
            span,
            inner: no_std::Error::from(error),
        }
    }

    /// Construct a new error out of the given message.
    pub fn msg<M>(span: Span, message: M) -> Self
    where
        M: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        Self {
            span,
            inner: no_std::Error::msg(message),
        }
    }

    /// Convert into inner.
    pub(crate) fn into_inner(self) -> no_std::Error {
        self.inner
    }
}

impl Spanned for SpannedError {
    fn span(&self) -> Span {
        self.span
    }
}

impl fmt::Display for SpannedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl crate::no_std::error::Error for SpannedError {
    fn source(&self) -> Option<&(dyn crate::no_std::error::Error + 'static)> {
        self.inner.source()
    }
}
