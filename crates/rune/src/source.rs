use crate::ast::Span;
use std::fmt;
use std::fs;
use std::io;
use std::ops::Range;
use std::path::Path;
use std::slice;

/// A single source file.
#[derive(Default, Clone)]
pub struct Source {
    /// The name of the source.
    name: Box<str>,
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
        Self::with_path(name, source, None::<Box<Path>>)
    }

    /// Read and load a source from the given path.
    pub fn from_path(path: &Path) -> io::Result<Self> {
        let name = path.display().to_string();
        let source = fs::read_to_string(path)?;
        Ok(Self::with_path(name, source, Some(path)))
    }

    /// Construct a new source with the given name.
    pub fn with_path(
        name: impl AsRef<str>,
        source: impl AsRef<str>,
        path: Option<impl AsRef<Path>>,
    ) -> Self {
        let source = source.as_ref();
        let line_starts = line_starts(source).collect::<Box<[_]>>();

        Self {
            name: name.as_ref().into(),
            source: source.into(),
            path: path.map(|p| p.as_ref().into()),
            line_starts,
        }
    }

    /// Access all line starts in the source.
    pub fn line_starts(&self) -> &[usize] {
        &self.line_starts
    }

    /// Test if the source is empty.
    pub fn is_empty(&self) -> bool {
        self.source.is_empty()
    }

    /// Get the length of the source.
    pub fn len(&self) -> usize {
        self.source.len()
    }

    /// Get the name of the source.
    pub fn name(&self) -> &str {
        &self.name
    }

    ///  et the given range from the source.
    pub fn get<I>(&self, i: I) -> Option<&I::Output>
    where
        I: slice::SliceIndex<str>,
    {
        self.source.get(i)
    }

    /// Access the underlying string for the source.
    pub fn as_str(&self) -> &str {
        &self.source
    }

    /// Get the (optional) path of the source.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Convert the given offset to a utf-16 line and character.
    pub fn pos_to_utf16cu_linecol(&self, offset: usize) -> (usize, usize) {
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
    pub fn line_index(&self, byte_index: usize) -> usize {
        self.line_starts
            .binary_search(&byte_index)
            .unwrap_or_else(|next_line| next_line.saturating_sub(1))
    }

    /// Get the range corresponding to the given line index.
    pub fn line_range(&self, line_index: usize) -> Option<Range<usize>> {
        let line_start = self.line_start(line_index)?;
        let next_line_start = self.line_start(line_index.saturating_add(1))?;
        Some(line_start..next_line_start)
    }

    /// Get the number of lines in the source.
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Access the line number of content that starts with the given span.
    pub fn line(&self, span: Span) -> Option<(usize, usize, &str)> {
        let start = span.start.into_usize();
        let (line, col) = self.pos_to_utf8_linecol(start);
        let range = self.line_range(line)?;
        let text = self.source.get(range)?;
        Some((line, col, text))
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

    fn line_start(&self, line_index: usize) -> Option<usize> {
        use std::cmp::Ordering;

        match line_index.cmp(&self.line_starts.len()) {
            Ordering::Less => self.line_starts.get(line_index).copied(),
            Ordering::Equal => Some(self.source.as_ref().len()),
            Ordering::Greater => None,
        }
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

fn line_starts(source: &str) -> impl Iterator<Item = usize> + '_ {
    std::iter::once(0).chain(source.match_indices('\n').map(|(i, _)| i + 1))
}
