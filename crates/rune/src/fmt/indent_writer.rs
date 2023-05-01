// Author: Tom Solberg <me@sbg.dev>
// Copyright © 2023, Tom Solberg, all rights reserved.
// Created: 28 April 2023

/*!

*/

use std::{
    io::Write,
    ops::{Deref, DerefMut},
};

use crate::{ast::Span, Source};

use super::{comments::Comment, error::FormattingError, whitespace::EmptyLine};

pub(super) struct IndentedWriter {
    lines: Vec<String>,
    indent: usize,
    needs_indent: bool,
}

impl IndentedWriter {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            indent: 0,
            needs_indent: true,
        }
    }

    pub fn into_inner(self) -> Vec<String> {
        self.lines
    }

    pub(super) fn indent(&mut self) {
        self.indent += 4;
    }

    pub(super) fn dedent(&mut self) {
        self.indent -= 4;
    }

    fn write_indent(&mut self) -> std::io::Result<usize> {
        for _ in 0..self.indent {
            self.lines.last_mut().unwrap().push(' ');
        }

        Ok(self.indent)
    }
}

impl Write for IndentedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf[0] == b'\n' {
            self.needs_indent = true;
            self.lines.push(String::new());
            return Ok(buf.len());
        }
        let ends_with_newline = buf.last() == Some(&b'\n');
        let line_count = buf.iter().filter(|&&b| b == b'\n').count();
        let lines = buf.split(|&b| b == b'\n');
        let lines_to_write = if ends_with_newline {
            line_count
        } else {
            line_count + 1
        };
        for (idx, line) in lines.enumerate().take(lines_to_write) {
            let line = std::str::from_utf8(line).unwrap();
            if self.needs_indent {
                self.write_indent()?;
                self.needs_indent = false;
            }

            if !line.is_empty() {
                self.lines.last_mut().unwrap().push_str(line);
            }

            if idx < line_count.saturating_sub(1) || ends_with_newline {
                self.lines.push(String::new());
                self.needs_indent = true;
            }
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

enum ResolvedSpan {
    Empty(EmptyLine),
    Comment(Comment),
}

impl ResolvedSpan {
    fn span(&self) -> Span {
        match self {
            Self::Empty(empty_line) => empty_line.span,
            Self::Comment(comment) => comment.span,
        }
    }
}

/// Writes a span to the writer, injecting comments and empty lines from the source file.
pub(super) struct SpanInjectionWriter<'a> {
    writer: IndentedWriter,
    queued_spans: Vec<ResolvedSpan>,
    source: &'a Source,
}

impl<'a> SpanInjectionWriter<'a> {
    pub fn new(writer: IndentedWriter, source: &'a Source) -> Result<Self, FormattingError> {
        let comment_spans = super::comments::parse_comments(source.as_str())?;
        let empty_line_spans = super::whitespace::gather_empty_line_spans(source.as_str())?;

        let mut queued_spans = Vec::new();
        queued_spans.extend(comment_spans.into_iter().map(ResolvedSpan::Comment));
        queued_spans.extend(empty_line_spans.into_iter().map(ResolvedSpan::Empty));

        queued_spans.sort_by_key(|span| span.span().start);
        Ok(Self {
            writer,
            queued_spans,
            source,
        })
    }

    pub fn into_inner(self) -> Vec<String> {
        self.writer.into_inner()
    }

    fn extend_previous_line(&mut self, text: &str) {
        let idx = self.writer.lines.len() - 2;
        let last_line = self.writer.lines.get_mut(idx).unwrap();
        last_line.push_str(text);
    }

    fn resolve(&self, span: Span) -> Result<String, FormattingError> {
        match self.source.get(span.range()) {
            Some(s) => Ok(s.to_owned()),
            None => Err(FormattingError::InvalidSpan(
                span.start.into_usize(),
                span.end.into_usize(),
                self.source.len(),
            )),
        }
    }

    pub fn write_spanned_raw(
        &mut self,
        span: Span,
        newline: bool,
        space: bool,
    ) -> Result<(), FormattingError> {
        let contents = self.resolve(span)?;
        self.write_spanned(span, contents.trim(), newline, space)
    }

    pub fn newline(&mut self) -> Result<(), FormattingError> {
        self.write_unspanned("\n")
    }

    pub fn write_unspanned(&mut self, text: &str) -> Result<(), FormattingError> {
        self.write_spanned(Span::new(0, 0), text, false, false)
    }

    pub fn write_spanned(
        &mut self,
        span: Span,
        text: &str,
        newline: bool,
        space: bool,
    ) -> Result<(), FormattingError> {
        // The queued recovered spans are ordered so we can pop them from the front if they're before the current span.
        // If the current span is before the first queued span, we need to inject the queued span.

        while let Some(queued_span) = self.queued_spans.first() {
            if queued_span.span().start > span.start {
                break;
            }

            let queued_span = self.queued_spans.remove(0);
            match queued_span {
                ResolvedSpan::Empty(_) => {
                    writeln!(self.writer)?;
                }
                ResolvedSpan::Comment(comment) => {
                    if comment.on_new_line {
                        writeln!(self.writer, "{}", self.resolve(comment.span)?)?;
                    } else {
                        self.extend_previous_line(" ");
                        self.extend_previous_line(&self.resolve(comment.span)?);
                    }
                }
            }
        }

        write!(self.writer, "{}", text)?;

        if space {
            write!(self.writer, " ")?;
        }

        if newline {
            writeln!(self.writer)?;
        }

        Ok(())
    }
}

impl<'a> Deref for SpanInjectionWriter<'_> {
    type Target = IndentedWriter;

    fn deref(&self) -> &Self::Target {
        &self.writer
    }
}

impl<'a> DerefMut for SpanInjectionWriter<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.writer
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let mut writer = IndentedWriter::new();
        writer.write(b"hello\nworld\n").unwrap();
        assert_eq!(writer.into_inner(), vec!["hello", "world", ""]);
    }

    #[test]
    fn test_roundtrip_with_indent() {
        let mut writer = IndentedWriter::new();
        writer.indent();
        writer.write(b"hello\nworld\n").unwrap();
        assert_eq!(writer.into_inner(), vec!["    hello", "    world", ""]);
    }
}
