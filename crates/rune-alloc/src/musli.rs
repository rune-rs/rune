use core::fmt;
use core::hash::{BuildHasher, Hash};

use crate::borrow::Cow;
use crate::borrow::TryToOwned;
use crate::{BTreeMap, BTreeSet, Box, HashMap, HashSet, String, Vec, VecDeque};

use musli::alloc::ToOwned;
use musli::de::SizeHint;
use musli::de::{
    Decode, DecodeBytes, DecodeSliceBuilder, DecodeTrace, Decoder, EntryDecoder, MapDecoder,
    SequenceDecoder, UnsizedVisitor,
};
use musli::en::{
    Encode, EncodeBytes, EncodePacked, EncodeTrace, Encoder, EntryEncoder, MapEncoder,
    SequenceEncoder,
};
use musli::{Allocator, Context};

// Uses the same heuristic as:
// https://github.com/serde-rs/serde/blob/d91f8ba950e2faf4db4e283e917ba2ee94a9b8a4/serde/src/de/size_hint.rs#L12
#[inline]
pub(crate) fn cautious<T>(hint: impl Into<SizeHint>) -> usize {
    const MAX_PREALLOC_BYTES: usize = 1024 * 1024;

    if size_of::<T>() == 0 {
        return 0;
    }

    hint.into()
        .or_default()
        .min(MAX_PREALLOC_BYTES / size_of::<T>())
}

impl<M> Encode<M> for String {
    type Encode = str;

    const IS_BITWISE_ENCODE: bool = false;

    #[inline]
    fn encode<E>(&self, encoder: E) -> Result<(), E::Error>
    where
        E: Encoder<Mode = M>,
    {
        self.as_str().encode(encoder)
    }

    #[inline]
    fn as_encode(&self) -> &Self::Encode {
        self
    }
}

impl<'de, M, A> Decode<'de, M, A> for String
where
    A: Allocator,
{
    const IS_BITWISE_DECODE: bool = false;

    #[inline]
    fn decode<D>(decoder: D) -> Result<Self, D::Error>
    where
        D: Decoder<'de, Mode = M>,
    {
        struct Visitor;

        #[musli::de::unsized_visitor]
        impl<C> UnsizedVisitor<'_, C, str> for Visitor
        where
            C: Context,
        {
            type Ok = String;

            #[inline]
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "string")
            }

            #[inline]
            fn visit_ref(self, cx: C, string: &str) -> Result<Self::Ok, Self::Error> {
                string.try_to_owned().map_err(|e| cx.custom(e))
            }
        }

        decoder.decode_string(Visitor)
    }
}

impl<'de, M, A> Decode<'de, M, A> for Box<str>
where
    A: Allocator,
{
    const IS_BITWISE_DECODE: bool = false;

    #[inline]
    fn decode<D>(decoder: D) -> Result<Self, D::Error>
    where
        D: Decoder<'de, Mode = M>,
    {
        let cx = decoder.cx();
        decoder
            .decode::<String>()?
            .try_into()
            .map_err(|e| cx.custom(e))
    }
}

impl<'de, M, A, T> Decode<'de, M, A> for Box<[T]>
where
    A: Allocator,
    T: Decode<'de, M, A>,
{
    const IS_BITWISE_DECODE: bool = false;

    #[inline]
    fn decode<D>(decoder: D) -> Result<Self, D::Error>
    where
        D: Decoder<'de, Mode = M, Allocator = A>,
    {
        let cx = decoder.cx();
        decoder
            .decode::<Vec<T>>()?
            .try_into()
            .map_err(|e| cx.custom(e))
    }
}

