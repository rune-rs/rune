use core::fmt;

use crate as rune;
use crate::alloc::prelude::*;
use crate::ast::{Span, Spanned};
use crate::SourceId;

/// A fully descriptive location which is a combination of a [SourceId] and a
/// [Span].
#[derive(Default, TryClone, Clone, Copy)]
#[try_clone(copy)]
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

impl Spanned for Location {
    #[inline]
    fn span(&self) -> Span {
        self.span
    }
}

impl Located for Location {
    #[inline]
    fn location(&self) -> Location {
        *self
    }

    #[inline]
    fn as_spanned(&self) -> &dyn Spanned {
        self
    }
}

impl Spanned for dyn Located {
    #[inline]
    fn span(&self) -> Span {
        self.as_spanned().span()
    }
}

/// Trait for things that have a [Location].
pub trait Located {
    /// Get the assocaited location.
    fn location(&self) -> Location;

    /// Get located item as spanned.
    fn as_spanned(&self) -> &dyn Spanned;
}

pub(crate) struct DynLocation<S> {
    source_id: SourceId,
    span: S,
}

impl<S> DynLocation<S> {
    #[inline(always)]
    pub(crate) const fn new(source_id: SourceId, span: S) -> Self {
        Self { source_id, span }
    }
}

impl<S> Spanned for DynLocation<S>
where
    S: Spanned,
{
    #[inline]
    fn span(&self) -> Span {
        self.span.span()
    }
}

impl<S> Located for DynLocation<S>
where
    S: Spanned,
{
    #[inline]
    fn location(&self) -> Location {
        Location {
            source_id: self.source_id,
            span: self.span.span(),
        }
    }

    #[inline]
    fn as_spanned(&self) -> &dyn Spanned {
        self
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
