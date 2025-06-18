use core::fmt;
use core::num;

#[cfg(feature = "musli")]
use musli::{Decode, Encode, Encoder, Decoder};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize, Serializer, Deserializer};

use crate as rune;
use crate::alloc;
use crate::alloc::path::Path;
use crate::alloc::prelude::*;
use crate::ast::Span;
use crate::source::Source;
#[cfg(feature = "codespan-reporting")]
use codespan_reporting::files;

/// Helper macro to define a collection of sources populatedc with the given
/// entries.
///
/// Calling this macro is fallible with [alloc::Error], so you should do it in a
/// function that returns a `Result`.
///
/// ```
/// let sources = rune::sources! {
///     entry => {
///         pub fn main() {
///             42
///         }
///     }
/// };
/// # Ok::<_, rune::support::Error>(())
/// ```
#[macro_export]
macro_rules! sources {
    ($($name:ident => {$($tt:tt)*}),* $(,)?) => {{
        let mut sources = $crate::Sources::new();
        $(sources.insert($crate::Source::new(stringify!($name), stringify!($($tt)*))?)?;)*
        sources
    }};
}

/// A collection of source files.
#[derive(Debug, Default)]
pub struct Sources {
    /// Sources associated.
    sources: Vec<Source>,
}

impl Sources {
    /// Construct a new collection of sources.
    #[inline]
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Insert a source and return its [`SourceId`].
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::{Sources, Source};
    ///
    /// let mut sources = Sources::new();
    /// let id = sources.insert(Source::new("<memory>", "pub fn main() { 10 }")?)?;
    /// let id2 = sources.insert(Source::new("<memory>", "pub fn main() { 10 }")?)?;
    /// assert_ne!(id, id2);
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn insert(&mut self, source: Source) -> alloc::Result<SourceId> {
        let id =
            SourceId::try_from(self.sources.len()).expect("could not build a source identifier");
        self.sources.try_push(source)?;
        Ok(id)
    }

    /// Get the source matching the given source id.
    ///
    /// # Examples
    ///
    /// ```
    /// # use anyhow::Context;
    /// use rune::{Sources, Source};
    ///
    /// let mut sources = Sources::new();
    /// let id = sources.insert(Source::new("<memory>", "pub fn main() { 10 }")?)?;
    ///
    /// let source = sources.get(id).context("expected source")?;
    ///
    /// assert_eq!(source.name(), "<memory>");
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[inline]
    pub fn get(&self, id: SourceId) -> Option<&Source> {
        self.sources.get(id.into_index())
    }

    /// Fetch name for the given source id.
    #[inline]
    pub(crate) fn name(&self, id: SourceId) -> Option<&str> {
        let source = self.sources.get(id.into_index())?;
        Some(source.name())
    }

    /// Fetch source for the given span.
    #[inline]
    pub(crate) fn source(&self, id: SourceId, span: Span) -> Option<&str> {
        let source = self.sources.get(id.into_index())?;
        source.get(span.range())
    }

    /// Access the optional path of the given source id.
    #[inline]
    pub(crate) fn path(&self, id: SourceId) -> Option<&Path> {
        let source = self.sources.get(id.into_index())?;
        source.path()
    }

    /// Get all available source ids.
    #[inline]
    pub(crate) fn source_ids(&self) -> impl Iterator<Item = SourceId> {
        (0..self.sources.len()).map(|index| SourceId::new(index as u32))
    }

    /// Iterate over all registered sources.
    #[cfg(feature = "cli")]
    #[inline]
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Source> {
        self.sources.iter()
    }
}

#[cfg(feature = "codespan-reporting")]
impl<'a> files::Files<'a> for Sources {
    type FileId = SourceId;
    type Name = &'a str;
    type Source = &'a str;

    #[inline]
    fn name(&'a self, file_id: SourceId) -> Result<Self::Name, files::Error> {
        let source = self.get(file_id).ok_or(files::Error::FileMissing)?;
        Ok(source.name())
    }

    #[inline]
    fn source(&'a self, file_id: SourceId) -> Result<Self::Source, files::Error> {
        let source = self.get(file_id).ok_or(files::Error::FileMissing)?;
        Ok(source.as_str())
    }

    #[cfg(feature = "emit")]
    #[inline]
    fn line_index(&self, file_id: SourceId, byte_index: usize) -> Result<usize, files::Error> {
        let source = self.get(file_id).ok_or(files::Error::FileMissing)?;
        Ok(source.line_index(byte_index))
    }

