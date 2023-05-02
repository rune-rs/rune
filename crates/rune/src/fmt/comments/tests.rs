#[cfg(test)]
use super::*;

#[test]
fn test_parse_line_comment() {
    let input = "// this is a comment\n";
    let mut chars = input.char_indices();
    let end = parse_line_comment(&mut chars);
    assert_eq!(end, input.len() - 1);
}

#[test]
fn test_parse_block_comment() {
    let input = "/* this is a comment */";
    let mut chars = input.char_indices();
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