macro_rules! cow {
    (
        $encode:ident :: $encode_fn:ident,
        $as_encode:ident,
        $decode:ident :: $decode_fn:ident,
        $encode_packed:ident,
        $decode_packed:ident,
        $ty:ty, $source:ty,
        $decode_method:ident, $cx:pat,
        |$owned:ident| $owned_expr:expr,
        |$borrowed:ident| $borrowed_expr:expr,
        |$reference:ident| $reference_expr:expr $(,)?
    ) => {
        impl<M> $encode<M> for Cow<'_, $ty> {
            const $encode_packed: bool = false;

            type $encode = $ty;

            #[inline]
            fn $encode_fn<E>(&self, encoder: E) -> Result<(), E::Error>
            where
                E: Encoder<Mode = M>,
            {
                self.as_ref().$encode_fn(encoder)
            }

            #[inline]
            fn $as_encode(&self) -> &Self::$encode {
                self
            }
        }

        impl<'de, M, A> $decode<'de, M, A> for Cow<'_, $ty>
        where
            A: Allocator,
        {
            const $decode_packed: bool = false;

            #[inline]
            fn $decode_fn<D>(decoder: D) -> Result<Self, D::Error>
            where
                D: Decoder<'de, Mode = M, Allocator = A>,
            {
                struct Visitor;

                #[musli::de::unsized_visitor]
                impl<'de, C> UnsizedVisitor<'de, C, $source> for Visitor
                where
                    C: Context,
                {
                    type Ok = Cow<'static, $ty>;

                    #[inline]
                    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        write!(f, "a string")
                    }

                    #[inline]
                    fn visit_owned(
                        self,
                        $cx: C,
                        $owned: <$source as ToOwned>::Owned<Self::Allocator>,
                    ) -> Result<Self::Ok, Self::Error> {
                        Ok($owned_expr)
                    }

                    #[inline]
                    fn visit_borrowed(
                        self,
                        $cx: C,
                        $borrowed: &'de $source,
                    ) -> Result<Self::Ok, Self::Error> {
                        Ok($borrowed_expr)
                    }

                    #[inline]
                    fn visit_ref(
                        self,
                        $cx: C,
                        $reference: &$source,
                    ) -> Result<Self::Ok, Self::Error> {
                        Ok($reference_expr)
                    }
                }

                decoder.$decode_method(Visitor)
            }
        }
    };
}

cow! {
    Encode::encode,
    as_encode,
    Decode::decode,
    IS_BITWISE_ENCODE,
    IS_BITWISE_DECODE,
    str, str, decode_string, cx,
    |owned| {
        match owned.into_std() {
            Ok(owned) => Cow::Owned(owned.try_into().map_err(|e| cx.custom(e))?),
            Err(owned) => {
                Cow::Owned(TryToOwned::try_to_owned(owned.as_str()).map_err(|e| cx.custom(e))?)
            }
        }
    },
    |borrowed| Cow::Owned(TryToOwned::try_to_owned(borrowed).map_err(|e| cx.custom(e))?),
    |reference| Cow::Owned(TryToOwned::try_to_owned(reference).map_err(|e| cx.custom(e))?),
}

cow! {
    EncodeBytes::encode_bytes,
    as_encode_bytes,
    DecodeBytes::decode_bytes,
    ENCODE_BYTES_PACKED,
    DECODE_BYTES_PACKED,
    [u8], [u8], decode_bytes, cx,
    |owned| {
        match owned.into_std() {
            Ok(owned) => Cow::Owned(owned.try_into().map_err(|e| cx.custom(e))?),
            Err(owned) => Cow::Owned(TryToOwned::try_to_owned(owned.as_slice()).map_err(|e| cx.custom(e))?),
        }
    },
    |borrowed| Cow::Owned(TryToOwned::try_to_owned(borrowed).map_err(|e| cx.custom(e))?),
    |reference| Cow::Owned(TryToOwned::try_to_owned(reference).map_err(|e| cx.custom(e))?),
}

