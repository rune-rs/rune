// Author: Tom Solberg <me@sbg.dev>
// Copyright © 2023, Tom Solberg, all rights reserved.
// Created: 30 April 2023

/*!

*/

use crate::ast::Span;

use super::error::FormattingError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommentKind {
    Line,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct Comment {
    pub kind: CommentKind,
    pub span: Span,
}

pub(super) fn parse_comments(input: &str) -> Result<Vec<Comment>, FormattingError> {
    let mut comments = Vec::new();

    let mut chars = input.char_indices().peekable();

    let mut in_string = false;
    let mut in_char = false;
    let mut in_template = false;

    while let Some((idx, c)) = chars.next() {
        match c {
            '/' if !in_string && !in_char && !in_template => match chars.peek() {
                Some((_, '/')) => {
                    let end = parse_line_comment(&mut chars)?;

                    if !input[idx..end].starts_with("///") && !input[idx..end].starts_with("//!") {
                        comments.push(Comment {
                            kind: CommentKind::Line,
                            span: Span::new(idx, end),
                        });
                    }
                }
                Some((_, '*')) => {
                    let end = parse_block_comment(&mut chars)?;

                    if !input[idx..end].starts_with("/**") && !input[idx..end].starts_with("/*!") {
                        comments.push(Comment {
                            kind: CommentKind::Block,
                            span: Span::new(idx, end),
                        });
                    }
                }
                _ => {}
            },
            '"' => {
                if !in_char && !in_template {
                    in_string = !in_string;
                }
            }
            '\'' => {
                if !in_string && !in_template {
                    in_char = !in_char;
                }
            }
            '`' => {
                if !in_string && !in_char {
                    in_template = !in_template;
                }
            }

            _ => {}
        }
    }

    Ok(comments)
}

fn parse_line_comment(
    chars: &mut std::iter::Peekable<impl Iterator<Item = (usize, char)>>,
) -> Result<usize, FormattingError> {
    let mut last_i = 0;
    while let Some((i, c)) = chars.next() {
        match c {
            '\n' => return Ok(i),
            _ => {
                last_i = i;
            }
        }
    }

    Ok(last_i + 1)
}

fn parse_block_comment(
    chars: &mut std::iter::Peekable<impl Iterator<Item = (usize, char)>>,
) -> Result<usize, FormattingError> {
    while let Some((_, c)) = chars.next() {
        match c {
            '*' => match chars.peek() {
                Some((_, '/')) => {
                    let (offset, _) = chars.next().unwrap();
                    return Ok(offset);
                }
                _ => {}
            },
            _ => {}
        }
    }

    Err(FormattingError::Eof)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_line_comment() {
        let input = "// this is a comment\n";
        let mut chars = input.char_indices().peekable();
        let end = parse_line_comment(&mut chars).unwrap();
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
