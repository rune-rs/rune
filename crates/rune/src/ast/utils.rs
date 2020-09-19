use crate::{ast, IntoTokens, MacroContext, Parse, Parser, Peek, TokenStream};
use crate::{ParseError, ParseErrorKind};
use runestick::Span;
use std::iter::Peekable;
use std::ops;

/// Indicates if we are parsing braces.
#[derive(Debug, Clone, Copy)]
pub(super) struct WithBrace(pub(super) bool);

impl ops::Deref for WithBrace {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Parse a byte escape sequence.
pub(super) fn parse_byte_escape<I>(span: Span, it: &mut Peekable<I>) -> Result<u8, ParseError>
where
    I: Iterator<Item = (usize, char)>,
{
    let (n, c) = match it.next() {
        Some(c) => c,
        None => {
            return Err(ParseError::new(span, ParseErrorKind::BadEscapeSequence));
        }
    };

    Ok(match c {
        '\'' => b'\'',
        '\"' => b'\"',
        'n' => b'\n',
        'r' => b'\r',
        't' => b'\t',
        '\\' => b'\\',
        '0' => b'\0',
        'x' => {
            let (result, span) = parse_hex_escape(span, it)?;

            if result > 0xff {
                return Err(ParseError::new(span, ParseErrorKind::UnsupportedByteEscape));
            }

            result as u8
        }
        'u' => {
            return Err(ParseError::new(
                span,
                ParseErrorKind::UnicodeEscapeNotSupported,
            ));
        }
        _ => {
            let span = span.with_end(n);
            return Err(ParseError::new(span, ParseErrorKind::BadEscapeSequence));
        }
    })
}

/// Parse a byte escape sequence.
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
            return Err(ParseError::new(span, ParseErrorKind::BadEscapeSequence));
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
        'x' => {
            let (result, span) = parse_hex_escape(span, it)?;

            if result > 0x7f {
                return Err(ParseError::new(
                    span,
                    ParseErrorKind::UnsupportedUnicodeByteEscape,
                ));
            }

            if let Some(c) = std::char::from_u32(result) {
                c
            } else {
                return Err(ParseError::new(span, ParseErrorKind::BadByteEscape));
            }
        }
        'u' => parse_unicode_escape(span, it)?,
        _ => {
            let span = span.with_end(n);
            return Err(ParseError::new(span, ParseErrorKind::BadEscapeSequence));
        }
    })
}

/// Parse a hex escape.
fn parse_hex_escape<I>(span: Span, it: &mut Peekable<I>) -> Result<(u32, Span), ParseError>
where
    I: Iterator<Item = (usize, char)>,
{
    let mut result = 0u32;

    for _ in 0..2 {
        let (_, c) = it
            .next()
            .ok_or_else(|| ParseError::new(span, ParseErrorKind::BadByteEscape))?;

        let span = it.peek().map(|(n, _)| span.with_end(*n)).unwrap_or(span);

        result = result
            .checked_shl(4)
            .ok_or_else(|| ParseError::new(span, ParseErrorKind::BadByteEscape))?;

        result += match c {
            '0'..='9' => c as u32 - '0' as u32,
            'a'..='f' => c as u32 - 'a' as u32 + 10,
            'A'..='F' => c as u32 - 'A' as u32 + 10,
            _ => return Err(ParseError::new(span, ParseErrorKind::BadByteEscape)),
        };
    }

    let span = it.peek().map(|(n, _)| span.with_end(*n)).unwrap_or(span);
    Ok((result, span))
}

