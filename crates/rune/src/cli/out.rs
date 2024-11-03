use std::fmt;
use std::io::{self, Write};

use crate::termcolor::{self, ColorSpec, StandardStream, WriteColor};

pub(super) enum Stream {
    Stdout,
    Stderr,
}

impl Stream {
    fn find<'a>(
        &self,
        stdout: &'a mut StandardStream,
        stderr: &'a mut StandardStream,
    ) -> &'a mut StandardStream {
        match self {
            Stream::Stdout => stdout,
            Stream::Stderr => stderr,
        }
    }
}

pub(super) enum Color {
    Error,
    Passed,
    Ignore,
    Highlight,
    Important,
}

impl Color {
    fn find<'a>(&self, colors: &'a Colors) -> &'a ColorSpec {
        match self {
            Color::Error => &colors.error,
            Color::Passed => &colors.passed,
            Color::Ignore => &colors.ignored,
            Color::Highlight => &colors.highlight,
            Color::Important => &colors.important,
        }
    }
}

pub(super) struct Io<'io> {
    pub(super) stdout: &'io mut StandardStream,
    pub(super) stderr: &'io mut StandardStream,
    colors: Option<Colors>,
}

impl<'io> Io<'io> {
    pub(super) fn new(stdout: &'io mut StandardStream, stderr: &'io mut StandardStream) -> Self {
        Self {
            stdout,
            stderr,
            colors: None,
        }
    }

    pub(super) fn with_color(
        &mut self,
        stream: Stream,
        color: Color,
    ) -> io::Result<&mut ColorStream> {
        let stream = stream.find(self.stdout, self.stderr);
        let colors = self.colors.get_or_insert_with(Colors::new);
        stream.set_color(color.find(colors))?;
        Ok(ColorStream::new(stream))
    }

    pub(super) fn section(
        &mut self,
        title: impl fmt::Display,
        stream: Stream,
        color: Color,
    ) -> io::Result<Section<'_>> {
        let io = stream.find(self.stdout, self.stderr);
        let colors = self.colors.get_or_insert_with(Colors::new);

        io.set_color(color.find(colors))?;
        write!(io, "{title:>12}")?;
        io.reset()?;

        Ok(Section { io, colors })
    }

    pub(super) fn write(
        &mut self,
        title: impl fmt::Display,
        stream: Stream,
        color: Color,
    ) -> io::Result<()> {
        let stream = stream.find(self.stdout, self.stderr);
        let colors = self.colors.get_or_insert_with(Colors::new);

        stream.set_color(color.find(colors))?;
        write!(stream, "{title}")?;
        stream.reset()?;
        Ok(())
    }
}

#[derive(Default)]
struct Colors {
    error: ColorSpec,
    passed: ColorSpec,
    highlight: ColorSpec,
    important: ColorSpec,
    ignored: ColorSpec,
}

impl Colors {
    fn new() -> Self {
        let mut this = Self::default();
        this.error.set_fg(Some(termcolor::Color::Red));
        this.passed.set_fg(Some(termcolor::Color::Green));
        this.highlight.set_fg(Some(termcolor::Color::Green));
        this.highlight.set_bold(true);
        this.important.set_fg(Some(termcolor::Color::White));
        this.important.set_bold(true);
        this.ignored.set_fg(Some(termcolor::Color::Yellow));
        this.ignored.set_bold(true);
        this
    }
}

#[must_use = "Section must be closed"]
pub(super) struct Section<'a> {
    pub(super) io: &'a mut StandardStream,
    colors: &'a Colors,
}

impl Section<'_> {
    pub(super) fn append(&mut self, text: impl fmt::Display) -> io::Result<&mut Self> {
        write!(self.io, "{text}")?;
        Ok(self)
    }

    /// Flush the current section.
    pub(super) fn flush(&mut self) -> io::Result<&mut Self> {
        self.io.flush()?;
        Ok(self)
    }

    pub(super) fn append_with(
        &mut self,
        text: impl fmt::Display,
        color: Color,
    ) -> io::Result<&mut Self> {
        self.io.set_color(color.find(self.colors))?;
        write!(self.io, "{text}")?;
        self.io.reset()?;
        Ok(self)
    }

    pub(super) fn error(&mut self, text: impl fmt::Display) -> io::Result<&mut Self> {
        self.append_with(text, Color::Error)?;
        Ok(self)
    }

    pub(super) fn passed(&mut self, text: impl fmt::Display) -> io::Result<&mut Self> {
        self.append_with(text, Color::Passed)?;
        Ok(self)
    }

    pub(super) fn close(&mut self) -> io::Result<()> {
        writeln!(self.io)?;
        Ok(())
    }
}

#[repr(transparent)]
pub(super) struct ColorStream(StandardStream);

impl ColorStream {
    fn new(io: &mut StandardStream) -> &mut Self {
        unsafe { &mut *(io as *mut StandardStream as *mut Self) }
    }

    pub(super) fn close(&mut self) -> io::Result<()> {
        self.0.reset()
    }
}

impl io::Write for ColorStream {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}
