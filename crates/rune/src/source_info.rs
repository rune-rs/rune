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
    }
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
fn try_path_serde<'de, D>(deserializer: D) -> Result<Option<Box<str>>, D::Error>
where
    D: Deserializer<'de>
{
    Ok(Some(try_read_serde(deserializer)?))
}

#[cfg(feature = "musli")]
impl<'de, M, A> Decode<'de, M, A> for Source
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
                unsized_visitor,
                VariantDecoder
            }
        };

        // An tag descriptor for the Source's discriminant.
        enum SourceTag
        {
            Memory,
            Named,
        }

        // A visitor for SourceTag.
        struct TagVisitor;

        #[unsized_visitor]
        impl<'de, C> UnsizedVisitor<'de, C, str> for TagVisitor
        where
            C: Context
        {
            type Ok = SourceTag;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
            {
                f.write_str("Expecting variant tag `Memory` or `Named`")
            }

            fn visit_ref(self, context: C, value: &str) -> Result<SourceTag, C::Error>
            {
                match value
                {
                    "Memory" => Ok(SourceTag::Memory),
                    "Named" => Ok(SourceTag::Named),
                    _ => Err(context.message(format_args!("Unknown tag variant `{}`", value)))
                }
            }
        }

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

        decoder.decode_variant(|variant|
        {
            let tag : SourceTag =
            variant
            .decode_tag()?
            .decode_string(TagVisitor)?;

            let contents = variant.decode_value()?;

            let context = contents.cx();

            match tag
            {
                SourceTag::Memory =>
                {
                    contents.decode_map_hint(1, |memory|
                    {
                        let mut source = None;

                        while let Some(mut entry) = memory.decode_entry()?
                        {
                            let key : &str =
                            entry
                            .decode_key()?
                            .decode_string(GenericVisitor)?;

                            match key
                            {
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
                                "source" => return Err(context.message(format_args!("Duplicate field `{}`", key))),

                                _ => return Err(context.message(format_args!("Unknown field `{}`", key)))
                            }
                        }

                        let Some(source) = source
                        else
                        {
                            return Err(context.message("Missing `source` field!"));
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
                    contents.decode_map(|named|
                    {
                        let (mut name, mut source, mut path) = (None, None, None);

                        while let Some(mut entry) = named.decode_entry()?
                        {
                            let key : &str =
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

                                "source" | "name" | "path" =>
                                return Err(context.message(format_args!("Duplicate field `{}`", key))),

                                _ => return Err(context.message(format_args!("Unknown field `{}`", key)))
                            }
                        }

                        let Some(name) = name
                        else
                        {
                            return Err(context.message("Missing `name` field!"))
                        };

                        let Some(source) = source
                        else
                        {
                            return Err(context.message("Missing `source` field!"))
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
            }
        }
        )
    }
}

pub use Source as SourceInfo;
