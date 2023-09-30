use core::cmp;
use core::fmt;
use core::ops;

use serde::{Deserialize, Serialize};

use crate::ast::prelude::*;

/// A span corresponding to a range in the source file being parsed.
#[derive(Default, TryClone, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[try_clone(copy)]
pub struct Span {
    /// The start of the span in bytes.
    pub start: ByteIndex,
    /// The end of the span in bytes.
    pub end: ByteIndex,
}

impl Span {
    /// Construct a new span.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::ast::Span;
    ///
    /// let span = Span::new(42, 50);
    /// assert!(span < Span::new(100, 101));
    /// ```
    pub fn new<S, E>(start: S, end: E) -> Self
    where
        S: TryInto<ByteIndex>,
        S::Error: fmt::Debug,
        E: TryInto<ByteIndex>,
        E::Error: fmt::Debug,
    {
        Self {
            start: start.try_into().expect("start out of bounds"),
            end: end.try_into().expect("end out of bounds"),
        }
    }

    /// Get a span corresponding to a single point where both start and end are
    /// the same byte offset.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::ast::Span;
    ///
    /// assert_eq!(Span::point(42), Span::new(42, 42));
    /// ```
    pub fn point<P>(pos: P) -> Self
    where
        P: TryInto<ByteIndex>,
        P::Error: fmt::Debug,
    {
        let pos = pos.try_into().expect("point out of bounds");

        Self {
            start: pos,
            end: pos,
        }
    }

    /// Constant function to build an empty span.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::ast::Span;
    ///
    /// assert_eq!(Span::empty(), Span::new(0, 0));
    /// ```
    pub const fn empty() -> Self {
        Self {
            start: ByteIndex(0),
            end: ByteIndex(0),
        }
    }

    /// Get the head of the span.
    pub fn head(self) -> Self {
        Self {
            start: self.start,
            end: self.start,
        }
    }

    /// Get the tail of the span.
    pub fn tail(self) -> Self {
        Self {
            start: self.end,
            end: self.end,
        }
    }

    /// Join two spans creating the larger of the two spans.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::ast::Span;
    ///
    /// let a = Span::new(10, 12);
    /// let b = Span::new(20, 22);
    ///
    /// assert_eq!(a.join(b), Span::new(10, 22));
    /// ```
    pub fn join(self, other: Self) -> Self {
        Self {
            start: ByteIndex::min(self.start, other.start),
            end: ByteIndex::max(self.end, other.end),
        }
    }

    /// Narrow the span with the given amount.
    ///
    /// If the narrowing causes the span to become empty, the resulting span
    /// will reflect the starting point of the narrowed span.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::ast::Span;
    ///
    /// assert_eq!(Span::new(10, 12).narrow(4), Span::new(10, 10));
    /// assert_eq!(Span::new(5, 15).narrow(2), Span::new(7, 13));
    /// ```
    pub fn narrow(self, amount: impl Into<ByteIndex>) -> Self {
        let amount = amount.into();
        let end = ByteIndex::max(self.start, self.end.saturating_sub(amount));
        let start = ByteIndex::min(self.start.saturating_add(amount), end);
        Self { start, end }
    }

    /// Get the span as a range of usize.
    ///
    /// # Panics
    ///
    /// Panics if the span contains ranges which cannot be faithfully
    /// represented in an [usize].
    pub fn range(self) -> ops::Range<usize> {
        ops::Range {
            start: usize::try_from(self.start.0).expect("start index out of bounds"),
            end: usize::try_from(self.end.0).expect("end index out of bounds"),
        }
    }

    /// Trim the start of the span by the given amount.
    pub(crate) fn trim_start(self, amount: impl Into<ByteIndex>) -> Self {
        let amount = amount.into();

        Self {
            start: ByteIndex::min(self.start.saturating_add(amount), self.end),
            end: self.end,
        }
    }

    /// Trim the end of the span by the given amount.
    pub(crate) fn trim_end(self, amount: impl Into<ByteIndex>) -> Self {
        let amount = amount.into();

        Self {
            start: self.start,
            end: ByteIndex::max(self.end.saturating_sub(amount), self.start),
        }
    }
}

impl Serialize for Span {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (self.start, self.end).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Span {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (start, end) = <(ByteIndex, ByteIndex)>::deserialize(deserializer)?;
        Ok(Self { start, end })
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
#[derive(
    Default, TryClone, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[repr(transparent)]
#[serde(transparent)]
#[try_clone(copy)]
pub struct ByteIndex(#[doc(hidden)] pub u32);

impl ByteIndex {
    /// Convert a byte index into a usize.
    ///
    /// # Panics
    ///
    /// Panics if the byte index contains values which cannot be faithfully
    /// represented in an [usize].
    pub fn into_usize(self) -> usize {
        usize::try_from(self.0).expect("byte index out of range")
    }

    fn min(a: Self, b: Self) -> Self {
        Self(u32::min(a.0, b.0))
    }

    fn max(a: Self, b: Self) -> Self {
        Self(u32::max(a.0, b.0))
    }

    pub(crate) fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }

    fn saturating_add(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }
}

impl From<u32> for ByteIndex {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl TryFrom<usize> for ByteIndex {
    type Error = <usize as TryFrom<u32>>::Error;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(Self(u32::try_from(value)?))
    }
}

impl TryFrom<i32> for ByteIndex {
    type Error = <i32 as TryFrom<u32>>::Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        Ok(Self(u32::try_from(value)?))
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
