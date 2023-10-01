//! Specialized writter/string builders for the formatting module.

#[cfg(test)]
mod tests;

use core::ops::{Deref, DerefMut};
use core::str;

use crate::alloc::fmt::TryWrite;
use crate::alloc::prelude::*;
use crate::alloc::{self, try_vec, Vec};
use crate::ast::Span;

use super::comments::Comment;
use super::error::FormattingError;
use super::whitespace::EmptyLine;

pub(super) struct IndentedWriter {
    lines: Vec<Vec<u8>>,
    indent: usize,
    needs_indent: bool,
}

impl IndentedWriter {
    pub(super) fn new() -> alloc::Result<Self> {
        Ok(Self {
            lines: try_vec![Vec::new()],
            indent: 0,
            needs_indent: true,
        })
    }

    pub(super) fn into_inner(self) -> Vec<Vec<u8>> {
        self.lines
    }

    pub(super) fn indent(&mut self) {
        self.indent += 4;
    }

    pub(super) fn dedent(&mut self) {
        self.indent = self.indent.saturating_sub(4);
    }

    fn write_indent(&mut self) -> alloc::Result<usize> {
        for _ in 0..self.indent {
            if let Some(line) = self.lines.last_mut() {
                line.try_push(b' ')?;
            }
        }

        Ok(self.indent)
    }
}

impl TryWrite for IndentedWriter {
    fn try_write_str(&mut self, buf: &str) -> alloc::Result<()> {
        if buf.starts_with('\n') {
            self.needs_indent = true;
            self.lines.try_push(Vec::new())?;
            return Ok(());
        }

        let ends_with_newline = buf.ends_with('\n');
        let line_count = buf.chars().filter(|&b| b == '\n').count();
        let lines = buf.split(|b| b == '\n');

        let lines_to_write = if ends_with_newline {
            line_count
        } else {
            line_count + 1
        };

        for (idx, line) in lines.enumerate().take(lines_to_write) {
            if self.needs_indent {
                self.write_indent()?;
                self.needs_indent = false;
            }

            if !line.is_empty() {
                if let Some(last_line) = self.lines.last_mut() {
                    last_line.try_extend_from_slice(line.as_bytes())?;
                }
            }

            if idx < line_count.saturating_sub(1) || ends_with_newline {
                self.lines.try_push(Vec::new())?;
                self.needs_indent = true;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
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
    source: &'a str,
}

impl<'a> SpanInjectionWriter<'a> {
    pub(super) fn new(writer: IndentedWriter, source: &'a str) -> Result<Self, FormattingError> {
        let comment_spans = super::comments::parse_comments(source)?;
        let empty_line_spans = super::whitespace::gather_empty_line_spans(source)?;

        let mut queued_spans = Vec::new();
        queued_spans.try_extend(comment_spans.into_iter().map(ResolvedSpan::Comment))?;
        queued_spans.try_extend(empty_line_spans.into_iter().map(ResolvedSpan::Empty))?;

        queued_spans.sort_by_key(|span| span.span().start);

        Ok(Self {
            writer,
            queued_spans,
            source,
        })
    }

    pub(super) fn into_inner(mut self) -> Result<Vec<Vec<u8>>, FormattingError> {
        while !self.queued_spans.is_empty() {
            let span = self.queued_spans.remove(0);
            match span {
                ResolvedSpan::Empty(_) => {
                    writeln!(self.writer)?;
                }
                ResolvedSpan::Comment(comment) => {
                    if comment.on_new_line {
                        writeln!(self.writer, "{}", self.resolve(comment.span)?)?;
                    } else {
                        self.extend_previous_line(b" ")?;
                        self.extend_previous_line(self.resolve(comment.span)?.as_bytes())?;
                    }
                }
            }
        }

        Ok(self.writer.into_inner())
    }

    fn extend_previous_line(&mut self, text: &[u8]) -> alloc::Result<()> {
        let Some(idx) = self.writer.lines.len().checked_sub(2) else {
            // TODO: bubble up an internal error?
            return Ok(());
        };

        let Some(last_line) = self.writer.lines.get_mut(idx) else {
            // TODO: bubble up an internal error?
            return Ok(());
        };

        last_line.try_extend_from_slice(text)?;
        Ok(())
    }

    fn resolve(&self, span: Span) -> Result<&'a str, FormattingError> {
        let Some(s) = self.source.get(span.range()) else {
            return Err(FormattingError::BadRange(span.range(), self.source.len()));
        };

        Ok(s)
    }

    pub(super) fn write_spanned_raw(
        &mut self,
        span: Span,
        newline: bool,
        space: bool,
    ) -> Result<(), FormattingError> {
        let contents = self.resolve(span)?;
        self.write_spanned(span, contents.trim(), newline, space)
    }

    pub(super) fn newline(&mut self) -> Result<(), FormattingError> {
        self.write_unspanned("\n")
    }

    pub(super) fn write_unspanned(&mut self, text: &str) -> Result<(), FormattingError> {
        self.write_spanned(Span::new(0, 0), text, false, false)
    }

    pub(super) fn write_spanned(
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
                        self.extend_previous_line(b" ")?;
                        self.extend_previous_line(self.resolve(comment.span)?.as_bytes())?;
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

impl Deref for SpanInjectionWriter<'_> {
    type Target = IndentedWriter;

    fn deref(&self) -> &Self::Target {
        &self.writer
    }
}

impl DerefMut for SpanInjectionWriter<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.writer
    }
}
