use serde::ser::{Serialize, Serializer};

use crate::borrow::{Cow, TryToOwned};
use crate::boxed::Box;
use crate::btree::set::BTreeSet;
use crate::vec::Vec;

macro_rules! deref_impl {
    (
        $(#[doc = $doc:tt])*
        <$($desc:tt)+
    ) => {
        $(#[doc = $doc])*
        impl <$($desc)+ {
            #[inline]
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                (**self).serialize(serializer)
            }
        }
    };
}

deref_impl!(<T: ?Sized> Serialize for Box<T> where T: Serialize);
deref_impl!(<T: ?Sized> Serialize for Cow<'_, T> where T: Serialize + TryToOwned);

macro_rules! seq_impl {
    ($ty:ident <T $(: $tbound1:ident $(+ $tbound2:ident)*)* $(, $typaram:ident : $bound:ident)*>) => {
        impl<T $(, $typaram)*> Serialize for $ty<T $(, $typaram)*>
        where
            T: Serialize,
        {
            #[inline]
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.collect_seq(self)
            }
        }
    }
}

seq_impl!(BTreeSet<T: Ord>);
seq_impl!(Vec<T>);
