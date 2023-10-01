//! Extract comments from source code.

#[cfg(test)]
mod tests;

use core::str::CharIndices;

use crate::alloc::Vec;
use crate::ast::Span;
use crate::fmt::FormattingError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum CommentKind {
    Line,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct Comment {
    pub(super) kind: CommentKind,
    pub(super) span: Span,
    pub(super) on_new_line: bool,
}

pub(super) fn parse_comments(input: &str) -> Result<Vec<Comment>, FormattingError> {
    let mut comments = Vec::new();

    let mut chars = input.char_indices();

    let mut in_string = false;
    let mut in_char = false;
    let mut in_template = false;
    let mut on_new_line = true;

    while let Some((start, c)) = chars.next() {
        match c {
            '/' if !in_string && !in_char && !in_template => match chars.clone().next() {
                Some((_, '/')) => {
                    let end = parse_line_comment(&mut chars);

                    if !input[start..end].starts_with("///")
                        && !input[start..end].starts_with("//!")
                    {
                        comments.try_push(Comment {
                            on_new_line,
                            kind: CommentKind::Line,
                            span: Span::new(start, end),
                        })?;
                    }
                }
                Some((_, '*')) => {
                    let end =
                        parse_block_comment(&mut chars).ok_or(FormattingError::OpenComment)?;

                    if !input[start..end].starts_with("/**")
                        && !input[start..end].starts_with("/*!")
                    {
                        comments.try_push(Comment {
                            on_new_line,
                            kind: CommentKind::Block,
                            span: Span::new(start, end),
                        })?;
                    }
                }
                _ => {}
            },
            '"' => {
                on_new_line = false;
                if !in_char && !in_template {
                    in_string = !in_string;
                }
            }
            '\'' => {
                on_new_line = false;
                if !in_string && !in_template {
                    in_char = !in_char;
                }
            }
            '`' => {
                on_new_line = false;
                if !in_string && !in_char {
                    in_template = !in_template;
                }
            }
            '\n' => {
                on_new_line = true;
            }
            c if c.is_whitespace() => {}
            _ => {
                on_new_line = false;
            }
        }
    }

    Ok(comments)
}

fn parse_line_comment(chars: &mut CharIndices<'_>) -> usize {
    let mut last_i = 0;

    for (i, c) in chars.by_ref() {
        match c {
            '\n' => return i,
            _ => {
                last_i = i;
            }
        }
    }

    last_i + 1
}

fn parse_block_comment(chars: &mut CharIndices<'_>) -> Option<usize> {
    while let Some((_, c)) = chars.next() {
        if c == '*' {
            if let Some((_, '/')) = chars.clone().next() {
                return Some(chars.next()?.0);
            }
        }
    }

    None
}
