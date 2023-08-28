use crate::alloc::{Allocator, Error, Global, TryFromIteratorIn};

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
}

impl<I> IteratorExt for I where I: Iterator {}

mod sealed {
    pub trait Sealed {}
    impl<I> Sealed for I where I: Iterator {}
}
