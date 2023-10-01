use core::char;
use core::fmt;
use core::iter::Peekable;
use core::ops;

#[derive(Debug)]
pub(crate) enum ErrorKind {
    BadEscapeSequence,
    BadUnicodeEscapeInByteString,
    BadUnicodeEscape,
    BadHexEscapeChar,
    BadHexEscapeByte,
    BadByteEscape,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::BadEscapeSequence => write!(f, "Bad escape sequence"),
            ErrorKind::BadUnicodeEscapeInByteString => {
                write!(
                    f,
                    "Unicode escapes are not supported as a byte or byte string"
                )
            }
            ErrorKind::BadUnicodeEscape => {
                write!(f, "Bad unicode escape")
            }
            ErrorKind::BadHexEscapeChar => {
                write!(f, "This form of character escape may only be used with characters in the range [\\x00-\\x7f]")
            }
            ErrorKind::BadHexEscapeByte => {
                write!(f,
                    "This form of byte escape may only be used with characters in the range [\\x00-\\xff]"
                )
            }
            ErrorKind::BadByteEscape => {
                write!(f, "Bad byte escape")
            }
        }
    }
}

cfg_std! {
    impl std::error::Error for ErrorKind {}
}

/// Indicates if we are parsing template escapes.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WithTemplate(pub(super) bool);

impl ops::Deref for WithTemplate {
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
) -> Result<Option<u8>, ErrorKind> {
    let (_, c) = it.next().ok_or(ErrorKind::BadEscapeSequence)?;

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
                return Err(ErrorKind::BadHexEscapeByte);
            }

            result as u8
        }
        'u' => {
            return Err(ErrorKind::BadUnicodeEscapeInByteString);
        }
        _ => {
            return Err(ErrorKind::BadEscapeSequence);
        }
    }))
}

/// Parse a byte escape sequence.
pub(super) fn parse_char_escape(
    it: &mut Peekable<impl Iterator<Item = (usize, char)>>,
    with_template: WithTemplate,
    with_line_cont: WithLineCont,
) -> Result<Option<char>, ErrorKind> {
    let (_, c) = it.next().ok_or(ErrorKind::BadEscapeSequence)?;

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
        '$' if *with_template => '$',
        '`' if *with_template => '`',
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
                return Err(ErrorKind::BadHexEscapeChar);
            }

            if let Some(c) = char::from_u32(result) {
                c
            } else {
                return Err(ErrorKind::BadByteEscape);
            }
        }
        'u' => parse_unicode_escape(it)?,
        _ => {
            return Err(ErrorKind::BadEscapeSequence);
        }
    }))
}

/// Parse a hex escape.
fn parse_hex_escape(
    it: &mut Peekable<impl Iterator<Item = (usize, char)>>,
) -> Result<u32, ErrorKind> {
    let mut result = 0u32;

    for _ in 0..2 {
        let (_, c) = it.next().ok_or(ErrorKind::BadByteEscape)?;

        result = result.checked_shl(4).ok_or(ErrorKind::BadByteEscape)?;

        result += match c {
            '0'..='9' => c as u32 - '0' as u32,
            'a'..='f' => c as u32 - 'a' as u32 + 10,
            'A'..='F' => c as u32 - 'A' as u32 + 10,
            _ => return Err(ErrorKind::BadByteEscape),
        };
    }

    Ok(result)
}

/// Parse a unicode escape.
pub(super) fn parse_unicode_escape(
    it: &mut Peekable<impl Iterator<Item = (usize, char)>>,
) -> Result<char, ErrorKind> {
    match it.next() {
        Some((_, '{')) => (),
        _ => return Err(ErrorKind::BadUnicodeEscape),
    };

    let mut first = true;
    let mut result = 0u32;

    loop {
        let (_, c) = it.next().ok_or(ErrorKind::BadUnicodeEscape)?;

        match c {
            '}' => {
                if first {
                    return Err(ErrorKind::BadUnicodeEscape);
                }

                if let Some(c) = char::from_u32(result) {
                    return Ok(c);
                }

                return Err(ErrorKind::BadUnicodeEscape);
            }
            c => {
                first = false;

                result = match result.checked_shl(4) {
                    Some(result) => result,
                    None => {
                        return Err(ErrorKind::BadUnicodeEscape);
                    }
                };

                result += match c {
                    '0'..='9' => c as u32 - '0' as u32,
                    'a'..='f' => c as u32 - 'a' as u32 + 10,
                    'A'..='F' => c as u32 - 'A' as u32 + 10,
                    _ => {
                        return Err(ErrorKind::BadUnicodeEscape);
                    }
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_hex_escape, parse_unicode_escape};
    use crate::tests::prelude::*;

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
