use core::cmp::Ordering;
use core::fmt;
use core::marker::PhantomData;

use crate as rune;
use crate::alloc::{Box, String};
use crate::module::InstallWith;
use crate::{item, Item};

/// The trait used for something that can be statically named.
pub trait Named {
    /// The name item.
    const ITEM: &'static Item;

    /// The exact type name
    fn full_name(f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Self::ITEM)
    }

    /// Return a display wrapper for the named type.
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

impl Named for String {
    const ITEM: &'static Item = item!(::std::string::String);
}

impl Named for &str {
    const ITEM: &'static Item = item!(::std::string::String);
}

impl Named for Box<str> {
    const ITEM: &'static Item = item!(::std::string::String);
}

impl InstallWith for String {}

impl Named for i64 {
    const ITEM: &'static Item = item!(::std::i64);
}

impl InstallWith for i64 {}

impl Named for u64 {
    const ITEM: &'static Item = item!(::std::u64);
}

impl InstallWith for u64 {}

impl Named for f64 {
    const ITEM: &'static Item = item!(::std::f64);
}

impl InstallWith for f64 {}

impl Named for char {
    const ITEM: &'static Item = item!(::std::char);
}

impl InstallWith for char {}

impl Named for bool {
    const ITEM: &'static Item = item!(::std::bool);
}

impl InstallWith for bool {}

impl Named for Ordering {
    const ITEM: &'static Item = item!(::std::cmp::Ordering);
}

impl InstallWith for Ordering {}
