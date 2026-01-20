//! Module for dealing with sources.
//!
//! The primary type in here is the [`Source`] struct, which holds onto all
//! metadata necessary related to a source in order to build it.
//!
//! Sources are stored in the [`Sources`] collection.
//!
//! [`Sources`]: crate::sources::Sources

#[cfg(feature = "emit")]
use core::cmp;
use core::fmt;
use core::iter;
#[cfg(feature = "emit")]
use core::ops::Range;
use core::slice;

#[cfg(feature = "emit")]
use std::io;
#[cfg(feature = "std")]
use std::path::Path;

use crate as rune;
#[cfg(feature = "std")]
use crate::alloc::borrow::Cow;
use crate::alloc::prelude::*;
use crate::alloc::{self, Box};
use crate::ast::Span;
#[cfg(feature = "emit")]
use crate::termcolor::{self, WriteColor};

/// Error raised when constructing a source.
#[derive(Debug)]
pub struct FromPathError {
    kind: FromPathErrorKind,
}

impl From<alloc::Error> for FromPathError {
    fn from(error: alloc::Error) -> Self {
        Self {
            kind: FromPathErrorKind::Alloc(error),
        }
    }
}

#[cfg(feature = "std")]
impl From<std::io::Error> for FromPathError {
    fn from(error: std::io::Error) -> Self {
        Self {
            kind: FromPathErrorKind::Io(error),
        }
    }
}

impl fmt::Display for FromPathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            FromPathErrorKind::Alloc(error) => error.fmt(f),
            #[cfg(feature = "std")]
            FromPathErrorKind::Io(error) => error.fmt(f),
        }
    }
}

#[derive(Debug)]
enum FromPathErrorKind {
    Alloc(alloc::Error),
    #[cfg(feature = "std")]
    Io(std::io::Error),
}

impl core::error::Error for FromPathError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match &self.kind {
            FromPathErrorKind::Alloc(error) => Some(error),
            #[cfg(feature = "std")]
            FromPathErrorKind::Io(error) => Some(error),
        }
    }
}

/// A single source file.
#[derive(Default, TryClone)]
pub struct Source {
    /// The name of the source.
    name: SourceName,
    /// The source string.
    source: Box<str>,
    /// The path the source was loaded from.
    #[cfg(feature = "std")]
    path: Option<Box<Path>>,
    /// The starting byte indices in the source code.
    line_starts: Box<[usize]>,
}

impl Source {
    /// Construct a new source with the given name.
    pub fn new(name: impl AsRef<str>, source: impl AsRef<str>) -> alloc::Result<Self> {
        let name = Box::try_from(name.as_ref())?;
        let source = source.as_ref();
        let line_starts = line_starts(source).try_collect::<Box<[_]>>()?;

        Ok(Self {
            name: SourceName::Name(name),
            source: source.try_into()?,
            #[cfg(feature = "std")]
            path: None,
            line_starts,
        })
    }

    /// Construct a new anonymously named `<memory>` source.
    ///
    /// # Examples
    ///
    /// ```
    /// use rune::Source;
    ///
    /// let source = Source::memory("pub fn main() { 42 }")?;
    /// assert_eq!(source.name(), "<memory>");
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    pub fn memory(source: impl AsRef<str>) -> alloc::Result<Self> {
        let source = source.as_ref();
        let line_starts = line_starts(source).try_collect::<Box<[_]>>()?;

        Ok(Self {
            name: SourceName::Memory,
            source: source.try_into()?,
            #[cfg(feature = "std")]
            path: None,
            line_starts,
        })
    }

