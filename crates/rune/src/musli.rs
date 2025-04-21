pub(crate) mod ordering {
    use core::cmp::Ordering;

    use musli::{Context, Decoder, Encoder};

    pub(crate) fn encode<E>(value: &Ordering, encoder: E) -> Result<(), E::Error>
    where
        E: Encoder,
    {
        match value {
            Ordering::Less => encoder.encode_i8(-1),
            Ordering::Equal => encoder.encode_i8(0),
            Ordering::Greater => encoder.encode_i8(1),
        }
    }

    pub(crate) fn decode<'de, D>(decoder: D) -> Result<Ordering, D::Error>
    where
        D: Decoder<'de>,
    {
        let cx = decoder.cx();

        match decoder.decode_i8()? {
            -1 => Ok(Ordering::Less),
            0 => Ok(Ordering::Equal),
            1 => Ok(Ordering::Greater),
            _ => Err(cx.message("invalid ordering")),
        }
    }
}
