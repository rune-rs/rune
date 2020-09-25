use crate::Spanned;
use runestick::Span;

/// An internal compiler error.
pub(crate) struct Internal {
    span: Span,
    message: &'static str,
}

impl Internal {
    /// Construct a new internal error.
    pub(crate) fn new<S>(spanned: S, message: &'static str) -> Self
    where
        S: Spanned,
    {
        Self {
            span: spanned.span(),
            message,
        }
    }

    /// Message of the internal error.
    pub(crate) fn message(&self) -> &'static str {
        self.message
    }
}

impl Spanned for Internal {
    fn span(&self) -> Span {
        self.span
    }
}
