use crate::alloc;
use crate::ast::Span;
use crate::parse::{Id, NonZeroId};

/// Helper derive to implement [`Spanned`].
pub use rune_macros::Spanned;

/// Helper derive to implement [`OptionSpanned`].
pub use rune_macros::OptionSpanned;

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

/// Types for which we can get a span.
pub trait Spanned {
    /// Get the span of the type.
    fn span(&self) -> Span;
}

impl<A, B> Spanned for (A, B)
where
    A: Spanned,
    B: OptionSpanned,
{
    fn span(&self) -> Span {
        if let Some(end) = self.1.option_span() {
            self.0.span().join(end)
        } else {
            self.0.span()
        }
    }
}

impl Spanned for Span {
    fn span(&self) -> Span {
        *self
    }
}

#[cfg(feature = "alloc")]
impl<T> Spanned for ::rust_alloc::boxed::Box<T>
where
    T: Spanned,
{
    fn span(&self) -> Span {
        Spanned::span(&**self)
    }
}

impl<T> Spanned for alloc::Box<T>
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

#[cfg(feature = "alloc")]
impl<T> OptionSpanned for ::rust_alloc::boxed::Box<T>
where
    T: OptionSpanned,
{
    fn option_span(&self) -> Option<Span> {
        OptionSpanned::option_span(&**self)
    }
}

impl<T> OptionSpanned for alloc::Box<T>
where
    T: OptionSpanned,
{
    fn option_span(&self) -> Option<Span> {
        OptionSpanned::option_span(&**self)
    }
}
