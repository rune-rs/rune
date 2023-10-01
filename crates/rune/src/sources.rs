use core::fmt;
use core::num;

use crate as rune;
use crate::alloc::path::Path;
use crate::alloc::prelude::*;
use crate::alloc::{self, Vec};
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
///
/// Ok::<_, rune::support::Error>(())
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
    pub fn get(&self, id: SourceId) -> Option<&Source> {
        self.sources.get(id.into_index())
    }

    /// Fetch name for the given source id.
    pub(crate) fn name(&self, id: SourceId) -> Option<&str> {
        let source = self.sources.get(id.into_index())?;
        Some(source.name())
    }

    /// Fetch source for the given span.
    pub(crate) fn source(&self, id: SourceId, span: Span) -> Option<&str> {
        let source = self.sources.get(id.into_index())?;
        source.get(span.range())
    }

    /// Access the optional path of the given source id.
    pub(crate) fn path(&self, id: SourceId) -> Option<&Path> {
        let source = self.sources.get(id.into_index())?;
        source.path()
    }

    /// Get all available source ids.
    pub(crate) fn source_ids(&self) -> impl Iterator<Item = SourceId> {
        (0..self.sources.len()).map(|index| SourceId::new(index as u32))
    }

    /// Iterate over all registered sources.
    #[cfg(feature = "cli")]
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Source> {
        self.sources.iter()
    }
}

#[cfg(feature = "codespan-reporting")]
impl<'a> files::Files<'a> for Sources {
    type FileId = SourceId;
    type Name = &'a str;
    type Source = &'a str;

    fn name(&'a self, file_id: SourceId) -> Result<Self::Name, files::Error> {
        let source = self.get(file_id).ok_or(files::Error::FileMissing)?;
        Ok(source.name())
    }

    fn source(&'a self, file_id: SourceId) -> Result<Self::Source, files::Error> {
        let source = self.get(file_id).ok_or(files::Error::FileMissing)?;
        Ok(source.as_str())
    }

    #[cfg(feature = "emit")]
    fn line_index(&self, file_id: SourceId, byte_index: usize) -> Result<usize, files::Error> {
        let source = self.get(file_id).ok_or(files::Error::FileMissing)?;
        Ok(source.line_index(byte_index))
    }

    #[cfg(feature = "emit")]
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

/// The opaque identifier of a source file, as returned by
/// [`Sources::insert`].
///
/// It can be used to reference the inserted source file in the future through
/// methods such as [`Sources::get`].
#[derive(TryClone, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[try_clone(copy)]
#[repr(transparent)]
pub struct SourceId {
    index: u32,
}

impl SourceId {
    /// The empty source identifier.
    pub const EMPTY: Self = Self::empty();

    /// Construct a source identifier from an index.
    pub const fn new(index: u32) -> Self {
        Self { index }
    }

    /// Define an empty source identifier that cannot reference a source.
    pub const fn empty() -> Self {
        Self { index: u32::MAX }
    }

    /// Access the source identifier as an index.
    pub fn into_index(self) -> usize {
        usize::try_from(self.index).expect("source id out of bounds")
    }
}

impl fmt::Debug for SourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.index.fmt(f)
    }
}

impl fmt::Display for SourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.index.fmt(f)
    }
}

impl Default for SourceId {
    fn default() -> Self {
        Self::empty()
    }
}

impl TryFrom<usize> for SourceId {
    type Error = num::TryFromIntError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(Self {
            index: u32::try_from(value)?,
        })
    }
}

impl serde::Serialize for SourceId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.index.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for SourceId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            index: u32::deserialize(deserializer)?,
        })
    }
}
