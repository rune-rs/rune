//! Extract comments from source code.

use std::iter::Peekable;

use crate::ast::Span;

use super::error::FormattingError;

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

    let mut chars = input.char_indices().peekable();

    let mut in_string = false;
    let mut in_char = false;
    let mut in_template = false;
    let mut on_new_line = true;

    while let Some((idx, c)) = chars.next() {
        match c {
            '/' if !in_string && !in_char && !in_template => match chars.peek() {
                Some((_, '/')) => {
                    let end = parse_line_comment(&mut chars);

                    if !input[idx..end].starts_with("///") && !input[idx..end].starts_with("//!") {
                        comments.push(Comment {
                            on_new_line,
                            kind: CommentKind::Line,
                            span: Span::new(idx, end),
                        });
                    }
                }
                Some((_, '*')) => {
                    let end = parse_block_comment(&mut chars).ok_or(FormattingError::Eof)?;

                    if !input[idx..end].starts_with("/**") && !input[idx..end].starts_with("/*!") {
                        comments.push(Comment {
                            on_new_line,
                            kind: CommentKind::Block,
                            span: Span::new(idx, end),
                        });
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

fn parse_line_comment(chars: &mut Peekable<impl Iterator<Item = (usize, char)>>) -> usize {
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

fn parse_block_comment(chars: &mut Peekable<impl Iterator<Item = (usize, char)>>) -> Option<usize> {
    while let Some((_, c)) = chars.next() {
        if c == '*' {
            if let Some((_, '/')) = chars.peek() {
                let (offset, _) = chars.next()?;
                return Some(offset);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_line_comment() {
        let input = "// this is a comment\n";
        let mut chars = input.char_indices().peekable();
        let end = parse_line_comment(&mut chars);
        assert_eq!(end, input.len() - 1);
    }

    #[test]
    fn test_parse_block_comment() {
        let input = "/* this is a comment */";
        let mut chars = input.char_indices().peekable();
        let end = parse_block_comment(&mut chars).unwrap();
        assert_eq!(end, input.len() - 1);
    }

    #[test]
    fn test_parse_comments() {
        let input = "// this is a comment\n/* this is a comment */";
        let comments = parse_comments(input).unwrap();
        assert_eq!(comments.len(), 2);
    }

    #[test]
    fn test_parse_comments2() {
        let input = "/* this is a comment */\n// this is a comment";
        let comments = parse_comments(input).unwrap();
        assert_eq!(comments.len(), 2);
    }
}
