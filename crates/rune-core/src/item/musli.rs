use musli_core::{Allocator, Context, Decode, Decoder, Encode, Encoder};

use crate::alloc::Vec;
use crate::item::internal;
use crate::item::{Item, ItemBuf};

impl<M> Encode<M> for Item {
    type Encode = [u8];

    #[inline]
    fn encode<E>(&self, encoder: E) -> Result<(), E::Error>
    where
        E: Encoder<Mode = M>,
    {
        self.as_bytes().encode(encoder)
    }

    #[inline]
    fn as_encode(&self) -> &Self::Encode {
        self.as_bytes()
    }
}

impl<M> Encode<M> for ItemBuf {
    type Encode = [u8];

    #[inline]
    fn encode<E>(&self, encoder: E) -> Result<(), E::Error>
    where
        E: Encoder<Mode = M>,
    {
        self.as_bytes().encode(encoder)
    }

    #[inline]
    fn as_encode(&self) -> &Self::Encode {
        self.as_bytes()
    }
}

impl<'de, M, A> Decode<'de, M, A> for ItemBuf
where
    A: Allocator,
{
    const IS_BITWISE_DECODE: bool = false;

    #[inline]
    fn decode<D>(decoder: D) -> Result<ItemBuf, D::Error>
    where
        D: Decoder<'de>,
    {
        let cx = decoder.cx();
        let bytes = Vec::<u8>::decode(decoder)?;

        if !internal::validate(&bytes) {
            return Err(cx.message("invalid item encoding"));
        }

        // SAFETY: The bytes were just checked to be the encoding of an item.
        unsafe { Ok(ItemBuf::from_raw(bytes)) }
    }
}
