use crate::SourceId;
use runestick::Source;
use std::sync::Arc;

/// A collection of source files, and a queue of things to compile.
#[derive(Debug, Default)]
pub struct Sources {
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
    pub fn source_at(&self, source_id: usize) -> Option<&Arc<Source>> {
        self.sources.get(source_id)
    }

    /// Insert a source to be built and return its id.
    #[deprecated = "use `insert` instead"]
    pub fn insert_default(&mut self, source: Source) -> usize {
        self.insert(source)
    }

    /// Insert a source to be built and return its id.
    pub fn insert(&mut self, source: Source) -> usize {
        let source_id = self.sources.len();
        self.sources.push(Arc::new(source));
        source_id
    }

    /// Get the source matching the given source id.
    pub fn get(&self, source_id: usize) -> Option<&Arc<Source>> {
        self.sources.get(source_id)
    }

    /// Get all available source ids.
    pub(crate) fn source_ids(&self) -> impl Iterator<Item = SourceId> {
        0..self.sources.len()
    }

    /// Iterate over all sources in order by index.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Source> {
        self.sources.iter().map(|s| &**s)
    }
}
