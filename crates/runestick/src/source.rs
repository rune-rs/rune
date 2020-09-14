use crate::Span;
use std::fs;
use std::io;
use std::path::Path;
use url::Url;

/// A single source file.
#[derive(Default, Debug, Clone)]
pub struct Source {
    /// The name of the source.
    name: String,
    /// The source string.
    source: String,
    /// The (optional) path of a source file.
    url: Option<Url>,
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
            url: None,
            line_starts,
        }
    }

    /// Load a source from a path.
    pub fn from_path(path: &Path) -> io::Result<Self> {
        let name = path.display().to_string();
        let path = &path.canonicalize()?;

        let url = url::Url::from_file_path(path).map_err(|_| {
            io::Error::new(io::ErrorKind::Other, "path could not be converted to url")
        })?;

        let source = fs::read_to_string(path)?;
        let line_starts = line_starts(&source).collect::<Vec<_>>();

        Ok(Self {
            name,
            source,
            url: Some(url),
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
    pub fn url(&self) -> Option<&Url> {
        self.url.as_ref()
    }

    /// Get a mutable path.
    pub fn url_mut(&mut self) -> &mut Option<Url> {
        &mut self.url
    }

    /// Convert the given offset to a utf-16 line and character.
    pub fn position_to_utf16cu_line_char(&self, offset: usize) -> Option<(usize, usize)> {
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

        None
    }
}

fn line_starts(source: &str) -> impl Iterator<Item = usize> + '_ {
    std::iter::once(0).chain(source.match_indices('\n').map(|(i, _)| i + 1))
}
