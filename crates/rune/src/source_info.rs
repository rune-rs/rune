use crate::alloc::Box;

#[cfg(feature = "serde")]
use serde::{
    Deserialize,
    Deserializer,
    de::Error
};

#[cfg(feature = "musli")]
use musli::{Decode, Decoder};

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

        /// Sometimes, the path gets included in a named source. If that field is present,
        /// we want to deserialize it as a regular string.
        path: Option<Box<str>>
    }
}

#[cfg(feature = "serde")]
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename = "Source")]
struct DecodedSourceInfo
{
    #[serde(default, deserialize_with = "try_optional_serde")]
    name: Option<Box<str>>,
    #[serde(deserialize_with = "try_read_serde")]
    source: Box<str>,
    #[serde(default, deserialize_with = "try_optional_serde")]
    path: Option<Box<str>>
}

#[cfg(feature = "serde")]
#[inline]
fn try_read_serde<'de, D>(deserializer: D) -> Result<Box<str>, D::Error>
where
    D: Deserializer<'de>
{
    let str_ : &str = Deserialize::deserialize(deserializer)?;

    match Box::try_from(str_)
    {
        Ok(o) => Ok(o),
        Err(e) => Err(D::Error::custom(e))
    }
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

        match (info.name, info.path)
        {
            (None, Some(_)) => Err(D::Error::custom("`path` exists; missing field `name`")),
            (None, None) =>
            Ok(
                Self::Memory
                {
                    source: info.source
                }
            ),
            (Some(name), path) =>
            {
                let output =
                Self::Named
                {
                    name,
                    source: info.source,
                    path
                };

                Ok(output)
            }
        }
    }
}

#[cfg(feature = "musli")]
impl<'de, M, A> Decode<'de, M, A> for SourceInfo
where
    A: musli::Allocator
{
    // With serde, we can  easily implement a proper deserializer with derive macros.
    //
    // Musli, on the other hand, was a different story. For now, in order to encapsulate the
    // SourceInfo properly, we have to manually implement it.
    #[inline]
    fn decode<D>(decoder: D) -> Result<Self, D::Error>
    where
        D: Decoder<'de>
    {
        use core::fmt;

        use musli::{
            Context,
            de::{
                EntryDecoder,
                MapDecoder,
                UnsizedVisitor,
                unsized_visitor
            }
        };

        // A generic visitor for &str.
        struct GenericVisitor;

        #[unsized_visitor]
        impl<'de, C> UnsizedVisitor<'de, C, str> for GenericVisitor
        where
            C: Context
        {
            type Ok = &'de str;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
            {
                f.write_str("Expecting a valid sequence of UTF8 bytes")
            }

            fn visit_borrowed(self, _: C, value: &'de str) -> Result<&'de str, C::Error>
            {
                Ok(value)
            }
        }

        // Tries to create a boxed string from a borrowed str value.
        fn musli_try_box<C>(val: &str, context: &C) -> Result<Box<str>, C::Error>
        where
            C: Context
        {
            match Box::try_from(val)
            {
                Ok(o) => Ok(o),
                Err(e) => Err(context.custom(e))
            }
        }

        decoder.decode_map(|map|
        {
            let context = map.cx();

            let (mut name, mut source, mut path) = (None, None, None);

            while let Some(mut entry) = map.decode_entry()?
            {
                let key =
                entry
                .decode_key()?
                .decode_string(GenericVisitor)?;

                match key
                {
                    "name"
                    if name.is_none()
                    =>
                    {
                        let value =
                        entry
                        .decode_value()?
                        .decode_string(GenericVisitor)?;

                        name = Some(musli_try_box(value, &context)?);
                    }

                    "source"
                    if source.is_none()
                    =>
                    {
                        let value =
                        entry
                        .decode_value()?
                        .decode_string(GenericVisitor)?;

                        source = Some(musli_try_box(value, &context)?);
                    }

                    "path"
                    if path.is_none()
                    =>
                    {
                        let value =
                        entry
                        .decode_value()?
                        .decode_string(GenericVisitor)?;

                        path = Some(musli_try_box(value, &context)?);
                    }

                    "name" | "source" | "path" =>
                    return Err(context.message(format_args!("duplicate field `{}`", key))),

                    _ =>
                    return Err(context.message(format_args!("unknown field `{}`", key)))
                }
            }

            let Some(source) = source
            else
            {
                return Err(context.message("missing field `source`"));
            };

            match (name, path)
            {
                (None, Some(_)) => Err(context.message("`path` exists; missing field `name`")),

                (None, None) =>
                Ok(
                    Source::Memory
                    {
                        source
                    }
                ),

                (Some(name), path) =>
                {
                    let output =
                    Source::Named
                    {
                        name,
                        source,
                        path
                    };

                    Ok(output)
                }
            }
        }
        )
    }
}
