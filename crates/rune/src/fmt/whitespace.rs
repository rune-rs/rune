#[cfg(test)]
mod tests;

use crate::alloc::Vec;
use crate::ast::Span;
use crate::fmt::FormattingError;

/// A span of an empty line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct EmptyLine {
    pub(super) span: Span,
}

/// Generate a list of all line spans that are empty. A span is the start and end byte index of the line.
pub(super) fn gather_empty_line_spans(source: &str) -> Result<Vec<EmptyLine>, FormattingError> {
    let mut empty_lines = Vec::new();

    let mut line_start = 0;
    let mut line_was_empty = true;

    for (i, c) in source.char_indices() {
        if c == '\n' {
            if line_was_empty {
                empty_lines.try_push(EmptyLine {
                    span: Span::new(line_start, i + 1),
                })?;
            }
            line_start = i + 1;
            line_was_empty = true;
        } else if c.is_whitespace() {
            // Do nothing.
        } else {
            line_was_empty = false;
        }
    }

    Ok(empty_lines)
}
