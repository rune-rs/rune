use crate::error::{ResolveError, Result};
use st::unit::Span;

/// A parsed input coupled with it's source.
#[derive(Debug, Clone, Copy)]
pub struct Source<'a> {
    pub(crate) source: &'a str,
}

impl<'a> Source<'a> {
    /// Fetch source for the given span.
    pub fn source(&self, span: Span) -> Result<&'a str, ResolveError> {
        Ok(&self.source[span.start..span.end])
    }
}
