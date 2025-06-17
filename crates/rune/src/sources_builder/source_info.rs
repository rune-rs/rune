use crate::alloc::Box;

#[cfg(feature = "serde")]
use serde::{
    Deserialize,
    Deserializer,
    de::Error
};

#[cfg(feature = "musli")]
use musli::Decode;

/// Deserialized information for constructing an actual Source module.
///
/// It shares the same name in order to prevent further headaches from
/// both musli and serde.
#[cfg_attr(feature = "musli", derive(Decode))]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum Source
{
    /// An unnamed module.
    Memory
    {
        #[cfg_attr(feature = "serde", serde(deserialize_with = "try_read_serde"))]
        #[cfg_attr(feature = "musli", musli(with = self::musli_read))]
        source: Box<str>
    },

    /// A named module.
    Named
    {
        #[cfg_attr(feature = "serde", serde(deserialize_with = "try_read_serde"))]
        #[cfg_attr(feature = "musli", musli(with = self::musli_read))]
        name: Box<str>,

        #[cfg_attr(feature = "serde", serde(deserialize_with = "try_read_serde"))]
        #[cfg_attr(feature = "musli", musli(with = self::musli_read))]
        source: Box<str>,

        /// Sometimes, the path gets included in a named source. If that field is present,
        /// we want to deserialize it as a regular string.
        #[cfg_attr(feature = "serde", serde(default, deserialize_with = "try_path_serde"))]
        #[cfg_attr(feature = "musli", musli(default, with = self::musli_path))]
        path: Option<Box<str>>
    },

    /// A module imported from a file path.
    #[cfg(feature = "std")]
    Imported
    {
        #[cfg_attr(feature = "serde", serde(deserialize_with = "try_read_serde"))]
        #[cfg_attr(feature = "musli", musli(with = self::musli_read))]
        path: Box<str>
    }
}

#[cfg(feature = "serde")]
fn try_read_serde<'de, D>(d: D) -> Result<Box<str>, D::Error>
where
    D: Deserializer<'de>
{
    let str_ : &str = Deserialize::deserialize(d)?;

    match Box::try_from(str_)
    {
        Ok(o) => Ok(o),
        Err(e) => Err(D::Error::custom(e))
    }
}

#[cfg(feature = "serde")]
fn try_path_serde<'de, D>(d: D) -> Result<Option<Box<str>>, D::Error>
where
    D: Deserializer<'de>
{
    Ok(Some(try_read_serde(d)?))
}

#[cfg(feature = "musli")]
mod musli_read
{
    use musli::{Decoder, Context};
    use super::Box;

    pub fn decode<'de, D>(d: D) -> Result<Box<str>, D::Error>
    where
    D: Decoder<'de>
    {
        let context = d.cx();

        let str_ : &str = Decoder::decode(d)?;

        match Box::try_from(str_)
        {
            Ok(o) => Ok(o),
            Err(e) => Err(context.custom(e))
        }
    }
}

#[cfg(feature = "musli")]
mod musli_path
{
    use musli::Decoder;
    use super::{Box, musli_read};

    pub fn decode<'de, D>(d: D) -> Result<Option<Box<str>>, D::Error>
    where
        D: Decoder<'de>
    {
        Ok(Some(musli_read::decode(d)?))
    }
}

pub use Source as SourceInfo;
