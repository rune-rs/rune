use crate::Span;
use std::fmt;
use std::fs;
use std::io;
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
    line_starts: Vec<usize>,
}

impl Source {
    /// Construct a new source with the given name.
    pub fn new(name: impl AsRef<str>, source: impl AsRef<str>) -> Self {
        Self::with_path(name, source, None::<Box<Path>>)
    }

    /// Construct a new source with the given name.
    pub fn with_path(
        name: impl AsRef<str>,
        source: impl AsRef<str>,
        path: Option<impl AsRef<Path>>,
    ) -> Self {
        let source = source.as_ref();
        let line_starts = line_starts(source).collect::<Vec<_>>();

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

    /// Load a source from a path.
    pub fn from_path(path: &Path) -> io::Result<Self> {
        let name = path.display().to_string();
        let path = path.canonicalize()?;

        let source = fs::read_to_string(&path)?;
        let line_starts = line_starts(&source).collect::<Vec<_>>();

        Ok(Self {
            name: name.into(),
            source: source.into(),
            path: Some(path.into()),
            line_starts,
        })
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

    /// Fetch source for the given span.
    pub fn source(&self, span: Span) -> Option<&'_ str> {
        self.get(span.range())
    }

    ///  et the given range from the source.
    pub fn get<I>(&self, i: I) -> Option<&I::Output>
    where
        I: slice::SliceIndex<str>,
    {
        self.source.get(i)
    }

    /// Get the end of the source.
    pub fn end(&self) -> usize {
        self.source.len()
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
    pub fn position_to_utf16cu_line_char(&self, offset: usize) -> Option<(usize, usize)> {
        if offset == 0 {
            return Some((0, 0));
        }

        let line = match self.line_starts.binary_search(&offset) {
            Ok(exact) => exact,
            Err(0) => return None,
            Err(n) => n - 1,
        };

        let line_start = self.line_starts[line];

        let rest = &self.source[line_start..];
        let offset = offset - line_start;
        let mut line_count = 0;

        for (n, c) in rest.char_indices() {
            if n == offset {
                return Some((line, line_count));
            }

            if n > offset {
                break;
            }

            line_count += c.encode_utf16(&mut [0u16; 2]).len();
        }

        Some((line, line_count))
    }

    /// Convert the given offset to a utf-16 line and character.
    pub fn position_to_unicode_line_char(&self, offset: usize) -> (usize, usize) {
        if offset == 0 {
            return (0, 0);
        }

        let line = match self.line_starts.binary_search(&offset) {
            Ok(exact) => exact,
            Err(0) => return (0, 0),
            Err(n) => n - 1,
        };

        let line_start = self.line_starts[line];

        let rest = &self.source[line_start..];
        let offset = offset - line_start;
        let mut line_count = 0;

        for (n, _) in rest.char_indices() {
            if n == offset {
                return (line, line_count);
            }

            if n > offset {
                break;
            }

            line_count += 1;
        }

        (line, line_count)
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
