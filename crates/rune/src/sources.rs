use crate::ast::Span;
use crate::{Source, SourceId};
#[cfg(feature = "codespan-reporting")]
use codespan_reporting::files;
use std::convert::TryFrom;
use std::path::Path;

/// Helper macro to define a collection of sources populatedc with the given
/// entries.
///
/// ```
/// let sources = rune::sources! {
///     entry => {
///         pub fn main() {
///             42
///         }
///     }
/// };
/// ```
#[macro_export]
macro_rules! sources {
    ($($name:ident => {$($tt:tt)*}),* $(,)?) => {{
        let mut sources = $crate::Sources::new();
        $(sources.insert($crate::Source::new(stringify!($name), stringify!($($tt)*)));)*
        sources
    }};
}

/// A collection of source files, and a queue of things to compile.
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

    /// Get the source matching the given source id.
    pub fn get(&self, id: SourceId) -> Option<&Source> {
        self.sources.get(id.into_index())
    }

    /// Insert a source to be built and return its id.
    pub fn insert(&mut self, source: Source) -> SourceId {
        let id =
            SourceId::try_from(self.sources.len()).expect("could not build a source identifier");
        self.sources.push(source);
        id
    }

    /// Fetch name for the given source id.
    pub fn name(&self, id: SourceId) -> Option<&str> {
        let source = self.sources.get(id.into_index())?;
        Some(source.name())
    }

    /// Fetch source for the given span.
    pub fn source(&self, id: SourceId, span: Span) -> Option<&str> {
        let source = self.sources.get(id.into_index())?;
        source.get(span.range())
    }

    /// Access the optional path of the given source id.
    pub fn path(&self, id: SourceId) -> Option<&Path> {
        let source = self.sources.get(id.into_index())?;
        source.path()
    }

    /// Get all available source ids.
    pub(crate) fn source_ids(&self) -> impl Iterator<Item = SourceId> {
        (0..self.sources.len()).map(|index| SourceId::new(index as u32))
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

    fn line_index(&self, file_id: SourceId, byte_index: usize) -> Result<usize, files::Error> {
        let source = self.get(file_id).ok_or(files::Error::FileMissing)?;
        Ok(source.line_index(byte_index))
    }

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