    #[cfg(feature = "emit")]
    #[inline]
    fn line_range(
        &self,
        file_id: SourceId,
        line_index: usize,
    ) -> Result<std::ops::Range<usize>, files::Error> {
        let source = self.get(file_id).ok_or(files::Error::FileMissing)?;
        let range = source
            .line_range(line_index)
            .ok_or_else(|| files::Error::LineTooLarge {
                given: line_index,
                max: source.line_count(),
            })?;
        Ok(range)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Sources
{
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        use serde::de::{Error, SeqAccess, Visitor};

        // A built-in sequence visitor for importing Sources.
        //
        // This guarantees that we catch Allocation errors and
        // table overflows during deserialization.
        struct SourcesVisitor;

        impl<'de> Visitor<'de> for SourcesVisitor
        {
            type Value = Vec<Source>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
            {
                f.write_str("A collection of Source objects")
            }

            fn visit_seq<A>(self, mut sequence: A) -> Result<Vec<Source>, A::Error>
            where
                A: SeqAccess<'de>
            {
                let mut table = Vec::new();

                // For preventing Source ID overflows.
                let mut counter = 0_u32;

                while let Some(source) = sequence.next_element()?
                {
                    if counter != u32::MAX
                    {
                        counter += 1;

                        if let Err(e) = table.try_push(source)
                        {
                            return Err(A::Error::custom(e));
                        }
                    }
                    else
                    {
                        return Err(A::Error::custom("source table exceeded max capacity"));
                    }
                }

                Ok(table)
            }
        }

        let sources : Vec<Source> =
        deserializer.deserialize_seq(SourcesVisitor)?;

        Ok(
            Self
            {
                sources
            }
        )
    }
}

#[cfg(feature = "serde")]
impl Serialize for Sources
{
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        use serde::ser::SerializeSeq;

        let mut sequence = serializer.serialize_seq(Some(self.sources.len()))?;

        for source in self.sources.iter()
        {
            sequence.serialize_element(source)?;
        }

        sequence.end()
    }
}

#[cfg(feature = "musli")]
impl<'de, M, A> Decode<'de, M, A> for Sources
where
    A: musli::Allocator
{
    #[inline]
    fn decode<D>(decoder: D) -> Result<Self, D::Error>
    where
        D: Decoder<'de>
    {
        use musli::{Context, de::SequenceDecoder};

        let context = decoder.cx();

        let sources : Vec<Source> =
        decoder.decode_sequence(|seq|
        {
            let mut table = Vec::new();

            // For preventing Source ID overflows.
            let mut counter = 0_u32;

            while let Some(element) = seq.try_decode_next()?
            {
                let source = element.decode::<Source>()?;

                if counter != u32::MAX
                {
                    counter += 1;

                    if let Err(e) = table.try_push(source)
                    {
                        return Err(context.custom(e));
                    }
                }
                else
                {
                    return Err(context.message("Source table exceeded max capacity"));
                }
            }

            Ok(table)
        }
        )?;

        Ok(
            Self
            {
                sources
            }
        )
    }
}

#[cfg(feature = "musli")]
impl<M> Encode<M> for Sources
{
    type Encode = Self;

    #[inline]
    fn encode<E>(&self, encoder: E) -> Result<(), E::Error>
    where
        E: Encoder<Mode = M>
    {
        use musli::en::SequenceEncoder;

        let mut sources = encoder.encode_sequence(self.sources.len())?;

        for source in self.sources.iter()
        {
            sources.push(source)?;
        }

        sources.finish_sequence()
    }

    #[inline]
    fn as_encode(&self) -> &Self
    {
        self
    }
}

/// The opaque identifier of a source file, as returned by
/// [`Sources::insert`].
///
/// It can be used to reference the inserted source file in the future through
/// methods such as [`Sources::get`].
#[derive(TryClone, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[try_clone(copy)]
#[repr(transparent)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(transparent))]
#[cfg_attr(feature = "musli", derive(Encode, Decode), musli(transparent))]
pub struct SourceId {
    index: u32,
}

impl SourceId {
    /// The empty source identifier.
    pub const EMPTY: Self = Self::empty();

    /// Construct a source identifier from an index.
    #[inline]
    pub const fn new(index: u32) -> Self {
        Self { index }
    }

    /// Define an empty source identifier that cannot reference a source.
    #[inline]
    pub const fn empty() -> Self {
        Self { index: u32::MAX }
    }

    /// Access the source identifier as an index.
    #[inline]
    pub fn into_index(self) -> usize {
        usize::try_from(self.index).expect("source id out of bounds")
    }
}

impl fmt::Debug for SourceId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.index.fmt(f)
    }
}

impl fmt::Display for SourceId {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.index.fmt(f)
    }
}

impl Default for SourceId {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

impl TryFrom<usize> for SourceId {
    type Error = num::TryFromIntError;

    #[inline]
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(Self {
            index: u32::try_from(value)?,
        })
    }
}
