use runestick::{Item, Source};
use std::collections::VecDeque;
use std::sync::Arc;

/// A collection of source files, and a queue of things to compile.
#[derive(Debug, Default)]
pub struct Sources {
    sources: Vec<Arc<Source>>,
    queue: VecDeque<(Item, usize)>,
}

impl Sources {
    /// Construct a new collection of sources.
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
            queue: VecDeque::new(),
        }
    }

    /// Get the source at the given source id.
    pub fn source_at(&self, source_id: usize) -> Option<&Arc<Source>> {
        self.sources.get(source_id)
    }

    /// Insert a new source and return its associated id.
    pub fn insert(&mut self, item: Item, source: Source) -> usize {
        let source_id = self.sources.len();
        self.queue.push_back((item, source_id));
        self.sources.push(Arc::new(source));
        source_id
    }

    /// Insert a new source and return its associated id.
    pub fn insert_default(&mut self, source: Source) -> usize {
        self.insert(Item::default(), source)
    }

    /// Get the source matching the given source id.
    pub fn get(&self, source_id: usize) -> Option<&Arc<Source>> {
        self.sources.get(source_id)
    }

    /// Get the next source in the queue to compile.
    pub(crate) fn next_source(&mut self) -> Option<(Item, usize)> {
        self.queue.pop_front()
    }

    /// Iterate over all sources in order by index.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Source> {
        self.sources.iter().map(|s| &**s)
    }
}
