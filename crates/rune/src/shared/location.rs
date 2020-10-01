use runestick::{SourceId, Span};

/// A source location.
#[derive(Debug, Default, Clone, Copy)]
pub struct Location {
    /// The source id of the file of the location.
    pub source_id: SourceId,
    /// The span of the location.
    pub span: Span,
}

impl Location {
    /// Construct a new location.
    pub fn new(source_id: SourceId, span: Span) -> Self {
        Self { source_id, span }
    }
}
