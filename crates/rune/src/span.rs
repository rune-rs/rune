use serde::{Deserialize, Serialize};
use std::cmp;
use std::fmt;
use std::ops;

/// A span corresponding to a range in the source file being parsed.
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Span {
    /// The start of the span in bytes.
    pub start: ByteIndex,
    /// The end of the span in bytes.
    pub end: ByteIndex,
}

impl Span {
    /// Construct a new span.
    pub fn new(start: impl IntoByteIndex, end: impl IntoByteIndex) -> Self {
        Self {
            start: start.into_byte_index(),
            end: end.into_byte_index(),
        }
    }

    /// Constant function to build a span.
    pub const fn const_new(start: u32, end: u32) -> Self {
        Self {
            start: ByteIndex(start),
            end: ByteIndex(end),
        }
    }

    /// Constant function to build an empty span.
    pub const fn empty() -> Self {
        Self {
            start: ByteIndex(0),
            end: ByteIndex(0),
        }
    }

    /// Get the span as an usize range.
    pub fn range(self) -> ops::Range<usize> {
        ops::Range {
            start: self.start.into_usize(),
            end: self.end.into_usize(),
        }
    }

    /// Adjust the span with the given positive offset.
    pub fn adjust(self, diff: ByteIndex) -> Self {
        Self {
            start: self.start.saturating_add(diff),
            end: self.end.saturating_add(diff),
        }
    }

    /// Check if current span completely overlaps with another.
    pub fn overlaps(self, other: Span) -> bool {
        self.start <= other.start && self.end >= other.end
    }

    /// Get the length of the span.
    pub fn len(self) -> ByteIndex {
        self.end.saturating_sub(self.start)
    }

    /// Check if the span is empty.
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Join this span with another span.
    pub fn join(self, other: Self) -> Self {
        Self {
            start: ByteIndex::min(self.start, other.start),
            end: ByteIndex::max(self.end, other.end),
        }
    }

    /// Get the point span.
    pub fn point(pos: impl IntoByteIndex) -> Self {
        let pos = pos.into_byte_index();

        Self {
            start: pos,
            end: pos,
        }
    }

    /// Narrow the span with the given amount.
    pub fn narrow(self, amount: impl IntoByteIndex) -> Self {
        let amount = amount.into_byte_index();

        Self {
            start: self.start.saturating_add(amount),
            end: self.end.saturating_sub(amount),
        }
    }

    /// Trim the start of the span by the given amount.
    pub fn trim_start(self, amount: impl IntoByteIndex) -> Self {
        let amount = amount.into_byte_index();

        Self {
            start: ByteIndex::min(self.start.saturating_add(amount), self.end),
            end: self.end,
        }
    }

    /// Trim the end of the span by the given amount.
    pub fn trim_end(self, amount: impl IntoByteIndex) -> Self {
        let amount = amount.into_byte_index();

        Self {
            start: self.start,
            end: ByteIndex::max(self.end.saturating_sub(amount), self.start),
        }
    }

    /// Get the start as a point span.
    pub fn start(self) -> Self {
        Self {
            start: self.start,
            end: self.start,
        }
    }

    /// Get the end as a point span.
    pub fn end(self) -> Self {
        Self {
            start: self.end,
            end: self.end,
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}:{}", self.start, self.end)
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Span")
            .field(&self.start)
            .field(&self.end)
            .finish()
    }
}

/// A single index in a [Span], like the start or ending index.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ByteIndex(#[doc(hidden)] pub u32);

impl ByteIndex {
    /// Convert into usize.
    pub fn into_usize(self) -> usize {
        usize::try_from(self.0).expect("index does not fit within usize")
    }

    fn min(a: Self, b: Self) -> Self {
        Self(u32::min(a.0, b.0))
    }

    fn max(a: Self, b: Self) -> Self {
        Self(u32::max(a.0, b.0))
    }

    fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }
}

impl fmt::Display for ByteIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for ByteIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Convert the given type into an index.
///
/// # Panics
///
/// This trait will cause a panic during conversion if the type being converted
/// doesn't fit within the ByteIndex type. However, ByteIndex is big enough (at
/// least `2**32` bytes) to fit reasonable source files.
pub trait IntoByteIndex {
    /// Convert into index. Panics if the type does not fit within the index.
    fn into_byte_index(self) -> ByteIndex;
}

impl IntoByteIndex for ByteIndex {
    fn into_byte_index(self) -> ByteIndex {
        self
    }
}

impl IntoByteIndex for usize {
    fn into_byte_index(self) -> ByteIndex {
        ByteIndex(u32::try_from(self).expect("value does not fit within index"))
    }
}

impl cmp::PartialEq<usize> for ByteIndex {
    fn eq(&self, other: &usize) -> bool {
        match u32::try_from(*other) {
            Ok(other) => self.0 == other,
            Err(..) => false,
        }
    }
}

impl cmp::PartialEq<u32> for ByteIndex {
    fn eq(&self, other: &u32) -> bool {
        self.0 == *other
    }
}

impl From<Span> for ops::Range<usize> {
    fn from(span: Span) -> Self {
        span.range()
    }
}