/// Parse a unicode escape.
pub(super) fn parse_unicode_escape<I>(span: Span, it: &mut Peekable<I>) -> Result<char, ParseError>
where
    I: Iterator<Item = (usize, char)>,
{
    match it.next() {
        Some((_, '{')) => (),
        _ => return Err(ParseError::new(span, ParseErrorKind::BadUnicodeEscape)),
    };

    let mut first = true;
    let mut result = 0u32;

    loop {
        let (_, c) = it
            .next()
            .ok_or_else(|| ParseError::new(span, ParseErrorKind::BadUnicodeEscape))?;

        let span = it.peek().map(|(n, _)| span.with_end(*n)).unwrap_or(span);

        match c {
            '}' => {
                if first {
                    return Err(ParseError::new(span, ParseErrorKind::BadUnicodeEscape));
                }

                if let Some(c) = std::char::from_u32(result) {
                    return Ok(c);
                }

                return Err(ParseError::new(span, ParseErrorKind::BadUnicodeEscape));
            }
            c => {
                first = false;

                result = result
                    .checked_shl(4)
                    .ok_or_else(|| ParseError::new(span, ParseErrorKind::BadUnicodeEscape))?;

                result += match c {
                    '0'..='9' => c as u32 - '0' as u32,
                    'a'..='f' => c as u32 - 'a' as u32 + 10,
                    'A'..='F' => c as u32 - 'A' as u32 + 10,
                    _ => {
                        return Err(ParseError::new(span, ParseErrorKind::BadUnicodeEscape));
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
            .ok_or_else(|| ParseError::new(span, ParseErrorKind::InvalidTemplateLiteral))?;

        if start.is_none() {
            start = Some(n);
        }

        match c {
            '{' => level += 1,
            '}' => level -= 1,
            _ => (),
        }

        if level == 0 {
            let start = start
                .ok_or_else(|| ParseError::new(span, ParseErrorKind::InvalidTemplateLiteral))?;
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

/// Any open delimiter `{`, `(`, or `{`
#[derive(Debug, Clone, Copy)]
pub enum OpenDelim {
    Paren(ast::OpenParen),
    Bracket(ast::OpenBracket),
    Brace(ast::OpenBrace),
}

impl OpenDelim {
    pub fn delim_kind(&self) -> ast::Delimiter {
        use OpenDelim::*;

        match self {
            Paren(_) => ast::Delimiter::Parenthesis,
            Bracket(_) => ast::Delimiter::Bracket,
            Brace(_) => ast::Delimiter::Brace,
        }
    }

    pub fn token(&self) -> ast::Token {
        use OpenDelim::*;

        match self {
            Paren(d) => d.token,
            Bracket(d) => d.token,
            Brace(d) => d.token,
        }
    }
}

impl Parse for OpenDelim {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        if parser.peek::<ast::OpenParen>()? {
            Ok(OpenDelim::Paren(parser.parse()?))
        } else if parser.peek::<ast::OpenBrace>()? {
            Ok(OpenDelim::Brace(parser.parse()?))
        } else if parser.peek::<ast::OpenBracket>()? {
            Ok(OpenDelim::Bracket(parser.parse()?))
        } else {
            let token = parser.token_peek_eof()?;
            Err(ParseError::new(
                token,
                ParseErrorKind::ExpectedOpenDelimiter { actual: token.kind },
            ))
        }
    }
}

impl Peek for OpenDelim {
    fn peek(t1: Option<ast::Token>, _t2: Option<ast::Token>) -> bool {
        let t1 = match t1 {
            Some(t1) => t1,
            None => return false,
        };

        match t1.kind {
            ast::Kind::Open(_delimiter) => true,
            _ => false,
        }
    }
}

impl IntoTokens for OpenDelim {
    fn into_tokens(&self, context: &mut MacroContext, stream: &mut TokenStream) {
        self.token().into_tokens(context, stream)
    }
}

/// Any close delimiter `}`, `)`, or `}`
#[derive(Debug, Clone, Copy)]
pub enum CloseDelim {
    Paren(ast::CloseParen),
    Bracket(ast::CloseBracket),
    Brace(ast::CloseBrace),
}

impl CloseDelim {
    pub fn delim_kind(&self) -> ast::Delimiter {
        use CloseDelim::*;

        match self {
            Paren(_) => ast::Delimiter::Parenthesis,
            Bracket(_) => ast::Delimiter::Bracket,
            Brace(_) => ast::Delimiter::Brace,
        }
    }

    pub fn token(&self) -> ast::Token {
        use CloseDelim::*;

        match self {
            Paren(d) => d.token,
            Bracket(d) => d.token,
            Brace(d) => d.token,
        }
    }
}

impl Parse for CloseDelim {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        if parser.peek::<ast::CloseParen>()? {
            Ok(CloseDelim::Paren(parser.parse()?))
        } else if parser.peek::<ast::CloseBrace>()? {
            Ok(CloseDelim::Brace(parser.parse()?))
        } else if parser.peek::<ast::CloseBracket>()? {
            Ok(CloseDelim::Bracket(parser.parse()?))
        } else {
            let token = parser.token_peek_eof()?;
            Err(ParseError::new(
                token,
                ParseErrorKind::ExpectedCloseDelimiter { actual: token.kind },
            ))
        }
    }
}

impl Peek for CloseDelim {
    fn peek(t1: Option<ast::Token>, _t2: Option<ast::Token>) -> bool {
        let t1 = match t1 {
            Some(t1) => t1,
            None => return false,
        };

        match t1.kind {
            ast::Kind::Close(_delimiter) => true,
            _ => false,
        }
    }
}

impl IntoTokens for CloseDelim {
    fn into_tokens(&self, context: &mut MacroContext, stream: &mut TokenStream) {
        self.token().into_tokens(context, stream)
    }
}

/// Any Any delimiter `}`, `)`, or `}`
#[derive(Debug, Clone, Copy)]
pub enum AnyDelim {
    Open(OpenDelim),
    Close(CloseDelim),
}

impl AnyDelim {
    #[allow(dead_code)]
    pub fn delim_kind(&self) -> ast::Delimiter {
        use AnyDelim::*;

        match self {
            Open(delim) => delim.delim_kind(),
            Close(delim) => delim.delim_kind(),
        }
    }

    pub fn token(&self) -> ast::Token {
        use AnyDelim::*;

        match self {
            Open(delim) => delim.token(),
            Close(delim) => delim.token(),
        }
    }
}

impl Parse for AnyDelim {
    fn parse(parser: &mut Parser<'_>) -> Result<Self, ParseError> {
        if parser.peek::<OpenDelim>()? {
            Ok(AnyDelim::Open(parser.parse()?))
        } else if parser.peek::<CloseDelim>()? {
            Ok(AnyDelim::Close(parser.parse()?))
        } else {
            let token = parser.token_peek_eof()?;
            Err(ParseError::new(
                token,
                ParseErrorKind::ExpectedDelimiter { actual: token.kind },
            ))
        }
    }
}

impl Peek for AnyDelim {
    fn peek(t1: Option<ast::Token>, t2: Option<ast::Token>) -> bool {
        OpenDelim::peek(t1, t2) || CloseDelim::peek(t1, t2)
    }
}

impl IntoTokens for AnyDelim {
    fn into_tokens(&self, context: &mut MacroContext, stream: &mut TokenStream) {
        self.token().into_tokens(context, stream)
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_hex_escape, parse_unicode_escape};
    use runestick::Span;

    macro_rules! input {
        ($string:expr) => {
            &mut String::from($string).char_indices().peekable()
        };
    }

    #[test]
    fn test_parse_hex_escape() {
        assert!(parse_hex_escape(Span::empty(), input!("a")).is_err());

        let (c, _) = parse_hex_escape(Span::empty(), input!("7f")).unwrap();
        assert_eq!(c, 0x7f);
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
