use std::fmt;

/// A span corresponding to a range in the source file being parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Span {
    /// The start of the span in bytes.
    pub start: usize,
    /// The end of the span in bytes.
    pub end: usize,
}

impl Span {
    /// Construct a new span.
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Return a span with a modified start position.
    pub fn with_start(self, start: usize) -> Self {
        Self {
            start,
            end: self.end,
        }
    }

    /// Return a span with a modified end position.
    pub fn with_end(self, end: usize) -> Self {
        Self {
            start: self.start,
            end,
        }
    }

    /// Check if current span completely overlaps with another.
    pub fn overlaps(self, other: Span) -> bool {
        self.start <= other.start && self.end >= other.end
    }

    /// An empty span.
    pub const fn empty() -> Self {
        Self { start: 0, end: 0 }
    }

    /// Get the length of the span.
    pub fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Check if the span is empty.
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Join this span with another span.
    pub fn join(self, other: Self) -> Self {
        Self {
            start: usize::min(self.start, other.start),
            end: usize::max(self.end, other.end),
        }
    }

    /// Get the point span.
    pub fn point(pos: usize) -> Self {
        Self {
            start: pos,
            end: pos,
        }
    }

    /// Narrow the span with the given amount.
    pub fn narrow(self, amount: usize) -> Self {
        Self {
            start: self.start.saturating_add(amount),
            end: self.end.saturating_sub(amount),
        }
    }

    /// Trim the start of the span by the given amount.
    pub fn trim_start(self, amount: usize) -> Self {
        Self {
            start: usize::min(self.start.saturating_add(amount), self.end),
            end: self.end,
        }
    }

    /// Trim the end of the span by the given amount.
    pub fn trim_end(self, amount: usize) -> Self {
        Self {
            start: self.start,
            end: usize::max(self.end.saturating_sub(amount), self.start),
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}:{}", self.start, self.end)
    }
}
