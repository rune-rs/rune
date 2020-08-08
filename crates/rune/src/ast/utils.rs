use crate::ast;
use crate::error::ParseError;
use runestick::unit::Span;
use std::iter::Peekable;
use std::ops;

#[derive(Debug, Clone, Copy)]
pub(super) struct WithBrace(pub(super) bool);

impl ops::Deref for WithBrace {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Parse an escape sequence.
pub(super) fn parse_char_escape<I>(
    span: Span,
    it: &mut Peekable<I>,
    with_brace: WithBrace,
) -> Result<char, ParseError>
where
    I: Iterator<Item = (usize, char)>,
{
    let (n, c) = match it.next() {
        Some(c) => c,
        None => {
            return Err(ParseError::BadEscapeSequence { span });
        }
    };

    Ok(match c {
        '{' if *with_brace => '{',
        '}' if *with_brace => '}',
        '\'' => '\'',
        '\"' => '\"',
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        '\\' => '\\',
        '0' => '\0',
        'x' => parse_hex_escape(span, it)?,
        'u' => parse_unicode_escape(span, it)?,
        _ => {
            let span = span.with_end(n);
            return Err(ParseError::BadEscapeSequence { span });
        }
    })
}

/// Parse a hex escape.
fn parse_hex_escape<I>(span: Span, it: &mut Peekable<I>) -> Result<char, ParseError>
where
    I: Iterator<Item = (usize, char)>,
{
    let mut result = 0u32;

    for _ in 0..2 {
        let (_, c) = it
            .next()
            .ok_or_else(|| ParseError::BadByteEscape { span })?;

        let span = it.peek().map(|(n, _)| span.with_end(*n)).unwrap_or(span);

        result = result
            .checked_shl(4)
            .ok_or_else(|| ParseError::BadByteEscape { span })?;

        result += match c {
            '0'..='9' => c as u32 - '0' as u32,
            'a'..='f' => c as u32 - 'a' as u32 + 10,
            'A'..='F' => c as u32 - 'A' as u32 + 10,
            _ => return Err(ParseError::BadByteEscape { span }),
        };
    }

    let span = it.peek().map(|(n, _)| span.with_end(*n)).unwrap_or(span);

    if result > 0x7f {
        return Err(ParseError::BadByteEscapeBounds { span });
    }

    if let Some(c) = std::char::from_u32(result) {
        Ok(c)
    } else {
        Err(ParseError::BadByteEscape { span })
    }
}

/// Parse a unicode escape.
pub(super) fn parse_unicode_escape<I>(span: Span, it: &mut Peekable<I>) -> Result<char, ParseError>
where
    I: Iterator<Item = (usize, char)>,
{
    match it.next() {
        Some((_, '{')) => (),
        _ => return Err(ParseError::BadUnicodeEscape { span }),
    };

    let mut first = true;
    let mut result = 0u32;

    loop {
        let (_, c) = it
            .next()
            .ok_or_else(|| ParseError::BadUnicodeEscape { span })?;

        let span = it.peek().map(|(n, _)| span.with_end(*n)).unwrap_or(span);

        match c {
            '}' => {
                if first {
                    return Err(ParseError::BadUnicodeEscape { span });
                }

                if let Some(c) = std::char::from_u32(result) {
                    return Ok(c);
                }

                return Err(ParseError::BadUnicodeEscape { span });
            }
            c => {
                first = false;

                result = result
                    .checked_shl(4)
                    .ok_or_else(|| ParseError::BadUnicodeEscape { span })?;

                result += match c {
                    '0'..='9' => c as u32 - '0' as u32,
                    'a'..='f' => c as u32 - 'a' as u32 + 10,
                    'A'..='F' => c as u32 - 'A' as u32 + 10,
                    _ => {
                        return Err(ParseError::BadUnicodeEscape { span });
                    }
                };
            }
        }
    }
}

/// Find the span of an expression inside of a balanced collection of braces.
///
/// This is expected to start parsing immediately after an opening brace `{`.
pub(crate) fn template_expr<I>(span: Span, it: &mut I) -> Result<Span, ParseError>
where
    I: Iterator<Item = (usize, char)>,
{
    let mut start = None;
    let mut level = 1;

    loop {
        let (n, c) = it
            .next()
            .ok_or_else(|| ParseError::InvalidTemplateLiteral { span })?;

        if start.is_none() {
            start = Some(n);
        }

        match c {
            '{' => level += 1,
            '}' => level -= 1,
            _ => (),
        }

        if level == 0 {
            let start = start.ok_or_else(|| ParseError::InvalidTemplateLiteral { span })?;
            return Ok(Span::new(start, n));
        }
    }
}

/// Test if the given expression qualifieis as a block end or not, as with a
/// body in a match expression.
///
/// This determines if a comma is necessary or not after the expression.
pub(crate) fn is_block_end(expr: &ast::Expr, comma: Option<&ast::Comma>) -> bool {
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
    use runestick::unit::Span;

    macro_rules! input {
        ($string:expr) => {
            &mut String::from($string).char_indices().peekable()
        };
    }

    #[test]
    fn test_parse_hex_escape() {
        assert!(parse_hex_escape(Span::empty(), input!("a")).is_err());

        let c = parse_hex_escape(Span::empty(), input!("7f")).unwrap();
        assert_eq!(c, '\x7f');
    }

    #[test]
    fn test_parse_unicode_escape() {
        parse_unicode_escape(Span::empty(), input!("{0}")).unwrap();

        let c = parse_unicode_escape(Span::empty(), input!("{1F4AF}")).unwrap();
        assert_eq!(c, '💯');

        let c = parse_unicode_escape(Span::empty(), input!("{1f4af}")).unwrap();
        assert_eq!(c, '💯');
    }
}
