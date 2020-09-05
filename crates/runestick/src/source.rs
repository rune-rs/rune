use crate::Span;

/// A single source file.
#[derive(Debug, Clone)]
pub struct Source {
    /// The name of the source.
    name: String,
    /// The source string.
    source: String,
}

impl Source {
    /// Construct a new source with the given name.
    pub fn new<N, S>(name: N, source: S) -> Self
    where
        N: AsRef<str>,
        S: AsRef<str>,
    {
        Self {
            name: name.as_ref().to_owned(),
            source: source.as_ref().to_owned(),
        }
    }

    /// Get the name of the source.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Fetch source for the given span.
    pub fn source(&self, span: Span) -> Option<&'_ str> {
        self.source.get(span.start..span.end)
    }

    /// Get the end of the source.
    pub fn end(&self) -> usize {
        self.source.len()
    }

    /// Access the underlying string for the source.
    pub fn as_str(&self) -> &str {
        &self.source
    }
}
