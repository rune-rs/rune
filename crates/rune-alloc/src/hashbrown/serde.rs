mod size_hint {
    use core::cmp;

    /// This presumably exists to prevent denial of service attacks.
    ///
    /// Original discussion: https://github.com/serde-rs/serde/issues/1114.
    #[cfg_attr(feature = "inline-more", inline)]
    pub(super) fn cautious(hint: Option<usize>) -> usize {
        cmp::min(hint.unwrap_or(0), 4096)
    }
}

mod map {
    use crate::alloc::Allocator;

    use core::fmt;
    use core::hash::{BuildHasher, Hash};
    use core::marker::PhantomData;

    use serde::de::{Deserialize, Deserializer, Error, MapAccess, Visitor};
    use serde::ser::{Serialize, Serializer};

    use crate::hash_map::HashMap;

    use super::size_hint;

    impl<K, V, H, A> Serialize for HashMap<K, V, H, A>
    where
        K: Serialize + Eq + Hash,
        V: Serialize,
        H: BuildHasher,
        A: Allocator,
    {
        #[cfg_attr(feature = "inline-more", inline)]
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.collect_map(self)
        }
    }

    impl<'de, K, V, S, A> Deserialize<'de> for HashMap<K, V, S, A>
    where
        K: Deserialize<'de> + Eq + Hash,
        V: Deserialize<'de>,
        S: BuildHasher + Default,
        A: Allocator + Default,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct MapVisitor<K, V, S, A>
            where
                A: Allocator,
            {
                marker: PhantomData<HashMap<K, V, S, A>>,
            }

            impl<'de, K, V, S, A> Visitor<'de> for MapVisitor<K, V, S, A>
            where
                K: Deserialize<'de> + Eq + Hash,
                V: Deserialize<'de>,
                S: BuildHasher + Default,
                A: Allocator + Default,
            {
                type Value = HashMap<K, V, S, A>;

                fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    formatter.write_str("a map")
                }

                #[cfg_attr(feature = "inline-more", inline)]
                fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
                where
                    M: MapAccess<'de>,
                {
                    let mut values = HashMap::try_with_capacity_and_hasher_in(
                        size_hint::cautious(map.size_hint()),
                        S::default(),
                        A::default(),
                    )
                    .map_err(M::Error::custom)?;

                    while let Some((key, value)) = map.next_entry()? {
                        values.try_insert(key, value).map_err(M::Error::custom)?;
                    }

                    Ok(values)
                }
            }

            let visitor = MapVisitor {
                marker: PhantomData,
            };
            deserializer.deserialize_map(visitor)
        }
    }
}

mod set {
    use crate::alloc::Allocator;

    use core::fmt;
    use core::hash::{BuildHasher, Hash};
    use core::marker::PhantomData;
    use serde::de::{Deserialize, Deserializer, Error, SeqAccess, Visitor};
    use serde::ser::{Serialize, Serializer};

    use crate::hash_set::HashSet;

    use super::size_hint;

    impl<T, H, A> Serialize for HashSet<T, H, A>
    where
        T: Serialize + Eq + Hash,
        H: BuildHasher,
        A: Allocator,
    {
        #[cfg_attr(feature = "inline-more", inline)]
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.collect_seq(self)
        }
    }

    impl<'de, T, S, A> Deserialize<'de> for HashSet<T, S, A>
    where
        T: Deserialize<'de> + Eq + Hash,
        S: BuildHasher + Default,
        A: Allocator + Default,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct SeqVisitor<T, S, A>
            where
                A: Allocator,
            {
                marker: PhantomData<HashSet<T, S, A>>,
            }

            impl<'de, T, S, A> Visitor<'de> for SeqVisitor<T, S, A>
            where
                T: Deserialize<'de> + Eq + Hash,
                S: BuildHasher + Default,
                A: Allocator + Default,
            {
                type Value = HashSet<T, S, A>;

                fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    formatter.write_str("a sequence")
                }

                #[cfg_attr(feature = "inline-more", inline)]
                fn visit_seq<M>(self, mut seq: M) -> Result<Self::Value, M::Error>
                where
                    M: SeqAccess<'de>,
                {
                    let mut values = HashSet::try_with_capacity_and_hasher_in(
                        size_hint::cautious(seq.size_hint()),
                        S::default(),
                        A::default(),
                    )
                    .map_err(M::Error::custom)?;

                    while let Some(value) = seq.next_element()? {
                        values.try_insert(value).map_err(M::Error::custom)?;
                    }

                    Ok(values)
                }
            }

            let visitor = SeqVisitor {
                marker: PhantomData,
            };
            deserializer.deserialize_seq(visitor)
        }

        #[allow(clippy::missing_errors_doc)]
        fn deserialize_in_place<D>(deserializer: D, place: &mut Self) -> Result<(), D::Error>
        where
            D: Deserializer<'de>,
        {
            struct SeqInPlaceVisitor<'a, T, S, A>(&'a mut HashSet<T, S, A>)
            where
                A: Allocator;

            impl<'a, 'de, T, S, A> Visitor<'de> for SeqInPlaceVisitor<'a, T, S, A>
            where
                T: Deserialize<'de> + Eq + Hash,
                S: BuildHasher + Default,
                A: Allocator,
            {
                type Value = ();

                fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    formatter.write_str("a sequence")
                }

                #[cfg_attr(feature = "inline-more", inline)]
                fn visit_seq<M>(self, mut seq: M) -> Result<Self::Value, M::Error>
                where
                    M: SeqAccess<'de>,
                {
                    self.0.clear();
                    self.0
                        .try_reserve(size_hint::cautious(seq.size_hint()))
                        .map_err(M::Error::custom)?;

                    while let Some(value) = seq.next_element()? {
                        self.0.try_insert(value).map_err(M::Error::custom)?;
                    }

                    Ok(())
                }
            }

            deserializer.deserialize_seq(SeqInPlaceVisitor(place))
        }
    }
}
