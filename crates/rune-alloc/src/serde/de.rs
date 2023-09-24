use core::fmt;
use core::marker::PhantomData;

use serde::de::{Deserialize, Deserializer, Error, SeqAccess, Visitor};

use crate::boxed::Box;
use crate::vec::Vec;

mod size_hint {
    use core::cmp;
    use core::mem;

    pub fn cautious<Element>(hint: Option<usize>) -> usize {
        const MAX_PREALLOC_BYTES: usize = 1024 * 1024;

        if mem::size_of::<Element>() == 0 {
            0
        } else {
            cmp::min(
                hint.unwrap_or(0),
                MAX_PREALLOC_BYTES / mem::size_of::<Element>(),
            )
        }
    }
}

mod seed {
    use serde::de::{Deserialize, DeserializeSeed, Deserializer};

    /// A DeserializeSeed helper for implementing deserialize_in_place Visitors.
    ///
    /// Wraps a mutable reference and calls deserialize_in_place on it.
    pub struct InPlaceSeed<'a, T: 'a>(pub &'a mut T);

    impl<'a, 'de, T> DeserializeSeed<'de> for InPlaceSeed<'a, T>
    where
        T: Deserialize<'de>,
    {
        type Value = ();
        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            T::deserialize_in_place(deserializer, self.0)
        }
    }
}

macro_rules! forwarded_impl {
    (
        $(#[doc = $doc:tt])*
        <$($id:ident),*>, $ty:ty, $func:expr
    ) => {
        $(#[doc = $doc])*
        impl<'de $(, $id : Deserialize<'de>,)*> Deserialize<'de> for $ty {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = Deserialize::deserialize(deserializer)?;
                $func(value).map_err(D::Error::custom)
            }
        }
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
forwarded_impl!(<T>, Box<T>, Box::try_new);

impl<'de, T> Deserialize<'de> for Vec<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VecVisitor<T> {
            marker: PhantomData<T>,
        }

        impl<'de, T> Visitor<'de> for VecVisitor<T>
        where
            T: Deserialize<'de>,
        {
            type Value = Vec<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a sequence")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let capacity = size_hint::cautious::<T>(seq.size_hint());
                let mut values = Vec::<T>::try_with_capacity(capacity).map_err(A::Error::custom)?;

                while let Some(value) = seq.next_element()? {
                    values.try_push(value).map_err(A::Error::custom)?;
                }

                Ok(values)
            }
        }

        let visitor = VecVisitor {
            marker: PhantomData,
        };

        deserializer.deserialize_seq(visitor)
    }

    fn deserialize_in_place<D>(deserializer: D, place: &mut Self) -> Result<(), D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VecInPlaceVisitor<'a, T: 'a>(&'a mut Vec<T>);

        impl<'a, 'de, T> Visitor<'de> for VecInPlaceVisitor<'a, T>
        where
            T: Deserialize<'de>,
        {
            type Value = ();

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a sequence")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let hint = size_hint::cautious::<T>(seq.size_hint());

                if let Some(additional) = hint.checked_sub(self.0.len()) {
                    self.0.try_reserve(additional).map_err(A::Error::custom)?;
                }

                for i in 0..self.0.len() {
                    let next = {
                        let next_place = seed::InPlaceSeed(&mut self.0[i]);
                        seq.next_element_seed(next_place)?
                    };

                    if next.is_none() {
                        self.0.truncate(i);
                        return Ok(());
                    }
                }

                while let Some(value) = seq.next_element()? {
                    self.0.try_push(value).map_err(A::Error::custom)?;
                }

                Ok(())
            }
        }

        deserializer.deserialize_seq(VecInPlaceVisitor(place))
    }
}

impl<'de, T> Deserialize<'de> for Box<[T]>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<T>::deserialize(deserializer)?
            .try_into_boxed_slice()
            .map_err(D::Error::custom)
    }
}
