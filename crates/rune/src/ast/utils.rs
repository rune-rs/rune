use crate::ast;
use crate::ParseErrorKind;
use std::iter::Peekable;
use std::ops;

/// Indicates if we are parsing braces.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WithBrace(pub(super) bool);

impl ops::Deref for WithBrace {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Indicates if we are parsing line continuations or not.
#[derive(Debug, Clone, Copy)]
pub(super) struct WithLineCont(pub(super) bool);

impl ops::Deref for WithLineCont {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Parse a byte escape sequence.
pub(super) fn parse_byte_escape(
    it: &mut Peekable<impl Iterator<Item = (usize, char)>>,
    with_line_cont: WithLineCont,
) -> Result<Option<u8>, ParseErrorKind> {
    let (_, c) = it.next().ok_or_else(|| ParseErrorKind::BadEscapeSequence)?;

    Ok(Some(match c {
        '\n' | '\r' if *with_line_cont => {
            while let Some((_, c)) = it.peek() {
                if !char::is_whitespace(*c) {
                    break;
                }

                it.next();
            }

            return Ok(None);
        }
        '\'' => b'\'',
        '\"' => b'\"',
        'n' => b'\n',
        'r' => b'\r',
        't' => b'\t',
        '\\' => b'\\',
        '0' => b'\0',
        'x' => {
            let result = parse_hex_escape(it)?;

            if result > 0xff {
                return Err(ParseErrorKind::UnsupportedByteEscape);
            }

            result as u8
        }
        'u' => {
            return Err(ParseErrorKind::UnicodeEscapeNotSupported);
        }
        _ => {
            return Err(ParseErrorKind::BadEscapeSequence);
        }
    }))
}

/// Parse a byte escape sequence.
pub(super) fn parse_char_escape(
    it: &mut Peekable<impl Iterator<Item = (usize, char)>>,
    with_brace: WithBrace,
    with_line_cont: WithLineCont,
) -> Result<Option<char>, ParseErrorKind> {
    let (_, c) = it.next().ok_or_else(|| ParseErrorKind::BadEscapeSequence)?;

    Ok(Some(match c {
        '\n' | '\r' if *with_line_cont => {
            while let Some((_, c)) = it.peek() {
                if !char::is_whitespace(*c) {
                    break;
                }

                it.next();
            }

            return Ok(None);
        }
        '{' if *with_brace => '{',
        '}' if *with_brace => '}',
        '\'' => '\'',
        '\"' => '\"',
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        '\\' => '\\',
        '0' => '\0',
        'x' => {
            let result = parse_hex_escape(it)?;

            if result > 0x7f {
                return Err(ParseErrorKind::UnsupportedUnicodeByteEscape);
            }

            if let Some(c) = std::char::from_u32(result) {
                c
            } else {
                return Err(ParseErrorKind::BadByteEscape);
            }
        }
        'u' => parse_unicode_escape(it)?,
        _ => {
            return Err(ParseErrorKind::BadEscapeSequence);
        }
    }))
}

/// Parse a hex escape.
fn parse_hex_escape(
    it: &mut Peekable<impl Iterator<Item = (usize, char)>>,
) -> Result<u32, ParseErrorKind> {
    let mut result = 0u32;

    for _ in 0..2 {
        let (_, c) = it.next().ok_or_else(|| ParseErrorKind::BadByteEscape)?;

        result = result
            .checked_shl(4)
            .ok_or_else(|| ParseErrorKind::BadByteEscape)?;

        result += match c {
            '0'..='9' => c as u32 - '0' as u32,
            'a'..='f' => c as u32 - 'a' as u32 + 10,
            'A'..='F' => c as u32 - 'A' as u32 + 10,
            _ => return Err(ParseErrorKind::BadByteEscape),
        };
    }

    Ok(result)
}

/// Parse a unicode escape.
pub(super) fn parse_unicode_escape(
    it: &mut Peekable<impl Iterator<Item = (usize, char)>>,
) -> Result<char, ParseErrorKind> {
    match it.next() {
        Some((_, '{')) => (),
        _ => return Err(ParseErrorKind::BadUnicodeEscape),
    };

    let mut first = true;
    let mut result = 0u32;

    loop {
        let (_, c) = it.next().ok_or_else(|| ParseErrorKind::BadUnicodeEscape)?;

        match c {
            '}' => {
                if first {
                    return Err(ParseErrorKind::BadUnicodeEscape);
                }

                if let Some(c) = std::char::from_u32(result) {
                    return Ok(c);
                }

                return Err(ParseErrorKind::BadUnicodeEscape);
            }
            c => {
                first = false;

                result = match result.checked_shl(4) {
                    Some(result) => result,
                    None => {
                        return Err(ParseErrorKind::BadUnicodeEscape);
                    }
                };

                result += match c {
                    '0'..='9' => c as u32 - '0' as u32,
                    'a'..='f' => c as u32 - 'a' as u32 + 10,
                    'A'..='F' => c as u32 - 'A' as u32 + 10,
                    _ => {
                        return Err(ParseErrorKind::BadUnicodeEscape);
                    }
                };
            }
        }
    }
}

/// Test if the given expression qualifieis as a block end or not, as with a
/// body in a match expression.
///
/// This determines if a comma is necessary or not after the expression.
pub(crate) fn is_block_end(expr: &ast::Expr, comma: Option<&T![,]>) -> bool {
    match (expr, comma) {
        (ast::Expr::ExprBlock(..), _) => false,
        (ast::Expr::ExprFor(..), _) => false,
        (ast::Expr::ExprWhile(..), _) => false,
        (ast::Expr::ExprIf(..), _) => false,
        (ast::Expr::ExprMatch(..), _) => false,
        (_, Some(..)) => false,
        (_, None) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_hex_escape, parse_unicode_escape};

    macro_rules! input {
        ($string:expr) => {
            &mut String::from($string).char_indices().peekable()
        };
    }

    #[test]
    fn test_parse_hex_escape() {
        assert!(parse_hex_escape(input!("a")).is_err());

        let c = parse_hex_escape(input!("7f")).unwrap();
        assert_eq!(c, 0x7f);
    }

    #[test]
    fn test_parse_unicode_escape() {
        parse_unicode_escape(input!("{0}")).unwrap();

        let c = parse_unicode_escape(input!("{1F4AF}")).unwrap();
        assert_eq!(c, '💯');

        let c = parse_unicode_escape(input!("{1f4af}")).unwrap();
        assert_eq!(c, '💯');
    }
}
