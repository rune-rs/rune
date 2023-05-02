use super::error::FormattingError;
use crate::ast::Span;

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
                empty_lines.push(EmptyLine {
                    span: Span::new(line_start, i + 1),
                });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_lines() {
        let source = r#"fn main() {

	let x = 1;
	let y = 2;
	let z = 3;
}"#;
        let empty_lines = gather_empty_line_spans(source).unwrap();
        assert_eq!(&source[12..13], "\n");
        assert_eq!(empty_lines.len(), 1);
        assert_eq!(empty_lines[0].span, Span::new(12, 13));
    }

    #[test]
    fn test_empty_lines_with_trailing_whitespace() {
        let source = r#"fn main() {

	let x = 1;
	let y = 2;
	let z = 3;

}"#;

        let empty_lines = gather_empty_line_spans(source).unwrap();
        assert_eq!(empty_lines.len(), 2);
        assert_eq!(empty_lines[0].span, Span::new(12, 13));
        assert_eq!(empty_lines[1].span, Span::new(49, 50));
    }

    #[test]
    fn test_empty_lines_with_multiple_empty_lines() {
        let source = r#"fn main() {

	let x = 1;

	let y = 2;
	let z = 3;
"#;

        let empty_lines = gather_empty_line_spans(source).unwrap();
        dbg!(&empty_lines);
        assert_eq!(empty_lines.len(), 2);
        assert_eq!(empty_lines[0].span, Span::new(12, 13));
        assert_eq!(empty_lines[1].span, Span::new(25, 26));
    }

    #[test]
    fn test_empty_lines_with_empty_file() {
        let source = r#""#;

        let empty_lines = gather_empty_line_spans(source).unwrap();
        assert_eq!(empty_lines.len(), 0);
    }

    #[test]
    fn test_empty_lines_with_empty_file_with_newline() {
        let source = r#"
"#;

        let empty_lines = gather_empty_line_spans(source).unwrap();
        assert_eq!(empty_lines[0].span, Span::new(0, 1));
    }

    #[test]
    fn test_empty_lines_with_empty_file_with_newline_and_whitespace() {
        let source = r#"
	"#;

        let empty_lines = gather_empty_line_spans(source).unwrap();
        assert_eq!(empty_lines.len(), 1);
        assert_eq!(empty_lines[0].span, Span::new(0, 1));
    }

    #[test]
    fn test_two_empty_lines() {
        let source = r#"fn main() {


}"#;

        let empty_lines = gather_empty_line_spans(source).unwrap();
        assert_eq!(empty_lines.len(), 2);
        assert_eq!(empty_lines[0].span, Span::new(12, 13));
        assert_eq!(empty_lines[1].span, Span::new(13, 14));
    }
}
