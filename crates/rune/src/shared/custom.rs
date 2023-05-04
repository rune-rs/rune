use crate::ast::{Span, Spanned};

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

impl Spanned for Custom {
    #[inline]
    fn span(&self) -> Span {
        self.span
    }
}
