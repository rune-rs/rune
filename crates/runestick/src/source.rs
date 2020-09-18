use crate::Span;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// A single source file.
#[derive(Default, Debug, Clone)]
pub struct Source {
    /// The name of the source.
    name: String,
    /// The source string.
    source: String,
    /// The path the source was loaded from.
    path: Option<PathBuf>,
    /// The starting byte indices in the source code.
    line_starts: Vec<usize>,
}

impl Source {
    /// Construct a new source with the given name.
    pub fn new<N, S>(name: N, source: S) -> Self
    where
        N: AsRef<str>,
        S: AsRef<str>,
    {
        let source = source.as_ref();
        let line_starts = line_starts(source).collect::<Vec<_>>();

        Self {
            name: name.as_ref().to_owned(),
            source: source.to_owned(),
            path: None,
            line_starts,
        }
    }

    /// Load a source from a path.
    pub fn from_path(path: &Path) -> io::Result<Self> {
        let name = path.display().to_string();
        let path = &path.canonicalize()?;

        let source = fs::read_to_string(path)?;
        let line_starts = line_starts(&source).collect::<Vec<_>>();

        Ok(Self {
            name,
            source,
            path: Some(path.to_owned()),
            line_starts,
        })
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

    /// Get the (optional) path of the source.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Get a mutable path.
    pub fn path_mut(&mut self) -> &mut Option<PathBuf> {
        &mut self.path
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

fn line_starts(source: &str) -> impl Iterator<Item = usize> + '_ {
    std::iter::once(0).chain(source.match_indices('\n').map(|(i, _)| i + 1))
}
