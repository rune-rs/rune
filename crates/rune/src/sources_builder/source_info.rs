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
///
/// It shares the same name in order to prevent further headaches from
/// both musli and serde.
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum Source
{
    /// An unnamed module.
    Memory
    {
        #[cfg_attr(feature = "serde", serde(deserialize_with = "try_read_serde"))]
        source: Box<str>
    },

    /// A named module.
    Named
    {
        #[cfg_attr(feature = "serde", serde(deserialize_with = "try_read_serde"))]
        name: Box<str>,

        #[cfg_attr(feature = "serde", serde(deserialize_with = "try_read_serde"))]
        source: Box<str>,

        /// Sometimes, the path gets included in a named source. If that field is present,
        /// we want to deserialize it as a regular string.
        #[cfg_attr(feature = "serde", serde(default, deserialize_with = "try_path_serde"))]
        path: Option<Box<str>>
    },

    /// A module imported from a file path.
    #[cfg(feature = "std")]
    Imported
    {
        #[cfg_attr(feature = "serde", serde(deserialize_with = "try_read_serde"))]
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
impl<'de, M, A> Decode<'de, M, A> for Source
where
    A: musli::Allocator
{
    fn decode<D>(decoder: D) -> Result<Self, D::Error>
    where
        D: Decoder<'de>
    {
        use core::fmt;
        use musli::{
            Context,
            de::{
                unsized_visitor,
                UnsizedVisitor,
                VariantDecoder,
                MapDecoder,
                EntryDecoder
            }
        };

        enum SourceTag
        {
            Memory,
            Named,
            #[cfg(feature = "std")]
            Imported
        }

        struct TagVisitor;

        struct GenericVisitor;

        #[unsized_visitor]
        impl<'de, C> UnsizedVisitor<'de, C, str> for TagVisitor
        where
            C: Context
        {
            type Ok = SourceTag;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
            {
                #[cfg(feature = "std")]
                {
                    f.write_str("Expecting variant tag `Memory`, `Named`, or `Imported`")
                }
                #[cfg(not(feature = "std"))]
                {
                    f.write_str("Expecting variant tag `Memory` or `Named`")
                }
            }

            fn visit_ref(self, ctx: C, value: &str) -> Result<SourceTag, C::Error>
            {
                match value
                {
                    "Memory" => Ok(SourceTag::Memory),
                    "Named" => Ok(SourceTag::Named),
                    #[cfg(feature = "std")]
                    "Imported" => Ok(SourceTag::Imported),
                    _ => Err(ctx.message(format_args!("Unknown tag variant `{}`", value)))
                }
            }
        }

        #[unsized_visitor]
        impl<'de, C> UnsizedVisitor<'de, C, str> for GenericVisitor
        where
        C: Context
        {
            type Ok = &'de str;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
            {
                {
                    f.write_str("Expecting a valid sequence of UTF8 bytes")
                }
            }

            fn visit_borrowed(self, _: C, value: &'de str) -> Result<&'de str, C::Error>
            {
                Ok(value)
            }
        }

        fn musli_try_box<C>(val: &str, c: &C) -> Result<Box<str>, C::Error>
        where
            C: Context
        {
            match Box::try_from(val)
            {
                Ok(o) => Ok(o),
                Err(e) => Err(c.custom(e))
            }
        }

        decoder.decode_variant(|variant|
        {
            let tag : SourceTag = variant.decode_tag()?.decode_string(TagVisitor)?;
            let contents = variant.decode_value()?;

            let cx = contents.cx();

            match tag
            {
                SourceTag::Memory =>
                {
                    contents.decode_map_hint(1, |c|
                    {
                        let mut found = None;

                        while let Some(mut item) = c.decode_entry()?
                        {
                            let key : &str = item.decode_key()?.decode_string(GenericVisitor)?;

                            match key
                            {
                                "source"
                                if found.is_none()
                                =>
                                {
                                    let value = item.decode_value()?.decode_string(GenericVisitor)?;
                                    found = Some(musli_try_box(value, &cx)?);
                                }
                                "source" => return Err(cx.message(format_args!("Duplicate field `{}`", key))),
                                _ => return Err(cx.message(format_args!("Unknown field `{}`", key)))
                            }
                        }

                        let Some(source) = found
                        else
                        {
                            return Err(cx.message("Missing `source` field!"))
                        };

                        Ok(
                            Self::Memory
                            {
                                source
                            }
                        )
                    }
                    )
                }
                SourceTag::Named =>
                {
                    contents.decode_map( |c|
                    {
                        let mut source = None;
                        let mut path = None;
                        let mut name = None;

                        while let Some(mut item) = c.decode_entry()?
                        {
                            let key : &str = item.decode_key()?.decode_string(GenericVisitor)?;

                            match key
                            {
                                "name"
                                if name.is_none()
                                =>
                                {
                                    let value = item.decode_value()?.decode_string(GenericVisitor)?;
                                    name = Some(musli_try_box(value, &cx)?);
                                }
                                "source"
                                if source.is_none()
                                =>
                                {
                                    let value = item.decode_value()?.decode_string(GenericVisitor)?;
                                    source = Some(musli_try_box(value, &cx)?);
                                }
                                "path"
                                if path.is_none()
                                =>
                                {
                                    let value = item.decode_value()?.decode_string(GenericVisitor)?;
                                    path = Some(musli_try_box(value, &cx)?);
                                }
                                "source" | "name" | "path" => return Err(cx.message(format_args!("Duplicate field `{}`", key))),
                                _ => return Err(cx.message(format_args!("Unknown field `{}`", key)))
                            }
                        }

                        let Some(name) = name
                        else
                        {
                            return Err(cx.message("Missing `name` field!"))
                        };

                        let Some(source) = source
                        else
                        {
                            return Err(cx.message("Missing `source` field!"))
                        };

                        Ok(
                            Self::Named
                            {
                                name,
                                source,
                                path
                            }
                        )
                    }
                    )
                }
                #[cfg(feature = "std")]
                SourceTag::Imported =>
                {
                    contents.decode_map_hint(1, |c|
                    {
                        let mut found = None;

                        while let Some(mut item) = c.decode_entry()?
                        {
                            let key : &str = item.decode_key()?.decode_string(GenericVisitor)?;

                            match key
                            {
                                "path"
                                if found.is_none()
                                =>
                                {
                                    let value = item.decode_value()?.decode_string(GenericVisitor)?;
                                    found = Some(musli_try_box(value, &cx)?);
                                }
                                "path" => return Err(cx.message(format_args!("Duplicate field `{}`", key))),
                                _ => return Err(cx.message(format_args!("Unknown field `{}`", key)))
                            }
                        }

                        let Some(path) = found
                        else
                        {
                            return Err(cx.message("Missing `path` field!"))
                        };

                        Ok(
                            Self::Imported
                            {
                                path
                            }
                        )
                    }
                    )
                }
            }
        }
        )
    }
}

pub use Source as SourceInfo;