    /// Read and load a source from the given filesystem path.
    #[cfg(feature = "std")]
    #[cfg_attr(rune_docsrs, doc(cfg(feature = "std")))]
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, FromPathError> {
        let name = Box::try_from(Cow::try_from(path.as_ref().to_string_lossy())?)?;
        let source = Box::try_from(std::fs::read_to_string(path.as_ref())?)?;
        let path = Some(path.as_ref().try_into()?);
        let line_starts = line_starts(source.as_ref()).try_collect::<Box<[_]>>()?;

        Ok(Self {
            name: SourceName::Name(name),
            source,
            path,
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
    /// let source = Source::with_path("test", "pub fn main() { 42 }", "test.rn")?;
    /// assert_eq!(source.name(), "test");
    /// assert_eq!(source.path(), Some(Path::new("test.rn")));
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[cfg(feature = "std")]
    #[cfg_attr(rune_docsrs, doc(cfg(feature = "std")))]
    pub fn with_path(
        name: impl AsRef<str>,
        source: impl AsRef<str>,
        path: impl AsRef<Path>,
    ) -> alloc::Result<Self> {
        let name = Box::try_from(name.as_ref())?;
        let source = Box::try_from(source.as_ref())?;
        let path = Some(path.as_ref().try_into()?);
        let line_starts = line_starts(source.as_ref()).try_collect::<Box<[_]>>()?;

        Ok(Self {
            name: SourceName::Name(name),
            source,
            path,
            line_starts,
        })
    }

    /// Access all line starts in the source.
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
    /// let source = Source::with_path("test", "pub fn main() { 42 }", "test.rn")?;
    /// assert_eq!(source.name(), "test");
    /// assert_eq!(source.path(), Some(Path::new("test.rn")));
    /// # Ok::<_, rune::support::Error>(())
    /// ```
    #[cfg(feature = "std")]
    #[cfg_attr(rune_docsrs, doc(cfg(feature = "std")))]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Convert the given position to a utf-8 line position in code units.
    ///
    /// A position is a character offset into the source in utf-8 characters.
    ///
    /// Note that utf-8 code units is what you'd count when using the
    /// [`str::chars()`] iterator.
    pub fn find_line_column(&self, position: usize) -> (usize, usize) {
        let (line, offset, rest) = self.position(position);
        let col = rest.char_indices().take_while(|&(n, _)| n < offset).count();
        (line, col)
    }

    /// Convert the given position to a utf-16 code units line and character.
    ///
    /// A position is a character offset into the source in utf-16 characters.
    ///
    /// Note that utf-16 code units is what you'd count when iterating over the
    /// string in terms of characters as-if they would have been encoded with
    /// [`char::encode_utf16()`].
    pub fn find_utf16cu_line_column(&self, position: usize) -> (usize, usize) {
        let (line, offset, rest) = self.position(position);

        let col = rest
            .char_indices()
            .flat_map(|(n, c)| (n < offset).then(|| c.encode_utf16(&mut [0u16; 2]).len()))
            .sum();

        (line, col)
    }

    /// Fetch [`SourceLine`] information for the given span.
    pub fn source_line(&self, span: Span) -> Option<SourceLine<'_>> {
        let (line, column, text, _span) = line_for(self, span)?;

        Some(SourceLine {
            #[cfg(feature = "emit")]
            name: self.name(),
            line,
            column,
            text,
            #[cfg(feature = "emit")]
            span: _span,
        })
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
        let (lin, col) = self.find_line_column(from.start);
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
        let mut st = f.debug_struct("Source");
        st.field("name", &self.name);
        #[cfg(feature = "std")]
        st.field("path", &self.path);
        st.finish()
    }
}

/// An extracted source line.
pub struct SourceLine<'a> {
    #[cfg(feature = "emit")]
    name: &'a str,
    /// The line number in the source.
    pub line: usize,
    /// The column number in the source.
    pub column: usize,
    /// The text of the span.
    pub text: &'a str,
    #[cfg(feature = "emit")]
    span: Span,
}

impl SourceLine<'_> {
    /// Pretty write a source line to the given output.
    #[cfg(feature = "emit")]
    pub(crate) fn write(&self, o: &mut dyn WriteColor) -> io::Result<()> {
        let mut highlight = termcolor::ColorSpec::new();
        highlight.set_fg(Some(termcolor::Color::Yellow));

        let mut new_line = termcolor::ColorSpec::new();
        new_line.set_fg(Some(termcolor::Color::Red));

        let text = self.text.trim_end();
        let end = self.span.end.into_usize().min(text.len());

        let before = &text[0..self.span.start.into_usize()].trim_start();
        let inner = &text[self.span.start.into_usize()..end];
        let after = &text[end..];

        {
            let name = self.name;
            let line = self.line + 1;
            let start = self.column + 1;
            let end = start + inner.chars().count();
            write!(o, "{name}:{line}:{start}-{end}: ")?;
        }

        write!(o, "{before}")?;
        o.set_color(&highlight)?;
        write!(o, "{inner}")?;
        o.reset()?;
        write!(o, "{after}")?;

        if self.span.end != end {
            o.set_color(&new_line)?;
            write!(o, "\\n")?;
            o.reset()?;
        }

        Ok(())
    }
}

/// Holder for the name of a source.
#[derive(Default, Debug, TryClone, PartialEq, Eq)]
enum SourceName {
    /// An in-memory source, will use `<memory>` when the source is being
    /// referred to in diagnostics.
    #[default]
    Memory,
    /// A named source.
    Name(Box<str>),
}

#[inline(always)]
fn line_starts(source: &str) -> impl Iterator<Item = usize> + '_ {
    iter::once(0).chain(source.match_indices('\n').map(|(i, _)| i + 1))
}

/// Get the line number and source line for the given source and span.
fn line_for(source: &Source, span: Span) -> Option<(usize, usize, &str, Span)> {
    let line_starts = source.line_starts();

    let line = match line_starts.binary_search(&span.start.into_usize()) {
        Ok(n) => n,
        Err(n) => n.saturating_sub(1),
    };

    let start = *line_starts.get(line)?;
    let end = line.checked_add(1)?;

    let s = if let Some(end) = line_starts.get(end) {
        source.get(start..*end)?
    } else {
        source.get(start..)?
    };

    let line_end = span.start.into_usize().saturating_sub(start);

    let column = s
        .get(..line_end)
        .into_iter()
        .flat_map(|s| s.chars())
        .count();

    let start = start.try_into().unwrap();

    Some((
        line,
        column,
        s,
        Span::new(
            span.start.saturating_sub(start),
            span.end.saturating_sub(start),
        ),
    ))
}
