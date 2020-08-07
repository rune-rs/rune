use crate::error::{ParseError, Result};
use runestick::unit::Span;

/// A parsed input coupled with it's source.
#[derive(Debug, Clone, Copy)]
pub struct Source<'a> {
    pub(crate) source: &'a str,
}

impl<'a> Source<'a> {
    /// Fetch source for the given span.
    pub fn source(&self, span: Span) -> Result<&'a str, ParseError> {
        self.source
            .get(span.start..span.end)
            .ok_or_else(|| ParseError::BadSlice { span })
    }

    /// Get the end of the source.
    pub fn end(&self) -> usize {
        self.source.len()
    }

    /// Access the underlying string for the source.
    pub fn as_str(&self) -> &str {
        &self.source
    }
}
