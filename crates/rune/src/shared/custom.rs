use crate::{Span, Spanned, SpannedError};

/// A custom opaque error helper.
pub(crate) struct Custom {
    span: Span,
    message: &'static str,
}

impl Custom {
    /// Construct a new custom error.
    pub(crate) fn new<S>(spanned: S, message: &'static str) -> Self
    where
        S: Spanned,
    {
        Self {
            span: spanned.span(),
            message,
        }
    }

    /// Message of the custom error.
    pub(crate) fn message(&self) -> &'static str {
        self.message
    }
}

impl From<Custom> for SpannedError {
    fn from(error: Custom) -> Self {
        SpannedError::msg(error.span, error.message)
    }
}

impl Spanned for Custom {
    fn span(&self) -> Span {
        self.span
    }
}
