use core::fmt;

use crate::ast::Span;
use crate::SourceId;

/// A fully descriptive location which is a combination of a [SourceId] and a
/// [Span].
#[derive(Default, Clone, Copy)]
#[non_exhaustive]
pub struct Location {
    /// The source id of the file of the location.
    pub source_id: SourceId,
    /// The span of the location.
    pub span: Span,
}

impl Location {
    /// Construct a new location.
    pub const fn new(source_id: SourceId, span: Span) -> Self {
        Self { source_id, span }
    }
}

impl fmt::Debug for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Location")
            .field(&self.source_id)
            .field(&self.span)
            .finish()
    }
}
