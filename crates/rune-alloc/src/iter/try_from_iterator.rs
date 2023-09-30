use crate::alloc::{Allocator, Global};
use crate::error::Error;

/// Conversion from an [`Iterator`] within a custom allocator `A`.
///
/// By implementing `TryFromIteratorIn` for a type, you define how it will be
/// created from an iterator. This is common for types which describe a
/// collection of some kind.
pub trait TryFromIteratorIn<T, A: Allocator>: Sized {
    /// Creates a value from an iterator within an allocator.
    fn try_from_iter_in<I>(iter: I, alloc: A) -> Result<Self, Error>
    where
        I: IntoIterator<Item = T>;
}

/// Conversion from an [`Iterator`] within the [`Global`] allocator.
///
/// By implementing `TryFromIteratorIn` for a type, you define how it will be created
/// from an iterator. This is common for types which describe a collection of
/// some kind.
pub trait TryFromIterator<T>: TryFromIteratorIn<T, Global> {
    /// Creates a value from an iterator within an allocator.
    fn try_from_iter<I>(iter: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = T>;
}

impl<T, U> TryFromIterator<T> for U
where
    U: TryFromIteratorIn<T, Global>,
{
    #[inline]
    fn try_from_iter<I>(iter: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = T>,
    {
        U::try_from_iter_in(iter, Global)
    }
}

impl<T, U, E, A: Allocator> TryFromIteratorIn<Result<T, E>, A> for Result<U, E>
where
    U: TryFromIteratorIn<T, A>,
{
    fn try_from_iter_in<I>(iter: I, alloc: A) -> Result<Self, Error>
    where
        I: IntoIterator<Item = Result<T, E>>,
    {
        struct Iter<'a, I, E> {
            error: &'a mut Option<E>,
            iter: I,
        }

        impl<T, I, E> Iterator for Iter<'_, I, E>
        where
            I: Iterator<Item = Result<T, E>>,
        {
            type Item = T;

            fn next(&mut self) -> Option<Self::Item> {
                let value = match self.iter.next()? {
                    Ok(value) => value,
                    Err(error) => {
                        *self.error = Some(error);
                        return None;
                    }
                };

                Some(value)
            }
        }

        let mut error = None;

        let iter = Iter {
            error: &mut error,
            iter: iter.into_iter(),
        };

        let out = U::try_from_iter_in(iter, alloc)?;

        match error {
            Some(error) => Ok(Err(error)),
            None => Ok(Ok(out)),
        }
    }
}
