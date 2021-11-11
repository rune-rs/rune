use crate::{Source, SourceId, Span};
#[cfg(feature = "codespan-reporting")]
use codespan_reporting::files;
use std::convert::TryFrom;
use std::path::Path;
use std::sync::Arc;

/// A collection of source files, and a queue of things to compile.
#[derive(Debug, Default)]
pub struct Sources {
    /// Sources associated.
    sources: Vec<Arc<Source>>,
}

impl Sources {
    /// Construct a new collection of sources.
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Get the source at the given source id.
    pub fn source_at(&self, source_id: SourceId) -> Option<&Arc<Source>> {
        self.sources.get(source_id.into_index())
    }

    /// Insert a source to be built and return its id.
    #[deprecated = "use `insert` instead"]
    pub fn insert_default(&mut self, source: Source) -> SourceId {
        self.insert(source)
    }

    /// Insert a source to be built and return its id.
    pub fn insert(&mut self, source: Source) -> SourceId {
        let source_id =
            SourceId::try_from(self.sources.len()).expect("could not build a source identifier");
        self.sources.push(Arc::new(source));
        source_id
    }

    /// Fetch name for the given source id.
    pub fn name(&self, source_id: SourceId) -> Option<&str> {
        let source = self.sources.get(source_id.into_index())?;
        Some(source.name())
    }

    /// Fetch source for the given span.
    pub fn source(&self, source_id: SourceId, span: Span) -> Option<&str> {
        let source = self.sources.get(source_id.into_index())?;
        source.source(span)
    }

    /// Access the optional path of the given source id.
    pub fn path(&self, source_id: SourceId) -> Option<&Path> {
        let source = self.sources.get(source_id.into_index())?;
        source.path()
    }

    /// Get the source matching the given source id.
    pub fn get(&self, source_id: SourceId) -> Option<&Arc<Source>> {
        self.sources.get(source_id.into_index())
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
        let source = self.get(file_id).ok_or_else(|| files::Error::FileMissing)?;
        Ok(source.name())
    }

    fn source(&'a self, file_id: SourceId) -> Result<Self::Source, files::Error> {
        let source = self.get(file_id).ok_or_else(|| files::Error::FileMissing)?;
        Ok(source.as_str())
    }

    fn line_index(&self, file_id: SourceId, byte_index: usize) -> Result<usize, files::Error> {
        let source = self.get(file_id).ok_or_else(|| files::Error::FileMissing)?;
        Ok(source.line_index(byte_index))
    }

    fn line_range(
        &self,
        file_id: SourceId,
        line_index: usize,
    ) -> Result<std::ops::Range<usize>, files::Error> {
        let source = self.get(file_id).ok_or_else(|| files::Error::FileMissing)?;
        let range = source
            .line_range(line_index)
            .ok_or_else(|| files::Error::LineTooLarge {
                given: line_index,
                max: source.line_count(),
            })?;
        Ok(range)
    }
}
