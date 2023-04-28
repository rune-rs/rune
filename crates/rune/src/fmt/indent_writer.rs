// Author: Tom Solberg <me@sbg.dev>
// Copyright © 2023, Tom Solberg, all rights reserved.
// Created: 28 April 2023

/*!

*/

use std::io::Write;

pub(super) struct IndentedWriter<W>
where
    W: Write,
{
    writer: W,
    indent: usize,
    needs_indent: bool,
}

impl<W> IndentedWriter<W>
where
    W: Write,
{
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            indent: 0,
            needs_indent: true,
        }
    }

    pub(super) fn indent(&mut self) {
        self.indent += 4;
    }

    pub(super) fn dedent(&mut self) {
        self.indent -= 4;
    }

    fn write_indent(&mut self) -> std::io::Result<usize> {
        for _ in 0..self.indent {
            write!(self.writer, " ")?;
        }

        Ok(self.indent)
    }
}

impl<W> Write for IndentedWriter<W>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        // We split the buffer into lines and write each line separately, indenting as we go. If we end on a newline, we set the needs_indent flag to true.
        let mut written = 0;

        if buf[0] == b'\n' {
            self.needs_indent = true;
            written += self.writer.write(&[b'\n']).unwrap();
            return Ok(written);
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
            if self.needs_indent {
                self.write_indent()?;
                self.needs_indent = false;
            }

            if !line.is_empty() {
                written += self.writer.write(line).unwrap();
            }

            if idx < line_count.saturating_sub(1) || ends_with_newline {
                written += self.writer.write(&[b'\n']).unwrap();
                self.needs_indent = true;
            }
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let mut writer = IndentedWriter::new(Vec::new());
        writer.write(b"hello\nworld\n").unwrap();
        assert_eq!(writer.writer, b"hello\nworld\n");
    }

    #[test]
    fn test_roundtrip_with_indent() {
        let mut writer = IndentedWriter::new(Vec::new());
        writer.indent();
        writer.write(b"hello\nworld\n").unwrap();
        assert_eq!(writer.writer, b"    hello\n    world\n");
    }
}
