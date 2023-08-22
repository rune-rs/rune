#[cfg(feature = "emit")]
use core::cmp;
use core::fmt;
use core::iter;
#[cfg(feature = "emit")]
use core::ops::Range;
use core::slice;

use crate::no_std::io;
use crate::no_std::path::Path;
use crate::no_std::prelude::*;

#[cfg(feature = "emit")]
use crate::ast::Span;

/// A single source file.
#[derive(Default, Clone)]
pub struct Source {
    /// The name of the source.
    name: SourceName,
    /// The source string.
    source: Box<str>,
    /// The path the source was loaded from.
    path: Option<Box<Path>>,
    /// The starting byte indices in the source code.
    line_starts: Box<[usize]>,
}

impl Source {
    /// Construct a new source with the given name.
    pub fn new(name: impl AsRef<str>, source: impl AsRef<str>) -> Self {
        let source = source.as_ref();
        let line_starts = line_starts(source).collect::<Box<[_]>>();

        Self {
            name: SourceName::Name(name.as_ref().into()),
            source: source.into(),
            path: None,
            line_starts,
        }
    }

    /// Construct a new anonymously named `<memory>` source.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Source;
    /// let source = Source::memory("pub fn main() { 42 }");
    /// assert_eq!(source.name(), "<memory>");
    /// ```
    pub fn memory(source: impl AsRef<str>) -> Self {
        let source = source.as_ref();
        let line_starts = line_starts(source).collect::<Box<[_]>>();

        Self {
            name: SourceName::Memory,
            source: source.into(),
            path: None,
            line_starts,
        }
    }

    /// Constructing sources from paths is not supported in no-std environments.
    #[cfg(not(feature = "std"))]
    pub fn from_path<P>(_: P) -> io::Result<Self> {
        Err(io::Error::new())
    }

    /// Read and load a source from the given filesystem path.
    #[cfg(feature = "std")]
    pub fn from_path(path: impl AsRef<Path>) -> io::Result<Self> {
        let source = std::fs::read_to_string(path.as_ref())?;
        let line_starts = line_starts(&source).collect::<Box<[_]>>();

        Ok(Self {
            name: SourceName::Name(path.as_ref().to_string_lossy().into_owned().into()),
            source: source.into(),
            path: Some(path.as_ref().into()),
            line_starts,
        })
    }

    /// Construct a new source with the given content and path.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    /// use rune::Source;
    ///
    /// let source = Source::with_path("test", "pub fn main() { 42 }", "test.rn");
    /// assert_eq!(source.name(), "test");
    /// assert_eq!(source.path(), Some(Path::new("test.rn")));
    /// ```
    pub fn with_path(
        name: impl AsRef<str>,
        source: impl AsRef<str>,
        path: impl AsRef<Path>,
    ) -> Self {
        let source = source.as_ref();
        let line_starts = line_starts(source).collect::<Box<[_]>>();

        Self {
            name: SourceName::Name(name.as_ref().into()),
            source: source.into(),
            path: Some(path.as_ref().into()),
            line_starts,
        }
    }

    /// Access all line starts in the source.
    #[cfg(feature = "emit")]
    pub(crate) fn line_starts(&self) -> &[usize] {
        &self.line_starts
    }

    /// Get the name of the source.
    pub fn name(&self) -> &str {
        match &self.name {
            SourceName::Memory => "<memory>",
            SourceName::Name(name) => name,
        }
    }

    ///  et the given range from the source.
    pub(crate) fn get<I>(&self, i: I) -> Option<&I::Output>
    where
        I: slice::SliceIndex<str>,
    {
        self.source.get(i)
    }

    /// Access the underlying string for the source.
    pub(crate) fn as_str(&self) -> &str {
        &self.source
    }

    /// Get the path associated with the source.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    /// use rune::Source;
    ///
    /// let source = Source::with_path("test", "pub fn main() { 42 }", "test.rn");
    /// assert_eq!(source.name(), "test");
    /// assert_eq!(source.path(), Some(Path::new("test.rn")));
    /// ```
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Convert the given offset to a utf-16 line and character.
    pub(crate) fn pos_to_utf16cu_linecol(&self, offset: usize) -> (usize, usize) {
        let (line, offset, rest) = self.position(offset);
        let col = rest
            .char_indices()
            .flat_map(|(n, c)| (n < offset).then(|| c.encode_utf16(&mut [0u16; 2]).len()))
            .sum();
        (line, col)
    }

    /// Convert the given offset to a utf-16 line and character.
    pub fn pos_to_utf8_linecol(&self, offset: usize) -> (usize, usize) {
        let (line, offset, rest) = self.position(offset);
        let col = rest.char_indices().take_while(|&(n, _)| n < offset).count();
        (line, col)
    }

    /// Get the line index for the given byte.
    #[cfg(feature = "emit")]
    pub(crate) fn line_index(&self, byte_index: usize) -> usize {
        self.line_starts
            .binary_search(&byte_index)
            .unwrap_or_else(|next_line| next_line.saturating_sub(1))
    }

    /// Get the range corresponding to the given line index.
    #[cfg(feature = "emit")]
    pub(crate) fn line_range(&self, line_index: usize) -> Option<Range<usize>> {
        let line_start = self.line_start(line_index)?;
        let next_line_start = self.line_start(line_index.saturating_add(1))?;
        Some(line_start..next_line_start)
    }

    /// Get the number of lines in the source.
    #[cfg(feature = "emit")]
    pub(crate) fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Access the line number of content that starts with the given span.
    #[cfg(feature = "emit")]
    pub(crate) fn line(&self, span: Span) -> Option<(usize, usize, [&str; 3])> {
        let from = span.range();
        let (lin, col) = self.pos_to_utf8_linecol(from.start);
        let line = self.line_range(lin)?;

        let start = from.start.checked_sub(line.start)?;
        let end = from.end.checked_sub(line.start)?;

        let text = self.source.get(line)?;
        let prefix = text.get(..start)?;
        let mid = text.get(start..end)?;
        let suffix = text.get(end..)?;

        Some((lin, col, [prefix, mid, suffix]))
    }

    fn position(&self, offset: usize) -> (usize, usize, &str) {
        if offset == 0 {
            return Default::default();
        }

        let line = match self.line_starts.binary_search(&offset) {
            Ok(exact) => exact,
            Err(0) => return Default::default(),
            Err(n) => n - 1,
        };

        let line_start = self.line_starts[line];

        let rest = &self.source[line_start..];
        let offset = offset.saturating_sub(line_start);
        (line, offset, rest)
    }

    #[cfg(feature = "emit")]
    fn line_start(&self, line_index: usize) -> Option<usize> {
        match line_index.cmp(&self.line_starts.len()) {
            cmp::Ordering::Less => self.line_starts.get(line_index).copied(),
            cmp::Ordering::Equal => Some(self.source.as_ref().len()),
            cmp::Ordering::Greater => None,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.source.len()
    }
}

impl fmt::Debug for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Source")
            .field("name", &self.name)
            .field("path", &self.path)
            .finish()
    }
}

/// Holder for the name of a source.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
enum SourceName {
    /// An in-memory source, will use `<memory>` when the source is being
    /// referred to in diagnostics.
    #[default]
    Memory,
    /// A named source.
    Name(Box<str>),
}

fn line_starts(source: &str) -> impl Iterator<Item = usize> + '_ {
    iter::once(0).chain(source.match_indices('\n').map(|(i, _)| i + 1))
}
