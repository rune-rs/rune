use crate::alloc::{Allocator, Global};
use crate::clone::TryClone;
use crate::error::Error;
use crate::iter::{TryCloned, TryFromIteratorIn, TryJoin};

/// Iterator extension trait.
pub trait IteratorExt: Iterator + self::sealed::Sealed {
    /// Transforms an iterator into a collection using fallible allocations.
    fn try_collect<B>(self) -> Result<B, Error>
    where
        Self: Sized,
        B: TryFromIteratorIn<Self::Item, Global>,
    {
        self.try_collect_in(Global)
    }

    /// Transforms an iterator into a collection using fallible allocations.
    fn try_collect_in<B, A: Allocator>(self, alloc: A) -> Result<B, Error>
    where
        Self: Sized,
        B: TryFromIteratorIn<Self::Item, A>,
    {
        TryFromIteratorIn::try_from_iter_in(self, alloc)
    }

    /// Try to join the given value.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::alloc::String;
    /// use rune::alloc::prelude::*;
    ///
    /// let values = ["foo", "bar"];
    /// let string: String = values.into_iter().try_join("/")?;
    /// assert_eq!(string, "foo/bar");
    ///
    /// let values = ["foo", "bar"];
    /// let string: String = values.into_iter().try_join('/')?;
    /// assert_eq!(string, "foo/bar");
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn try_join<J, S>(self, sep: S) -> Result<J, Error>
    where
        Self: Sized,
        J: TryJoin<S, Self::Item, Global>,
    {
        J::try_join_in(self, sep, Global)
    }

    /// Try to join the given value.
    fn try_join_in<J, S, A: Allocator>(self, sep: S, alloc: A) -> Result<J, Error>
    where
        Self: Sized,
        J: TryJoin<S, Self::Item, A>,
    {
        J::try_join_in(self, sep, alloc)
    }

    /// Creates an iterator which [`try_clone`]s all of its elements.
    ///
    /// This is useful when you have an iterator over `&T`, but you need an
    /// iterator over `T`.
    ///
    /// There is no guarantee whatsoever about the `try_clone` method actually
    /// being called *or* optimized away. So code should not depend on either.
    ///
    /// [`try_clone`]: TryClone::try_clone
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use rune::alloc::{try_vec, Vec};
    /// use rune::alloc::prelude::*;
    ///
    /// let a = [1, 2, 3];
    ///
    /// let v_cloned: Vec<_> = a.iter().try_cloned().try_collect::<Result<_, _>>()??;
    ///
    /// // cloned is the same as .map(|&x| x), for integers
    /// let v_map: Vec<_> = a.iter().map(|&x| x).try_collect()?;
    ///
    /// assert_eq!(v_cloned, [1, 2, 3]);
    /// assert_eq!(v_map, [1, 2, 3]);
    /// # Ok::<_, rune::alloc::Error>(())
    /// ```
    fn try_cloned<'a, T: 'a>(self) -> TryCloned<Self>
    where
        Self: Sized + Iterator<Item = &'a T>,
        T: TryClone,
    {
        TryCloned::new(self)
    }
}

impl<I> IteratorExt for I where I: Iterator {}

mod sealed {
    pub trait Sealed {}
    impl<I> Sealed for I where I: Iterator {}
}
