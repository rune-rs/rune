use core::cmp::Ordering;
use core::fmt;
use core::marker::PhantomData;

use crate as rune;
use crate::{item, Hash, Item};

/// The trait used for something that can be statically named.
pub trait Named {
    /// The name item.
    const ITEM: &'static Item;

    /// The full name of the type.
    #[inline]
    fn full_name(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Self::ITEM)
    }

    /// Return a display wrapper for the full name of the type.
    #[inline]
    fn display() -> impl fmt::Display {
        struct DisplayNamed<T>(PhantomData<T>)
        where
            T: ?Sized;

        impl<T> DisplayNamed<T>
        where
            T: ?Sized,
        {
            fn new() -> Self {
                Self(PhantomData)
            }
        }

        impl<T> fmt::Display for DisplayNamed<T>
        where
            T: ?Sized + Named,
        {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                T::full_name(f)
            }
        }

        DisplayNamed::<Self>::new()
    }
}

impl Named for i64 {
    const ITEM: &'static Item = item!(::std::i64);
}

impl Named for u64 {
    const ITEM: &'static Item = item!(::std::u64);
}

impl Named for f64 {
    const ITEM: &'static Item = item!(::std::f64);
}

impl Named for char {
    const ITEM: &'static Item = item!(::std::char);
}

impl Named for bool {
    const ITEM: &'static Item = item!(::std::bool);
}

impl Named for Ordering {
    const ITEM: &'static Item = item!(::std::cmp::Ordering);
}

impl Named for Hash {
    const ITEM: &'static Item = item!(::std::any::Hash);
}