macro_rules! sequence {
    (
        $(#[$($meta:meta)*])*
        $cx:ident,
        $ty:ident <T $(: $trait0:ident $(+ $trait:ident)*)? $(, $extra:ident: $extra_bound0:ident $(+ $extra_bound:ident)*)*>,
        $insert:ident,
        $access:ident,
        $factory:expr
    ) => {
        $(#[$($meta)*])*
        impl<M, T $(, $extra)*> Encode<M> for $ty<T $(, $extra)*>
        where
            T: Encode<M>,
            $($extra: $extra_bound0 $(+ $extra_bound)*),*
        {
            const IS_BITWISE_ENCODE: bool = false;

            type Encode = Self;

            #[inline]
            fn encode<E>(&self, encoder: E) -> Result<(), E::Error>
            where
                E: Encoder<Mode = M>,
            {
                let $cx = encoder.cx();

                encoder.encode_sequence_fn(self.len(), |seq| {
                    let mut index = 0;

                    for value in self {
                        $cx.enter_sequence_index(index);
                        seq.push(value)?;
                        $cx.leave_sequence_index();
                        index = index.wrapping_add(1);
                    }

                    Ok(())
                })
            }

            #[inline]
            fn as_encode(&self) -> &Self::Encode {
                self
            }
        }

        $(#[$($meta)*])*
        impl<'de, M, A, T $(, $extra)*> Decode<'de, M, A> for $ty<T $(, $extra)*>
        where
            A: Allocator,
            T: Decode<'de, M, A> $(+ $trait0 $(+ $trait)*)*,
            $($extra: $extra_bound0 $(+ $extra_bound)*),*
        {
            const IS_BITWISE_DECODE: bool = false;

            #[inline]
            fn decode<D>(decoder: D) -> Result<Self, D::Error>
            where
                D: Decoder<'de, Mode = M, Allocator = A>,
            {
                let $cx = decoder.cx();

                decoder.decode_sequence(|$access| {
                    let mut out = $factory;

                    let mut index = 0;

                    while let Some(value) = $access.try_decode_next()? {
                        $cx.enter_sequence_index(index);
                        out.$insert(value.decode()?).map_err(|e| $cx.custom(e))?;
                        $cx.leave_sequence_index();
                        index = index.wrapping_add(1);
                    }

                    Ok(out)
                })
            }
        }

        $(#[$($meta)*])*
        impl<M, T $(, $extra)*> EncodePacked<M> for $ty<T $(, $extra)*>
        where
            T: Encode<M>,
            $($extra: $extra_bound0 $(+ $extra_bound)*),*
        {
            #[inline]
            fn encode_packed<E>(&self, encoder: E) -> Result<(), E::Error>
            where
                E: Encoder<Mode = M>,
            {
                let $cx = encoder.cx();

                encoder.encode_pack_fn(|pack| {
                    let mut index = 0;

                    for value in self {
                        $cx.enter_sequence_index(index);
                        pack.push(value)?;
                        $cx.leave_sequence_index();
                        index = index.wrapping_add(1);
                    }

                    Ok(())
                })
            }
        }
    }
}

macro_rules! slice_sequence {
    (
        $(#[$($meta:meta)*])*
        $cx:ident,
        $ty:ident <T $(, $alloc:ident)?>,
        || $new:expr,
        |$vec:ident, $value:ident| $insert:expr,
        |$reserve_vec:ident, $reserve_capacity:ident| $reserve:expr,
        |$capacity:ident| $with_capacity:expr,
    ) => {
        $(#[$($meta)*])*
        impl<M, T $(, $alloc)*> Encode<M> for $ty<T $(, $alloc)*>
        where
            T: Encode<M>,
            $($alloc: Allocator,)*
        {
            const IS_BITWISE_ENCODE: bool = false;

            type Encode = Self;

            #[inline]
            fn encode<E>(&self, encoder: E) -> Result<(), E::Error>
            where
                E: Encoder<Mode = M>,
            {
                encoder.encode_slice(self)
            }

            #[inline]
            fn as_encode(&self) -> &Self::Encode {
                self
            }
        }

        $(#[$($meta)*])*
        impl<'de, M, A, T> Decode<'de, M, A> for $ty<T $(, $alloc)*>
        where
            A: Allocator,
            T: Decode<'de, M, A>,
        {
            const IS_BITWISE_DECODE: bool = false;

            #[inline]
            fn decode<D>(decoder: D) -> Result<Self, D::Error>
            where
                D: Decoder<'de, Mode = M, Allocator = A>,
            {
                struct Builder<'de, M, A, T>
                where
                    $($alloc: Allocator,)*
                {
                    vec: $ty<T $(, $alloc)*>,
                    _marker: core::marker::PhantomData<(M, A, &'de ())>
                }

                #[allow(unused_variables)]
                impl<'de, M, A, T> DecodeSliceBuilder<T, A> for Builder<'de, M, A, T>
                where
                    T: Decode<'de, M, A>,
                    A: Allocator,
                {
                    #[inline]
                    fn new<C>($cx: C) -> Result<Self, C::Error>
                    where
                        C: Context<Allocator = A>,
                    {
                        Ok(Builder {
                            vec: $new,
                            _marker: core::marker::PhantomData
                        })
                    }

                    #[inline]
                    fn with_capacity<C>($cx: C, $capacity: usize) -> Result<Self, C::Error>
                    where
                        C: Context<Allocator = A>,
                    {
                        Ok(Builder {
                            vec: $with_capacity,
                            _marker: core::marker::PhantomData
                        })
                    }

                    #[inline]
                    fn push<C>(&mut self, $cx: C, $value: T) -> Result<(), C::Error>
                    where
                        C: Context<Allocator = A>,
                    {
                        let $vec = &mut self.vec;
                        $insert;
                        Ok(())
                    }

                    #[inline]
                    fn reserve<C>( &mut self, $cx: C, $reserve_capacity: usize) -> Result<(), C::Error>
                    where
                        C: Context<Allocator = A>,
                    {
                        let $reserve_vec = &mut self.vec;
                        $reserve;
                        Ok(())
                    }

                    #[inline]
                    unsafe fn set_len(&mut self, len: usize) {
                        self.vec.set_len(len);
                    }

                    #[inline]
                    fn as_mut_ptr(&mut self) -> *mut T {
                        self.vec.as_mut_ptr()
                    }
                }

                let Builder { vec, _marker: core::marker::PhantomData } = decoder.decode_slice()?;
                Ok(vec)
            }
        }

        $(#[$($meta)*])*
        impl<M, T $(, $alloc)*> EncodePacked<M> for $ty<T $(, $alloc)*>
        where
            T: Encode<M>,
            $($alloc: Allocator,)*
        {
            #[inline]
            fn encode_packed<E>(&self, encoder: E) -> Result<(), E::Error>
            where
                E: Encoder<Mode = M>,
            {
                encoder.encode_pack_fn(|pack| {
                    SequenceEncoder::encode_slice(pack, self)
                })
            }
        }
    }
}

slice_sequence! {
    cx,
    Vec<T>,
    || Vec::new(),
    |vec, value| vec.try_push(value).map_err(|e| cx.custom(e))?,
    |vec, capacity| vec.try_reserve(capacity).map_err(|e| cx.custom(e))?,
    |size| Vec::try_with_capacity(size).map_err(|e| cx.custom(e))?,
}

impl<M, T> Encode<M> for VecDeque<T>
where
    T: Encode<M>,
{
    type Encode = Self;

    const IS_BITWISE_ENCODE: bool = false;

    #[inline]
    fn encode<E>(&self, encoder: E) -> Result<(), E::Error>
    where
        E: Encoder<Mode = M>,
    {
        let (a, b) = self.as_slices();
        encoder.encode_slices(self.len(), [a, b])
    }

    #[inline]
    fn as_encode(&self) -> &Self::Encode {
        self
    }
}

impl<'de, M, A, T> Decode<'de, M, A> for VecDeque<T>
where
    A: Allocator,
    T: Decode<'de, M, A>,
{
    const IS_BITWISE_DECODE: bool = false;

    #[inline]
    fn decode<D>(decoder: D) -> Result<Self, D::Error>
    where
        D: Decoder<'de, Mode = M, Allocator = A>,
    {
        Ok(VecDeque::from(Vec::decode(decoder)?))
    }
}

impl<M, T> EncodePacked<M> for VecDeque<T>
where
    T: Encode<M>,
{
    #[inline]
    fn encode_packed<E>(&self, encoder: E) -> Result<(), E::Error>
    where
        E: Encoder<Mode = M>,
    {
        encoder.encode_pack_fn(|pack| {
            let (a, b) = self.as_slices();
            pack.encode_slices([a, b])
        })
    }
}

sequence! {
    cx,
    BTreeSet<T: Ord>,
    try_insert,
    seq,
    BTreeSet::new()
}

sequence! {
    cx,
    HashSet<T: Eq + Hash, S: BuildHasher + Default>,
    try_insert,
    seq,
    HashSet::try_with_capacity_and_hasher(cautious::<T>(seq.size_hint()), S::default()).map_err(|e| cx.custom(e))?
}

macro_rules! map {
    (
        $(#[$($meta:meta)*])*
        $cx:ident,
        $ty:ident<K $(: $key_bound0:ident $(+ $key_bound:ident)*)?, V $(, $extra:ident: $extra_bound0:ident $(+ $extra_bound:ident)*)*>,
        $access:ident,
        $with_capacity:expr
    ) => {
        $(#[$($meta)*])*
        impl<'de, M, K, V $(, $extra)*> Encode<M> for $ty<K, V $(, $extra)*>
        where
            K: Encode<M>,
            V: Encode<M>,
            $($extra: $extra_bound0 $(+ $extra_bound)*),*
        {
            const IS_BITWISE_ENCODE: bool = false;

            type Encode = Self;

            #[inline]
            fn encode<E>(&self, encoder: E) -> Result<(), E::Error>
            where
                E: Encoder<Mode = M>,
            {
                let hint = self.len();

                encoder.encode_map_fn(hint, |map| {
                    for (k, v) in self {
                        map.insert_entry(k, v)?;
                    }

                    Ok(())
                })
            }

            #[inline]
            fn as_encode(&self) -> &Self::Encode {
                self
            }
        }

        $(#[$($meta)*])*
        impl<'de, M, K, V $(, $extra)*> EncodeTrace<M> for $ty<K, V $(, $extra)*>
        where
            K: fmt::Display + Encode<M>,
            V: Encode<M>,
            $($extra: $extra_bound0 $(+ $extra_bound)*),*
        {
            #[inline]
            fn trace_encode<E>(&self, encoder: E) -> Result<(), E::Error>
            where
                E: Encoder<Mode = M>,
            {
                let hint = self.len();

                let $cx = encoder.cx();

                encoder.encode_map_fn(hint, |map| {
                    for (k, v) in self {
                        $cx.enter_map_key(k);
                        map.encode_entry_fn(|entry| {
                            entry.encode_key()?.encode(k)?;
                            entry.encode_value()?.encode(v)?;
                            Ok(())
                        })?;
                        $cx.leave_map_key();
                    }

                    Ok(())
                })
            }
        }

        $(#[$($meta)*])*
        impl<'de, K, V, A, M $(, $extra)*> Decode<'de, M, A> for $ty<K, V $(, $extra)*>
        where
            A: Allocator,
            K: Decode<'de, M, A> $(+ $key_bound0 $(+ $key_bound)*)*,
            V: Decode<'de, M, A>,
            $($extra: $extra_bound0 $(+ $extra_bound)*),*
        {
            const IS_BITWISE_DECODE: bool = false;

            #[inline]
            fn decode<D>(decoder: D) -> Result<Self, D::Error>
            where
                D: Decoder<'de, Mode = M, Allocator = A>,
            {
                let $cx = decoder.cx();

                decoder.decode_map(|$access| {
                    let mut out = $with_capacity;

                    while let Some((key, value)) = $access.entry()? {
                        out.try_insert(key, value).map_err(|e| $cx.custom(e))?;
                    }

                    Ok(out)
                })
            }
        }

        $(#[$($meta)*])*
        impl<'de, K, V, A, M $(, $extra)*> DecodeTrace<'de, M, A> for $ty<K, V $(, $extra)*>
        where
            A: Allocator,
            K: fmt::Display + Decode<'de, M, A> $(+ $key_bound0 $(+ $key_bound)*)*,
            V: Decode<'de, M, A>,
            $($extra: $extra_bound0 $(+ $extra_bound)*),*
        {
            #[inline]
            fn trace_decode<D>(decoder: D) -> Result<Self, D::Error>
            where
                D: Decoder<'de, Mode = M, Allocator = A>,
            {
                let $cx = decoder.cx();

                decoder.decode_map(|$access| {
                    let mut out = $with_capacity;

                    while let Some(mut entry) = $access.decode_entry()? {
                        let key = entry.decode_key()?.decode()?;
                        $cx.enter_map_key(&key);
                        let value = entry.decode_value()?.decode()?;
                        out.try_insert(key, value).map_err(|e| $cx.custom(e))?;
                        $cx.leave_map_key();
                    }

                    Ok(out)
                })
            }
        }
    }
}

map!(_cx, BTreeMap<K: Ord, V>, map, BTreeMap::new());

map!(
    cx,
    HashMap<K: Eq + Hash, V, S: BuildHasher + Default>,
    map,
    HashMap::try_with_capacity_and_hasher(cautious::<(K, V)>(map.size_hint()), S::default()).map_err(|e| cx.custom(e))?
);

macro_rules! smart_pointer {
    ($($ty:ident),* $(,)?) => {
        $(
            impl<M, T> Encode<M> for $ty<T>
            where
                T: ?Sized + Encode<M>,
            {
                const IS_BITWISE_ENCODE: bool = false;

                type Encode = T;

                #[inline]
                fn encode<E>(&self, encoder: E) -> Result<(), E::Error>
                where
                    E: Encoder<Mode = M>,
                {
                    self.as_ref().encode(encoder)
                }

                #[inline]
                fn as_encode(&self) -> &Self::Encode {
                    self
                }
            }

            impl<'de, M, A, T> Decode<'de, M, A> for $ty<T>
            where
                A: Allocator,
                T: Decode<'de, M, A>,
            {
                const IS_BITWISE_DECODE: bool = false;

                #[inline]
                fn decode<D>(decoder: D) -> Result<Self, D::Error>
                where
                    D: Decoder<'de, Mode = M, Allocator = A>,
                {
                    let cx = decoder.cx();
                    $ty::try_new(decoder.decode()?).map_err(|e| cx.custom(e))
                }
            }

            impl<'de, M, A> DecodeBytes<'de, M, A> for $ty<[u8]>
            where
                A: Allocator
            {
                const DECODE_BYTES_PACKED: bool = false;

                #[inline]
                fn decode_bytes<D>(decoder: D) -> Result<Self, D::Error>
                where
                    D: Decoder<'de, Mode = M, Allocator = A>,
                {
                    let cx = decoder.cx();
                    $ty::try_from(<Vec<u8>>::decode_bytes(decoder)?).map_err(|e| cx.custom(e))
                }
            }
        )*
    };
}

smart_pointer!(Box);

impl<M> EncodeBytes<M> for Vec<u8> {
    const ENCODE_BYTES_PACKED: bool = false;

    type EncodeBytes = [u8];

    #[inline]
    fn encode_bytes<E>(&self, encoder: E) -> Result<(), E::Error>
    where
        E: Encoder<Mode = M>,
    {
        encoder.encode_bytes(self.as_slice())
    }

    #[inline]
    fn as_encode_bytes(&self) -> &Self::EncodeBytes {
        self
    }
}

impl<M> EncodeBytes<M> for Box<[u8]> {
    const ENCODE_BYTES_PACKED: bool = false;

    type EncodeBytes = [u8];

    #[inline]
    fn encode_bytes<E>(&self, encoder: E) -> Result<(), E::Error>
    where
        E: Encoder<Mode = M>,
    {
        encoder.encode_bytes(self.as_ref())
    }

    #[inline]
    fn as_encode_bytes(&self) -> &Self::EncodeBytes {
        self
    }
}

impl<'de, M, A> DecodeBytes<'de, M, A> for Vec<u8>
where
    A: Allocator,
{
    const DECODE_BYTES_PACKED: bool = false;

    #[inline]
    fn decode_bytes<D>(decoder: D) -> Result<Self, D::Error>
    where
        D: Decoder<'de, Mode = M, Allocator = A>,
    {
        struct Visitor;

        #[musli::de::unsized_visitor]
        impl<'de, C> UnsizedVisitor<'de, C, [u8]> for Visitor
        where
            C: Context,
        {
            type Ok = Vec<u8>;

            #[inline]
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "bytes")
            }

            #[inline]
            fn visit_borrowed(self, cx: C, bytes: &'de [u8]) -> Result<Self::Ok, Self::Error> {
                Vec::try_from(bytes).map_err(|e| cx.custom(e))
            }

            #[inline]
            fn visit_ref(self, cx: C, bytes: &[u8]) -> Result<Self::Ok, Self::Error> {
                Vec::try_from(bytes).map_err(|e| cx.custom(e))
            }
        }

        decoder.decode_bytes(Visitor)
    }
}

impl<M> EncodeBytes<M> for VecDeque<u8> {
    const ENCODE_BYTES_PACKED: bool = false;

    type EncodeBytes = VecDeque<u8>;

    #[inline]
    fn encode_bytes<E>(&self, encoder: E) -> Result<(), E::Error>
    where
        E: Encoder<Mode = M>,
    {
        let (first, second) = self.as_slices();
        encoder.encode_bytes_vectored(self.len(), &[first, second])
    }

    #[inline]
    fn as_encode_bytes(&self) -> &Self::EncodeBytes {
        self
    }
}

impl<'de, M, A> DecodeBytes<'de, M, A> for VecDeque<u8>
where
    A: Allocator,
{
    const DECODE_BYTES_PACKED: bool = false;

    #[inline]
    fn decode_bytes<D>(decoder: D) -> Result<Self, D::Error>
    where
        D: Decoder<'de, Mode = M, Allocator = A>,
    {
        Ok(VecDeque::from(<Vec<u8>>::decode_bytes(decoder)?))
    }
}
