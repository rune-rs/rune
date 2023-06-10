use core::slice;

use crate::no_std::boxed::Box;
use crate::no_std::vec::Vec;

use crate::ast::Span;
use crate::parse::{Id, NonZeroId};

/// Defer building a span from a function.
pub(crate) fn from_fn<F>(function: F) -> FromFn<F> {
    FromFn { function }
}

/// Function used to build a [`Span`].
#[derive(Clone, Copy)]
pub(crate) struct FromFn<F> {
    function: F,
}

impl<F> Spanned for FromFn<F>
where
    F: Fn() -> Span,
{
    #[inline]
    fn span(&self) -> Span {
        (self.function)()
    }
}

/// Helper derive to implement [`OptionSpanned`].
pub use rune_macros::OptionSpanned;

/// Helper derive to implement [`Spanned`].
pub use rune_macros::Spanned;

/// Types for which we can get a span.
pub trait Spanned {
    /// Get the span of the type.
    fn span(&self) -> Span;
}

impl<A, B> Spanned for (A, B)
where
    A: Spanned,
    B: Spanned,
{
    fn span(&self) -> Span {
        self.0.span().join(self.1.span())
    }
}

impl Spanned for Span {
    fn span(&self) -> Span {
        *self
    }
}

impl<T> Spanned for Box<T>
where
    T: Spanned,
{
    fn span(&self) -> Span {
        Spanned::span(&**self)
    }
}

impl<T> Spanned for &T
where
    T: ?Sized + Spanned,
{
    fn span(&self) -> Span {
        Spanned::span(*self)
    }
}

impl<T> Spanned for &mut T
where
    T: ?Sized + Spanned,
{
    fn span(&self) -> Span {
        Spanned::span(*self)
    }
}

impl<S> Spanned for (S, Id)
where
    S: Spanned,
{
    fn span(&self) -> Span {
        self.0.span()
    }
}

impl<S> Spanned for (S, Option<Id>)
where
    S: Spanned,
{
    fn span(&self) -> Span {
        self.0.span()
    }
}

impl<S> Spanned for (S, NonZeroId)
where
    S: Spanned,
{
    fn span(&self) -> Span {
        self.0.span()
    }
}

/// Types for which we can optionally get a span.
pub trait OptionSpanned {
    /// Get the optional span of the type.
    fn option_span(&self) -> Option<Span>;
}

impl<S> OptionSpanned for slice::Iter<'_, S>
where
    S: Spanned,
{
    fn option_span(&self) -> Option<Span> {
        OptionSpanned::option_span(self.as_slice())
    }
}

impl<T> OptionSpanned for Box<T>
where
    T: OptionSpanned,
{
    fn option_span(&self) -> Option<Span> {
        OptionSpanned::option_span(&**self)
    }
}

/// Take the span of a vector of spanned.
/// Provides the span between the first and the last element.
impl<T> OptionSpanned for Vec<T>
where
    T: Spanned,
{
    fn option_span(&self) -> Option<Span> {
        OptionSpanned::option_span(&**self)
    }
}

/// Take the span of a vector of spanned.
/// Provides the span between the first and the last element.
impl<T> OptionSpanned for [T]
where
    T: Spanned,
{
    fn option_span(&self) -> Option<Span> {
        let span = self.first()?.span();

        if let Some(last) = self.last() {
            Some(span.join(last.span()))
        } else {
            Some(span)
        }
    }
}

impl<T> OptionSpanned for Option<T>
where
    T: Spanned,
{
    fn option_span(&self) -> Option<Span> {
        Some(self.as_ref()?.span())
    }
}
