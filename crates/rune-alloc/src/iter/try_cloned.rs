use crate::clone::TryClone;
use crate::error::Error;

/// An iterator that clones the elements of an underlying iterator.
///
/// This `struct` is created by the [`try_cloned`] method on [`IteratorExt`].
/// See its documentation for more.
///
/// [`try_cloned`]: crate::iter::IteratorExt::try_cloned
/// [`IteratorExt`]: crate::iter::IteratorExt
pub struct TryCloned<I> {
    it: I,
}

impl<I> TryCloned<I> {
    pub(in crate::iter) fn new(it: I) -> Self {
        Self { it }
    }
}

impl<'a, I, T: 'a> Iterator for TryCloned<I>
where
    I: Iterator<Item = &'a T>,
    T: TryClone,
{
    type Item = Result<T, Error>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        Some(self.it.next()?.try_clone())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.it.size_hint()
    }
}
