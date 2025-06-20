use crate::alloc::Box;

#[cfg(feature = "serde")]
use serde::{
    Deserialize,
    Deserializer,
    de::Error
};

#[cfg(feature = "musli")]
use musli::{Decode, Decoder, Context, mode};

/// Deserialized information for constructing an actual Source module.
pub enum SourceInfo
{
    /// An unnamed module.
    Memory
    {
        source: Box<str>
    },

    /// A named module.
    Named
    {
        name: Box<str>,
        source: Box<str>,
        path: Option<Box<str>>
    }
}

#[cfg_attr(feature = "serde", derive(Deserialize), serde(deny_unknown_fields, rename = "Source"))]
#[cfg_attr(feature = "musli", derive(Decode))]
struct DecodedSourceInfo
{
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "try_optional_serde"))]
    #[cfg_attr(feature = "musli", musli(default, with = self::try_optional_musli))]
    name: Option<Box<str>>,

    #[cfg_attr(feature = "serde", serde(deserialize_with = "try_read_serde"))]
    source: Box<str>,

    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "try_optional_serde"))]
    #[cfg_attr(feature = "musli", musli(default, with = self::try_optional_musli))]
    path: Option<Box<str>>
}

impl DecodedSourceInfo
{
    fn build(self) -> Option<SourceInfo>
    {
        match (self.name, self.path)
        {
            (None, Some(_)) => None,

            (None, None) =>
            {
                Some(
                    SourceInfo::Memory
                    {
                        source: self.source
                    }
                )
            }

            (Some(name), path) =>
            {
                Some(
                    {
                        SourceInfo::Named
                        {
                            name,
                            source: self.source,
                            path
                        }
                    }
                )
            }
        }
    }
}

#[cfg(feature = "musli")]
mod try_optional_musli
{
    use musli::Decoder;
    use crate::alloc::Box;

    #[inline]
    pub fn decode<'de, D>(decoder: D) -> Result<Option<Box<str>>, D::Error>
    where
        D: Decoder<'de>
    {
        Ok(Some(decoder.decode()?))
    }
}

#[cfg(feature = "serde")]
#[inline]
fn try_read_serde<'de, D>(deserializer: D) -> Result<Box<str>, D::Error>
where
    D: Deserializer<'de>
{
    use core::fmt;

    use serde::de::{Error, Visitor};

    // This only exists so that we can deseralize strings, regardless of
    // their borrow status.
    struct StringVisitor;

    impl<'de> Visitor<'de> for StringVisitor
    {
        type Value = Box<str>;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
        {
            f.write_str("A valid UTF-8 string")
        }

        fn visit_str<E>(self, v: &str) -> Result<Box<str>, E>
        where
            E: Error
        {
            match Box::try_from(v)
            {
                Ok(v) => Ok(v),
                Err(e) => Err(E::custom(e))
            }
        }

        fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Box<str>, E>
        where
            E: Error
        {
            self.visit_str(v)
        }
    }


    deserializer.deserialize_string(StringVisitor)
}

#[cfg(feature = "serde")]
#[inline]
fn try_optional_serde<'de, D>(deserializer: D) -> Result<Option<Box<str>>, D::Error>
where
    D: Deserializer<'de>
{
    Ok(Some(try_read_serde(deserializer)?))
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for SourceInfo
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        let info = DecodedSourceInfo::deserialize(deserializer)?;

        match info.build()
        {
            Some(output) => Ok(output),
            None => Err(D::Error::custom("`path` exists; missing field `name`"))
        }
    }
}

// With musli, derive(Decode) seems to only work on two modes: Text and Binary.
//
// Since we cannot implement a generic Mode, we have to implement Decode twice.
#[cfg(feature = "musli")]
impl<'de, A> Decode<'de, mode::Text, A> for SourceInfo
where
    A: musli::Allocator
{
    #[inline]
    fn decode<D>(decoder: D) -> Result<Self, D::Error>
    where
        D: Decoder<'de, Mode = mode::Text>
    {
        let context = decoder.cx();

        let info : DecodedSourceInfo = decoder.decode()?;

        match info.build()
        {
            Some(output) => Ok(output),
            None => Err(context.message("`path` exists; missing field `name`"))
        }
    }
}

#[cfg(feature = "musli")]
impl<'de, A> Decode<'de, mode::Binary, A> for SourceInfo
where
    A: musli::Allocator
{
    #[inline]
    fn decode<D>(decoder: D) -> Result<Self, D::Error>
    where
        D: Decoder<'de, Mode = mode::Binary>
    {
        let context = decoder.cx();

        let info : DecodedSourceInfo = decoder.decode()?;

        match info.build()
        {
            Some(output) => Ok(output),
            None => Err(context.message("`path` exists; missing field `name`"))
        }
    }
}
